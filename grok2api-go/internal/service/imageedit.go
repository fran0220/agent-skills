package service

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/google/uuid"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/model"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/token"
)

const (
	editUpstreamModel = "grok-4"
	editUpstreamMode  = "MODEL_MODE_AUTO"
)

type ImageEditResult struct {
	Stream     bool
	StreamData <-chan string
	Data       []string
	Usage      map[string]any
}

type ImageEditService struct {
	cfg *config.Config
}

func NewImageEditService(cfg *config.Config) *ImageEditService {
	return &ImageEditService{cfg: cfg}
}

func (s *ImageEditService) Edit(
	ctx context.Context,
	tokenMgr *token.TokenManager,
	seedToken string,
	modelInfo model.ModelInfo,
	prompt string,
	images []string,
	n int,
	responseFormat string,
	stream bool,
	chatFormat bool,
) (*ImageEditResult, error) {
	if len(images) > 3 {
		images = images[len(images)-3:]
	}
	if n <= 0 {
		n = 1
	}
	maxRetry := 3
	if s.cfg != nil {
		maxRetry = s.cfg.GetInt("retry.max_retry", 3)
	}
	if maxRetry <= 0 {
		maxRetry = 1
	}

	tried := make(map[string]bool)
	var lastErr error
	for attempt := 0; attempt < maxRetry; attempt++ {
		preferred := ""
		if attempt == 0 {
			preferred = seedToken
		}
		currentToken := s.pickToken(tokenMgr, modelInfo, tried, preferred)
		if currentToken == "" {
			if lastErr != nil {
				return nil, normalizeChatError(lastErr)
			}
			return nil, NewStatusError(http.StatusTooManyRequests, "No available tokens. Please try again later.", "rate_limit_error", "rate_limit_exceeded", nil)
		}
		tried[token.NormalizeToken(currentToken)] = true

		fileAttachments, err := s.uploadImages(images, currentToken)
		if err != nil {
			return nil, err
		}

		toolOverrides := map[string]any{
			"gmailSearch":           false,
			"googleCalendarSearch":  false,
			"outlookSearch":         false,
			"outlookCalendarSearch": false,
			"googleDriveSearch":     false,
		}

		if stream {
			streamData, err := s.streamEdit(ctx, tokenMgr, currentToken, modelInfo.ModelID, prompt, fileAttachments, n, responseFormat, chatFormat, toolOverrides)
			if err == nil {
				return &ImageEditResult{Stream: true, StreamData: streamData, Usage: zeroUsage()}, nil
			}
			lastErr = err
			if isRateLimited(err) {
				_ = tokenMgr.MarkRateLimited(currentToken)
				continue
			}
			return nil, normalizeChatError(err)
		}

		items, err := s.collectEdit(ctx, currentToken, prompt, fileAttachments, n, responseFormat, chatFormat, toolOverrides)
		if err == nil {
			if consumeErr := tokenMgr.Consume(currentToken, imageEffortForModel(modelInfo)); consumeErr != nil {
			}
			return &ImageEditResult{Stream: false, Data: items, Usage: zeroUsage()}, nil
		}
		lastErr = err
		if isRateLimited(err) {
			_ = tokenMgr.MarkRateLimited(currentToken)
			continue
		}
		return nil, normalizeChatError(err)
	}

	if lastErr != nil {
		return nil, normalizeChatError(lastErr)
	}
	return nil, NewStatusError(http.StatusTooManyRequests, "No available tokens. Please try again later.", "rate_limit_error", "rate_limit_exceeded", nil)
}

func (s *ImageEditService) pickToken(tokenMgr *token.TokenManager, modelInfo model.ModelInfo, tried map[string]bool, preferred string) string {
	poolName := token.BasicPoolName
	if modelInfo.Tier == model.TierSuper {
		poolName = token.SuperPoolName
	}
	if normalized := token.NormalizeToken(preferred); normalized != "" {
		if !tried[normalized] && tokenMgr.GetPoolNameForToken(normalized) == poolName {
			return normalized
		}
	}
	return tokenMgr.GetToken(poolName, tried, nil)
}

func (s *ImageEditService) uploadImages(images []string, tokenValue string) ([]string, error) {
	uploadService := NewUploadService(s.cfg)
	defer uploadService.Close()
	fileAttachments := make([]string, 0, len(images))
	for _, image := range images {
		fileID, _, err := uploadService.UploadFile(image, tokenValue)
		if err != nil {
			return nil, normalizeChatError(err)
		}
		if fileID != "" {
			fileAttachments = append(fileAttachments, fileID)
		}
	}
	if len(fileAttachments) == 0 {
		return nil, NewStatusError(http.StatusBadGateway, "Image upload failed", "upload_error", "upload_failed", nil)
	}
	return fileAttachments, nil
}

func (s *ImageEditService) streamEdit(
	ctx context.Context,
	tokenMgr *token.TokenManager,
	tokenValue, modelID, prompt string,
	fileAttachments []string,
	n int,
	responseFormat string,
	chatFormat bool,
	toolOverrides map[string]any,
) (<-chan string, error) {
	body, session, err := s.requestEdit(ctx, tokenMgr, tokenValue, prompt, fileAttachments, n, toolOverrides)
	if err != nil {
		return nil, err
	}
	parsed, parseErr := s.parseEditStream(body, session, tokenValue, responseFormat)
	if parseErr != nil {
		return nil, parseErr
	}
	if len(parsed.Images) == 0 {
		return nil, &reverse.UpstreamError{Message: "Image edit returned no results", StatusCode: http.StatusBadGateway}
	}
	chunks := make([]string, 0, len(parsed.Progresses)+len(parsed.Images)+1)
	responseID := "chatcmpl-" + strings.ReplaceAll(uuid.NewString(), "-", "")
	imageIDs := make(map[int]string)
	for _, progress := range parsed.Progresses {
		if n == 1 && progress.ImageIndex != 0 {
			continue
		}
		imageID := imageIDs[progress.ImageIndex]
		if imageID == "" {
			imageID = fmt.Sprintf("app-chat-%d-%d", time.Now().UnixMilli(), progress.ImageIndex)
			imageIDs[progress.ImageIndex] = imageID
		}
		if chatFormat {
			continue
		}
		chunks = append(chunks, sseEvent("image_generation.partial_image", map[string]any{
			"type":                            "image_generation.partial_image",
			responseFieldName(responseFormat): "",
			"index":                           progress.ImageIndex,
			"progress":                        progress.Progress,
			"image_id":                        imageID,
		}))
	}
	for index, item := range parsed.Images {
		outIndex := index
		if n == 1 {
			outIndex = 0
		}
		if chatFormat {
			payload, _ := json.Marshal(map[string]any{
				"id":      responseID,
				"object":  "chat.completion.chunk",
				"created": time.Now().Unix(),
				"model":   modelID,
				"choices": []map[string]any{{
					"index": outIndex,
					"delta": map[string]any{
						"role":    "assistant",
						"content": wrapImageContent(item, responseFormat),
					},
					"finish_reason": "stop",
				}},
				"usage": zeroUsage(),
			})
			chunks = append(chunks, fmt.Sprintf("event: chat.completion.chunk\ndata: %s\n\n", payload))
			continue
		}
		imageID := imageIDs[index]
		if imageID == "" {
			imageID = fmt.Sprintf("app-chat-%d-%d", time.Now().UnixMilli(), index)
		}
		chunks = append(chunks, sseEvent("image_generation.completed", map[string]any{
			"type":                            "image_generation.completed",
			responseFieldName(responseFormat): item,
			"index":                           outIndex,
			"image_id":                        imageID,
			"stage":                           "final",
			"usage":                           zeroUsage(),
		}))
	}
	if chatFormat {
		chunks = append(chunks, "data: [DONE]\n\n")
	}
	return replayStream(ctx, chunks), nil
}

func (s *ImageEditService) collectEdit(
	ctx context.Context,
	tokenValue, prompt string,
	fileAttachments []string,
	n int,
	responseFormat string,
	chatFormat bool,
	toolOverrides map[string]any,
) ([]string, error) {
	perCall := 2
	callsNeeded := (n + perCall - 1) / perCall
	if callsNeeded < 1 {
		callsNeeded = 1
	}
	allImages := make([]string, 0, n)
	var lastErr error
	for i := 0; i < callsNeeded; i++ {
		body, session, err := s.requestEdit(ctx, nil, tokenValue, prompt, fileAttachments, min(n-len(allImages), perCall), toolOverrides)
		if err != nil {
			lastErr = err
			continue
		}
		parsed, parseErr := s.parseEditStream(body, session, tokenValue, responseFormat)
		if parseErr != nil {
			lastErr = parseErr
			continue
		}
		allImages = append(allImages, parsed.Images...)
		if len(allImages) >= n {
			break
		}
	}
	if len(allImages) == 0 {
		if lastErr != nil {
			return nil, lastErr
		}
		return nil, &reverse.UpstreamError{Message: "Image edit returned no results", StatusCode: http.StatusBadGateway}
	}
	if len(allImages) > n {
		allImages = allImages[:n]
	}
	if !chatFormat {
		return allImages, nil
	}
	wrapped := make([]string, 0, len(allImages))
	for _, item := range allImages {
		wrapped = append(wrapped, wrapImageContent(item, responseFormat))
	}
	return wrapped, nil
}

func (s *ImageEditService) requestEdit(
	ctx context.Context,
	tokenMgr *token.TokenManager,
	tokenValue, prompt string,
	fileAttachments []string,
	n int,
	toolOverrides map[string]any,
) (io.ReadCloser, *reverse.ResettableSession, error) {
	session := reverse.NewResettableSession(reverse.SessionOptions{Browser: s.cfg.GetString("proxy.browser", "")})
	appChat := reverse.NewAppChatReverse()
	if tokenMgr != nil {
		appChat.RecordTokenFailure = func(ctx context.Context, tokenValue string, status int, reason string) error {
			return tokenMgr.RecordFail(tokenValue, status, reason)
		}
	}
	requestOverrides := map[string]any{
		"imageGenerationCount": max(1, n),
		"modeId":               "auto",
		"disableMemory":        false,
		"temporary":            false,
	}
	body, err := appChat.Request(ctx, session, tokenValue, prompt, editUpstreamModel, editUpstreamMode, fileAttachments, toolOverrides, nil, requestOverrides)
	if err != nil {
		session.Close()
		return nil, nil, err
	}
	return body, session, nil
}

type imageEditParsed struct {
	Progresses []appProgress
	Images     []string
}

func (s *ImageEditService) parseEditStream(body io.ReadCloser, session *reverse.ResettableSession, tokenValue, responseFormat string) (*imageEditParsed, error) {
	defer closeResources(body, session)
	result := &imageEditParsed{}
	seen := make(map[string]bool)
	reader := bufio.NewReader(body)
	downloader := reverse.NewDownloadService()
	defer downloader.Close()
	for {
		line, err := reader.ReadBytes('\n')
		if len(line) > 0 {
			normalized := normalizeLine(line)
			if normalized != "" {
				var payload map[string]any
				if json.Unmarshal([]byte(normalized), &payload) == nil {
					response := nestedMap(payload, "result", "response")
					if response != nil {
						if progress := mapValue(response["streamingImageGenerationResponse"]); progress != nil {
							result.Progresses = append(result.Progresses, appProgress{ImageIndex: intValue(progress["imageIndex"]), Progress: intValue(progress["progress"])})
						}
						if card := mapValue(response["cardAttachment"]); card != nil {
							var cardData map[string]any
							if raw := strings.TrimSpace(stringValue(card["jsonData"])); raw != "" && json.Unmarshal([]byte(raw), &cardData) == nil {
								kind := stringValue(cardData["type"])
								if kind == "render_generated_image" || kind == "render_edited_image" {
									if chunk := mapValue(cardData["image_chunk"]); chunk != nil && intValue(chunk["progress"]) >= 100 {
										if url := normalizeAppChatURL(stringValue(chunk["imageUrl"])); url != "" && !seen[url] {
											if output, outputErr := appChatOutput(downloader, url, tokenValue, responseFormat); outputErr == nil && output != "" {
												seen[url] = true
												result.Images = append(result.Images, output)
											}
										}
									}
								}
							}
						}
						for _, url := range collectImageURLs(response["modelResponse"]) {
							if url == "" || seen[url] {
								continue
							}
							if output, outputErr := appChatOutput(downloader, url, tokenValue, responseFormat); outputErr == nil && output != "" {
								seen[url] = true
								result.Images = append(result.Images, output)
							}
						}
					}
				}
			}
		}
		if err != nil {
			if errors.Is(err, io.EOF) {
				break
			}
			return nil, err
		}
	}
	return result, nil
}

func wrapImageContent(content, responseFormat string) string {
	if strings.TrimSpace(content) == "" {
		return content
	}
	if responseFormat == "url" {
		return fmt.Sprintf("![image](%s)", content)
	}
	return fmt.Sprintf("![image](data:image/png;base64,%s)", content)
}
