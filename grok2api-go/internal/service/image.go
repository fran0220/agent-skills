package service

import (
	"bufio"
	"context"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"time"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/model"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/storage"
	"github.com/fran0220/grok2api-go/internal/token"
)

type ImageResult struct {
	Stream     bool
	StreamData <-chan string
	Data       []string
	Usage      map[string]any
}

type ImageGenerationService struct {
	cfg     *config.Config
	imagine *reverse.ImagineWebSocketReverse
}

type appProgress struct {
	ImageIndex int
	Progress   int
}

type appChatParseResult struct {
	Progresses []appProgress
	ImageURLs  []string
}

var appChatGeneratePrefixRE = regexp.MustCompile(`(?i)^\s*(generate an image|create an image|draw an image|make an image)\s*:`)

func NewImageGenerationService(cfg *config.Config) *ImageGenerationService {
	return &ImageGenerationService{cfg: cfg, imagine: reverse.NewImagineWebSocketReverse()}
}

func (s *ImageGenerationService) Generate(
	ctx context.Context,
	tokenMgr *token.TokenManager,
	seedToken string,
	modelInfo model.ModelInfo,
	prompt string,
	n int,
	responseFormat string,
	size string,
	aspectRatio string,
	stream bool,
	enableNSFW *bool,
) (*ImageResult, error) {
	maxTokenRetries := max(s.cfg.GetInt("retry.max_retry", 3), 8)
	tried := map[string]bool{}
	var lastErr error
	resolvedNSFW := s.cfg.GetBool("image.nsfw", true)
	if enableNSFW != nil {
		resolvedNSFW = *enableNSFW
	}
	var preferTags map[string]bool
	if resolvedNSFW {
		preferTags = map[string]bool{"nsfw": true}
	}

	for attempt := 0; attempt < maxTokenRetries; attempt++ {
		preferred := ""
		if attempt == 0 && len(preferTags) == 0 {
			preferred = seedToken
		}
		currentToken := s.pickToken(tokenMgr, modelInfo, tried, preferred, preferTags)
		if currentToken == "" {
			if lastErr != nil {
				return nil, lastErr
			}
			return nil, &reverse.UpstreamError{Message: "No available tokens. Please try again later.", StatusCode: http.StatusTooManyRequests}
		}
		tried[token.NormalizeToken(currentToken)] = true

		if stream {
			chunks, err := s.generateStreamChunks(ctx, tokenMgr, currentToken, modelInfo, prompt, n, responseFormat, size, aspectRatio, resolvedNSFW)
			if err == nil {
				return &ImageResult{Stream: true, StreamData: replayStream(ctx, chunks), Usage: zeroUsage()}, nil
			}
			lastErr = err
			if isRateLimited(err) {
				_ = tokenMgr.MarkRateLimited(currentToken)
				continue
			}
			return nil, err
		}

		images, err := s.generateImages(ctx, tokenMgr, currentToken, modelInfo, prompt, n, responseFormat, aspectRatio, resolvedNSFW)
		if err == nil {
			return &ImageResult{Stream: false, Data: images, Usage: zeroUsage()}, nil
		}
		lastErr = err
		if isRateLimited(err) {
			_ = tokenMgr.MarkRateLimited(currentToken)
			continue
		}
		return nil, err
	}

	if lastErr != nil {
		return nil, lastErr
	}
	return nil, &reverse.UpstreamError{Message: "No available tokens. Please try again later.", StatusCode: http.StatusTooManyRequests}
}

func (s *ImageGenerationService) generateStreamChunks(
	ctx context.Context,
	tokenMgr *token.TokenManager,
	currentToken string,
	modelInfo model.ModelInfo,
	prompt string,
	n int,
	responseFormat string,
	size string,
	aspectRatio string,
	enableNSFW bool,
) ([]string, error) {
	chunks, err := s.streamAppChat(ctx, currentToken, modelInfo, prompt, n, responseFormat, size, enableNSFW, tokenMgr)
	if err != nil && !isRateLimited(err) {
		chunks, err = s.streamWS(ctx, currentToken, prompt, n, responseFormat, size, aspectRatio, modelInfo.ModelID, enableNSFW)
	}
	if err != nil {
		return nil, err
	}
	if err := tokenMgr.Consume(currentToken, imageEffortForModel(modelInfo)); err != nil {
		// usage persistence should not fail the request
	}
	return chunks, nil
}

func (s *ImageGenerationService) generateImages(
	ctx context.Context,
	tokenMgr *token.TokenManager,
	currentToken string,
	modelInfo model.ModelInfo,
	prompt string,
	n int,
	responseFormat string,
	aspectRatio string,
	enableNSFW bool,
) ([]string, error) {
	images, err := s.collectAppChat(ctx, currentToken, modelInfo, prompt, n, responseFormat, enableNSFW, tokenMgr)
	if err != nil && !isRateLimited(err) {
		images, err = s.collectWS(ctx, currentToken, prompt, n, responseFormat, aspectRatio, enableNSFW)
	}
	if err != nil {
		return nil, err
	}
	if err := tokenMgr.Consume(currentToken, imageEffortForModel(modelInfo)); err != nil {
	}
	return images, nil
}

func (s *ImageGenerationService) collectWS(ctx context.Context, tokenValue, prompt string, n int, responseFormat, aspectRatio string, enableNSFW bool) ([]string, error) {
	streamRetries := min(max(s.cfg.GetInt("image.blocked_parallel_attempts", 5)+1, 1), 10)
	upstream := s.imagine.Stream(ctx, tokenValue, prompt, aspectRatio, n, enableNSFW, streamRetries)
	images := map[string]reverse.ImageEvent{}
	for item := range upstream {
		if item.Type == "error" {
			if item.ErrorCode == "rate_limit_exceeded" || item.Status == http.StatusTooManyRequests {
				return nil, &reverse.UpstreamError{Message: item.ErrorMsg, StatusCode: http.StatusTooManyRequests}
			}
			return nil, &reverse.UpstreamError{Message: item.ErrorMsg, StatusCode: http.StatusBadGateway}
		}
		if item.Type != "image" || item.ImageID == "" {
			continue
		}
		images[item.ImageID] = pickBestImage(images[item.ImageID], item)
	}
	selected := make([]reverse.ImageEvent, 0, len(images))
	for _, item := range images {
		if item.IsFinal {
			selected = append(selected, item)
		}
	}
	sort.Slice(selected, func(i, j int) bool { return selected[i].BlobSize > selected[j].BlobSize })
	if len(selected) > n {
		selected = selected[:n]
	}
	if len(selected) == 0 {
		return nil, &reverse.UpstreamError{Message: "Image generation blocked or no valid final image", StatusCode: http.StatusBadGateway}
	}
	results := make([]string, 0, len(selected))
	for _, item := range selected {
		output, err := s.wsOutput(item.ImageID, item, responseFormat)
		if err == nil && output != "" {
			results = append(results, output)
		}
	}
	if len(results) == 0 {
		return nil, &reverse.UpstreamError{Message: "Image generation returned no results", StatusCode: http.StatusBadGateway}
	}
	return results, nil
}

func (s *ImageGenerationService) streamWS(ctx context.Context, tokenValue, prompt string, n int, responseFormat, size, aspectRatio, modelID string, enableNSFW bool) ([]string, error) {
	streamRetries := min(max(s.cfg.GetInt("image.blocked_parallel_attempts", 5)+1, 1), 10)
	upstream := s.imagine.Stream(ctx, tokenValue, prompt, aspectRatio, n, enableNSFW, streamRetries)
	images := map[string]reverse.ImageEvent{}
	indexMap := map[string]int{}
	partialMap := map[string]int{}
	initialSent := map[string]bool{}
	targetID := ""
	chunks := make([]string, 0, 16)

	assignIndex := func(imageID string) (int, bool) {
		if existing, ok := indexMap[imageID]; ok {
			return existing, true
		}
		if len(indexMap) >= n {
			return 0, false
		}
		index := len(indexMap)
		indexMap[imageID] = index
		return index, true
	}

	for item := range upstream {
		if item.Type == "error" {
			if item.ErrorCode == "rate_limit_exceeded" || item.Status == http.StatusTooManyRequests {
				return nil, &reverse.UpstreamError{Message: item.ErrorMsg, StatusCode: http.StatusTooManyRequests}
			}
			chunks = append(chunks, sseEvent("error", map[string]any{"error": map[string]any{"message": item.ErrorMsg, "type": "server_error", "code": item.ErrorCode}}))
			return chunks, nil
		}
		if item.Type != "image" || item.ImageID == "" {
			continue
		}
		var index int
		var ok bool
		if n == 1 {
			if targetID == "" {
				targetID = item.ImageID
			}
			if item.ImageID != targetID {
				ok = false
			} else {
				index, ok = 0, true
			}
		} else {
			index, ok = assignIndex(item.ImageID)
		}
		images[item.ImageID] = pickBestImage(images[item.ImageID], item)
		if !ok {
			continue
		}
		if item.Stage != "final" {
			stage := item.Stage
			if stage == "" {
				stage = "preview"
			}
			if !initialSent[item.ImageID] {
				initialSent[item.ImageID] = true
				if stage == "medium" {
					partialMap[item.ImageID] = 1
				} else {
					partialMap[item.ImageID] = 0
				}
			} else {
				if stage == "preview" {
					continue
				}
				if stage == "medium" {
					partialMap[item.ImageID] = max(partialMap[item.ImageID], 1)
				}
			}
			partialIndex := partialMap[item.ImageID]
			output, err := s.wsPartialOutput(item.ImageID, stage, partialIndex, item, responseFormat)
			if err != nil || output == "" {
				continue
			}
			chunks = append(chunks, sseEvent("image_generation.partial_image", map[string]any{
				"type":                            "image_generation.partial_image",
				responseFieldName(responseFormat): output,
				"created_at":                      time.Now().Unix(),
				"size":                            size,
				"index":                           index,
				"partial_image_index":             partialIndex,
				"image_id":                        item.ImageID,
				"stage":                           stage,
			}))
		}
	}

	selected := selectFinalWSImages(images, indexMap, targetID, n)
	if len(selected) == 0 {
		return nil, &reverse.UpstreamError{Message: "Image generation blocked or no valid final image", StatusCode: http.StatusBadGateway}
	}
	for _, item := range selected {
		imageID := item.ImageID
		output, err := s.wsFinalOutput(imageID, item, responseFormat, modelID)
		if err != nil || output == "" {
			continue
		}
		index := 0
		if n > 1 {
			index = indexMap[imageID]
		}
		chunks = append(chunks, sseEvent("image_generation.completed", map[string]any{
			"type":                            "image_generation.completed",
			responseFieldName(responseFormat): output,
			"created_at":                      time.Now().Unix(),
			"size":                            size,
			"index":                           index,
			"image_id":                        imageID,
			"stage":                           "final",
			"usage":                           zeroUsage(),
		}))
	}
	if len(chunks) == 0 {
		return nil, &reverse.UpstreamError{Message: "Image generation returned no results", StatusCode: http.StatusBadGateway}
	}
	return chunks, nil
}

func (s *ImageGenerationService) collectAppChat(ctx context.Context, tokenValue string, modelInfo model.ModelInfo, prompt string, n int, responseFormat string, enableNSFW bool, tokenMgr *token.TokenManager) ([]string, error) {
	parsed, err := s.parseAppChatStream(ctx, tokenValue, modelInfo, prompt, n, enableNSFW, tokenMgr)
	if err != nil {
		return nil, err
	}
	downloader := reverse.NewDownloadService()
	defer downloader.Close()
	results := make([]string, 0, len(parsed.ImageURLs))
	for _, rawURL := range parsed.ImageURLs {
		output, err := appChatOutput(downloader, rawURL, tokenValue, responseFormat)
		if err == nil && output != "" {
			results = append(results, output)
		}
		if len(results) >= n {
			break
		}
	}
	if len(results) == 0 {
		return nil, &reverse.UpstreamError{Message: "Image generation returned no results", StatusCode: http.StatusBadGateway}
	}
	return results, nil
}

func (s *ImageGenerationService) streamAppChat(ctx context.Context, tokenValue string, modelInfo model.ModelInfo, prompt string, n int, responseFormat, size string, enableNSFW bool, tokenMgr *token.TokenManager) ([]string, error) {
	parsed, err := s.parseAppChatStream(ctx, tokenValue, modelInfo, prompt, n, enableNSFW, tokenMgr)
	if err != nil {
		return nil, err
	}
	downloader := reverse.NewDownloadService()
	defer downloader.Close()
	chunks := make([]string, 0, len(parsed.Progresses)+len(parsed.ImageURLs))
	ids := map[int]string{}
	for _, progress := range parsed.Progresses {
		if n == 1 && progress.ImageIndex != 0 {
			continue
		}
		imageID := ids[progress.ImageIndex]
		if imageID == "" {
			imageID = fmt.Sprintf("app-chat-%d-%d", time.Now().UnixMilli(), progress.ImageIndex)
			ids[progress.ImageIndex] = imageID
		}
		chunks = append(chunks, sseEvent("image_generation.partial_image", map[string]any{
			"type":                            "image_generation.partial_image",
			responseFieldName(responseFormat): "",
			"index":                           progress.ImageIndex,
			"progress":                        progress.Progress,
			"image_id":                        imageID,
			"size":                            size,
		}))
	}
	count := 0
	for _, rawURL := range parsed.ImageURLs {
		output, err := appChatOutput(downloader, rawURL, tokenValue, responseFormat)
		if err != nil || output == "" {
			continue
		}
		chunks = append(chunks, sseEvent("image_generation.completed", map[string]any{
			"type":                            "image_generation.completed",
			responseFieldName(responseFormat): output,
			"created_at":                      time.Now().Unix(),
			"size":                            size,
			"index":                           count,
			"image_id":                        ids[count],
			"stage":                           "final",
			"usage":                           zeroUsage(),
		}))
		count++
		if count >= n {
			break
		}
	}
	if len(chunks) == 0 {
		return nil, &reverse.UpstreamError{Message: "Image generation returned no results", StatusCode: http.StatusBadGateway}
	}
	return chunks, nil
}

func (s *ImageGenerationService) parseAppChatStream(ctx context.Context, tokenValue string, modelInfo model.ModelInfo, prompt string, n int, enableNSFW bool, tokenMgr *token.TokenManager) (*appChatParseResult, error) {
	session := reverse.NewResettableSession(reverse.SessionOptions{Browser: s.cfg.GetString("proxy.browser", "")})
	defer session.Close()
	appChat := reverse.NewAppChatReverse()
	appChat.RecordTokenFailure = func(ctx context.Context, tokenValue string, status int, reason string) error {
		return tokenMgr.RecordFail(tokenValue, status, reason)
	}
	requestOverrides := buildImageAppChatRequestOverrides(n, enableNSFW)
	body, err := appChat.Request(
		ctx,
		session,
		tokenValue,
		buildImageAppChatMessage(prompt),
		modelInfo.GrokModel,
		modelInfo.ModelMode,
		nil,
		map[string]any{"imageGen": true},
		nil,
		requestOverrides,
	)
	if err != nil {
		return nil, err
	}
	defer body.Close()

	result := &appChatParseResult{}
	seenURLs := map[string]bool{}
	scanner := bufio.NewScanner(body)
	scanner.Buffer(make([]byte, 64*1024), 16*1024*1024)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" || line == "data: [DONE]" {
			continue
		}
		if strings.HasPrefix(line, "data:") {
			line = strings.TrimSpace(strings.TrimPrefix(line, "data:"))
		}
		var payload map[string]any
		if err := json.Unmarshal([]byte(line), &payload); err != nil {
			continue
		}
		response := nestedMap(payload, "result", "response")
		if response == nil {
			continue
		}
		if progressMap := mapValue(response["streamingImageGenerationResponse"]); progressMap != nil {
			result.Progresses = append(result.Progresses, appProgress{ImageIndex: intValue(progressMap["imageIndex"]), Progress: intValue(progressMap["progress"])})
		}
		if card := mapValue(response["cardAttachment"]); card != nil {
			jsonData := stringValue(card["jsonData"])
			if jsonData != "" {
				var cardData map[string]any
				if json.Unmarshal([]byte(jsonData), &cardData) == nil {
					kind := stringValue(cardData["type"])
					if kind == "render_generated_image" || kind == "render_edited_image" {
						if chunk := mapValue(cardData["image_chunk"]); chunk != nil && intValue(chunk["progress"]) >= 100 {
							if imageURL := normalizeAppChatURL(stringValue(chunk["imageUrl"])); imageURL != "" && !seenURLs[imageURL] {
								seenURLs[imageURL] = true
								result.ImageURLs = append(result.ImageURLs, imageURL)
							}
						}
					}
				}
			}
		}
		for _, rawURL := range collectImageURLs(response["modelResponse"]) {
			if rawURL == "" || seenURLs[rawURL] {
				continue
			}
			seenURLs[rawURL] = true
			result.ImageURLs = append(result.ImageURLs, rawURL)
		}
	}
	if err := scanner.Err(); err != nil && !errors.Is(err, io.EOF) {
		return nil, err
	}
	if len(result.ImageURLs) == 0 {
		return nil, &reverse.UpstreamError{Message: "Image generation returned no results", StatusCode: http.StatusBadGateway}
	}
	return result, nil
}

func (s *ImageGenerationService) pickToken(tokenMgr *token.TokenManager, modelInfo model.ModelInfo, tried map[string]bool, preferred string, preferTags map[string]bool) string {
	poolName := token.BasicPoolName
	if modelInfo.Tier == model.TierSuper {
		poolName = token.SuperPoolName
	}
	if preferred != "" {
		normalized := token.NormalizeToken(preferred)
		if !tried[normalized] && tokenMgr.GetPoolNameForToken(normalized) == poolName {
			return normalized
		}
	}
	return tokenMgr.GetToken(poolName, tried, preferTags)
}

func buildImageAppChatMessage(prompt string) string {
	text := strings.TrimSpace(prompt)
	if text == "" {
		return prompt
	}
	if appChatGeneratePrefixRE.MatchString(text) {
		return text
	}
	return "Generate an image: " + text
}

func buildImageAppChatRequestOverrides(n int, enableNSFW bool) map[string]any {
	return map[string]any{
		"imageGenerationCount": max(1, n),
		"enableNsfw":           enableNSFW,
		"disableSearch":        true,
	}
}

func pickBestImage(existing, incoming reverse.ImageEvent) reverse.ImageEvent {
	if existing.ImageID == "" {
		return incoming
	}
	if incoming.IsFinal && !existing.IsFinal {
		return incoming
	}
	if existing.IsFinal && !incoming.IsFinal {
		return existing
	}
	if incoming.BlobSize > existing.BlobSize {
		return incoming
	}
	return existing
}

func selectFinalWSImages(images map[string]reverse.ImageEvent, indexMap map[string]int, targetID string, n int) []reverse.ImageEvent {
	if n == 1 {
		if targetID != "" {
			if item, ok := images[targetID]; ok && item.IsFinal {
				return []reverse.ImageEvent{item}
			}
		}
		all := make([]reverse.ImageEvent, 0, len(images))
		for _, item := range images {
			all = append(all, item)
		}
		sort.Slice(all, func(i, j int) bool {
			if all[i].IsFinal != all[j].IsFinal {
				return all[i].IsFinal
			}
			return all[i].BlobSize > all[j].BlobSize
		})
		if len(all) > 0 {
			return all[:1]
		}
		return nil
	}
	pairs := make([]struct {
		id   string
		item reverse.ImageEvent
	}, 0, len(indexMap))
	for id, index := range indexMap {
		item, ok := images[id]
		if !ok || !item.IsFinal {
			continue
		}
		pairs = append(pairs, struct {
			id   string
			item reverse.ImageEvent
		}{id: fmt.Sprintf("%08d:%s", index, id), item: item})
	}
	sort.Slice(pairs, func(i, j int) bool { return pairs[i].id < pairs[j].id })
	selected := make([]reverse.ImageEvent, 0, len(pairs))
	for _, pair := range pairs {
		selected = append(selected, pair.item)
	}
	return selected
}

func (s *ImageGenerationService) wsPartialOutput(imageID, stage string, partialIndex int, item reverse.ImageEvent, responseFormat string) (string, error) {
	if responseFormat == "url" {
		return s.saveBlob(fmt.Sprintf("%s-%s-%d", imageID, stage, partialIndex), item.Blob, false, item.Ext)
	}
	return stripBase64(item.Blob), nil
}

func (s *ImageGenerationService) wsOutput(imageID string, item reverse.ImageEvent, responseFormat string) (string, error) {
	if responseFormat == "url" {
		return s.saveBlob(imageID, item.Blob, item.IsFinal, item.Ext)
	}
	return stripBase64(item.Blob), nil
}

func (s *ImageGenerationService) wsFinalOutput(imageID string, item reverse.ImageEvent, responseFormat, modelID string) (string, error) {
	if responseFormat == "url" {
		finalID := imageID
		if modelID != "grok-imagine-1.0-fast" {
			finalID = imageID + "-final"
		}
		return s.saveBlob(finalID, item.Blob, item.IsFinal, item.Ext)
	}
	return stripBase64(item.Blob), nil
}

func (s *ImageGenerationService) saveBlob(imageID, blob string, isFinal bool, ext string) (string, error) {
	data := stripBase64(blob)
	if data == "" {
		return "", nil
	}
	decoded, err := base64.StdEncoding.DecodeString(data)
	if err != nil {
		return "", err
	}
	if ext == "" {
		ext = guessExt(blob)
	}
	if ext == "" {
		if isFinal {
			ext = "jpg"
		} else {
			ext = "png"
		}
	}
	if ext == "jpeg" {
		ext = "jpg"
	}
	imageDir := filepath.Join(storage.DataDir(), "tmp", "image")
	if err := os.MkdirAll(imageDir, 0o755); err != nil {
		return "", err
	}
	filename := fmt.Sprintf("%s.%s", imageID, ext)
	if err := os.WriteFile(filepath.Join(imageDir, filename), decoded, 0o644); err != nil {
		return "", err
	}
	appURL := strings.TrimSpace(s.cfg.GetString("app.app_url", ""))
	if appURL != "" {
		return fmt.Sprintf("%s/v1/files/image/%s", strings.TrimRight(appURL, "/"), filename), nil
	}
	return "/v1/files/image/" + filename, nil
}

func stripBase64(blob string) string {
	if blob == "" {
		return ""
	}
	if idx := strings.Index(blob, ","); idx >= 0 && strings.Contains(strings.ToLower(blob[:idx]), "base64") {
		return blob[idx+1:]
	}
	return blob
}

func guessExt(blob string) string {
	header := ""
	data := blob
	if idx := strings.Index(blob, ","); idx >= 0 {
		header = strings.ToLower(blob[:idx])
		data = blob[idx+1:]
	}
	switch {
	case strings.Contains(header, "image/png") || strings.HasPrefix(data, "iVBORw0KGgo"):
		return "png"
	case strings.Contains(header, "image/jpeg"), strings.Contains(header, "image/jpg"), strings.HasPrefix(data, "/9j/"):
		return "jpg"
	default:
		return ""
	}
}

func appChatOutput(downloader *reverse.DownloadService, rawURL, tokenValue, responseFormat string) (string, error) {
	if responseFormat == "url" {
		return downloader.ResolveURL(rawURL, tokenValue, "image")
	}
	dataURI, err := downloader.ParseB64(rawURL, tokenValue, "image")
	if err != nil {
		return "", err
	}
	if idx := strings.Index(dataURI, ","); idx >= 0 {
		return dataURI[idx+1:], nil
	}
	return dataURI, nil
}

func normalizeAppChatURL(raw string) string {
	value := strings.TrimSpace(raw)
	if value == "" {
		return ""
	}
	if strings.HasPrefix(value, "http://") || strings.HasPrefix(value, "https://") {
		return value
	}
	return strings.TrimRight(reverse.AssetsDownloadAPI, "/") + "/" + strings.TrimLeft(value, "/")
}

func collectImageURLs(value any) []string {
	seen := map[string]bool{}
	urls := make([]string, 0)
	var walk func(any)
	walk = func(node any) {
		switch typed := node.(type) {
		case map[string]any:
			for key, child := range typed {
				switch key {
				case "generatedImageUrls", "imageUrls", "imageURLs":
					switch list := child.(type) {
					case []any:
						for _, item := range list {
							if raw, ok := item.(string); ok && raw != "" && !seen[raw] {
								seen[raw] = true
								urls = append(urls, raw)
							}
						}
					case []string:
						for _, raw := range list {
							if raw != "" && !seen[raw] {
								seen[raw] = true
								urls = append(urls, raw)
							}
						}
					case string:
						if list != "" && !seen[list] {
							seen[list] = true
							urls = append(urls, list)
						}
					}
				default:
					walk(child)
				}
			}
		case []any:
			for _, child := range typed {
				walk(child)
			}
		}
	}
	walk(value)
	return urls
}

func nestedMap(root map[string]any, keys ...string) map[string]any {
	current := root
	for _, key := range keys {
		next, ok := current[key]
		if !ok {
			return nil
		}
		mapped, ok := next.(map[string]any)
		if !ok {
			return nil
		}
		current = mapped
	}
	return current
}

func mapValue(value any) map[string]any {
	mapped, _ := value.(map[string]any)
	return mapped
}

func intValue(value any) int {
	switch typed := value.(type) {
	case float64:
		return int(typed)
	case int:
		return typed
	case int64:
		return int(typed)
	case json.Number:
		parsed, _ := typed.Int64()
		return int(parsed)
	default:
		return 0
	}
}

func replayStream(ctx context.Context, chunks []string) <-chan string {
	out := make(chan string, 8)
	go func() {
		defer close(out)
		for _, chunk := range chunks {
			select {
			case <-ctx.Done():
				return
			case out <- chunk:
			}
		}
	}()
	return out
}

func sseEvent(event string, payload any) string {
	data, _ := json.Marshal(payload)
	return fmt.Sprintf("event: %s\ndata: %s\n\n", event, data)
}

func responseFieldName(responseFormat string) string {
	if responseFormat == "url" {
		return "url"
	}
	if responseFormat == "base64" {
		return "base64"
	}
	return "b64_json"
}

func zeroUsage() map[string]any {
	return map[string]any{
		"total_tokens":  0,
		"input_tokens":  0,
		"output_tokens": 0,
		"input_tokens_details": map[string]any{
			"text_tokens":  0,
			"image_tokens": 0,
		},
	}
}

func imageEffortForModel(modelInfo model.ModelInfo) token.EffortType {
	if modelInfo.Cost == model.CostHigh {
		return token.EffortHigh
	}
	return token.EffortLow
}

func isRateLimited(err error) bool {
	var upstream *reverse.UpstreamError
	return errors.As(err, &upstream) && upstream.StatusCode == http.StatusTooManyRequests
}
