package service

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"html"
	"io"
	"log/slog"
	"math"
	"net/http"
	"regexp"
	"strings"
	"time"

	"github.com/google/uuid"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/model"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/token"
)

const (
	VideoModelID   = "grok-imagine-1.0-video"
	appChatModel   = "grok-3"
	mediaTypeVideo = "MEDIA_POST_TYPE_VIDEO"
)

var (
	postIDURLPattern       = regexp.MustCompile(`/generated/([0-9a-fA-F-]{32,36})/`)
	referencePlaceholderRE = regexp.MustCompile(`@(?:(?:图|image|img)\s*(\d+))`)
)

type VideoError struct {
	StatusCode int
	Message    string
	Param      string
	Code       string
}

func (e *VideoError) Error() string {
	if e == nil {
		return "<nil>"
	}
	return e.Message
}

type VideoRoundPlan struct {
	RoundIndex         int
	TotalRounds        int
	IsExtension        bool
	VideoLength        int
	ExtensionStartTime *float64
}

type VideoRoundResult struct {
	ResponseID    string
	PostID        string
	PostIDRank    int
	VideoURL      string
	ThumbnailURL  string
	LastProgress  any
	SawVideoEvent bool
	StreamErrors  []string
}

type CompletionParams struct {
	Model           string
	Messages        []map[string]any
	Stream          bool
	ReasoningEffort string
	AspectRatio     string
	VideoLength     int
	Resolution      string
	Preset          string
}

type GenerateParams struct {
	Model       string
	Prompt      string
	AspectRatio string
	VideoLength int
	Resolution  string
	Preset      string
	Stream      bool
}

type VideoCompletionResult struct {
	Response      map[string]any
	StreamChunks  []string
	FinalVideoURL string
	ThumbnailURL  string
	Rendered      string
	ResponseID    string
}

type VideoService struct {
	cfg           *config.Config
	models        *model.Service
	appChat       *reverse.AppChatReverse
	mediaPost     *reverse.MediaPostReverse
	mediaPostLink *reverse.MediaPostLinkReverse
	videoUpscale  *reverse.VideoUpscaleReverse
	semaphore     chan struct{}
}

func NewVideoService(cfg *config.Config, models *model.Service) *VideoService {
	concurrent := 100
	if cfg != nil {
		concurrent = max(1, cfg.GetInt("video.concurrent", 100))
	}
	return &VideoService{
		cfg:           cfg,
		models:        models,
		appChat:       reverse.NewAppChatReverse(),
		mediaPost:     reverse.NewMediaPostReverse(),
		mediaPostLink: reverse.NewMediaPostLinkReverse(),
		videoUpscale:  reverse.NewVideoUpscaleReverse(),
		semaphore:     make(chan struct{}, concurrent),
	}
}

func (s *VideoService) Generate(ctx context.Context, params GenerateParams) (*VideoCompletionResult, error) {
	content := []map[string]any{{"type": "text", "text": strings.TrimSpace(params.Prompt)}}
	return s.Completions(ctx, CompletionParams{
		Model:       strings.TrimSpace(params.Model),
		Messages:    []map[string]any{{"role": "user", "content": content}},
		Stream:      params.Stream,
		AspectRatio: strings.TrimSpace(params.AspectRatio),
		VideoLength: params.VideoLength,
		Resolution:  strings.TrimSpace(params.Resolution),
		Preset:      strings.TrimSpace(params.Preset),
	})
}

func (s *VideoService) Completions(ctx context.Context, params CompletionParams) (*VideoCompletionResult, error) {
	if s == nil || s.models == nil {
		return nil, &VideoError{StatusCode: http.StatusInternalServerError, Message: "video service is not initialized", Code: "service_unavailable"}
	}
	modelID := strings.TrimSpace(params.Model)
	if modelID == "" {
		modelID = VideoModelID
	}
	modelInfo, ok := s.models.Get(modelID)
	if !ok || !modelInfo.IsVideo {
		return nil, &VideoError{StatusCode: http.StatusBadRequest, Message: fmt.Sprintf("The model `%s` is not supported for video generation.", modelID), Param: "model", Code: "model_not_supported"}
	}

	prompt, imageReferences := extractLastUserPromptAndImages(params.Messages)
	if strings.TrimSpace(prompt) == "" {
		return nil, &VideoError{StatusCode: http.StatusBadRequest, Message: "prompt is required", Param: "prompt", Code: "invalid_request_error"}
	}
	if len(imageReferences) > 7 {
		return nil, &VideoError{StatusCode: http.StatusBadRequest, Message: "Video generation supports at most 7 reference images", Param: "messages", Code: "invalid_reference"}
	}
	if referencePlaceholderRE.MatchString(prompt) && len(imageReferences) == 0 {
		return nil, &VideoError{StatusCode: http.StatusBadRequest, Message: "Reference placeholders require uploaded images", Param: "prompt", Code: "invalid_reference"}
	}

	videoLength := params.VideoLength
	if videoLength <= 0 {
		videoLength = 6
	}
	resolution := defaultString(params.Resolution, "480p")
	aspectRatio := defaultString(params.AspectRatio, "3:2")
	preset := defaultString(params.Preset, "custom")
	showThink := s.cfg == nil || s.cfg.GetBool("app.thinking", true)
	if strings.TrimSpace(params.ReasoningEffort) != "" {
		showThink = !strings.EqualFold(strings.TrimSpace(params.ReasoningEffort), "none")
	}

	maxRetries := 1
	if s.cfg != nil {
		maxRetries = max(1, s.cfg.GetInt("retry.max_retry", 1))
	}
	manager := token.GetInstance()
	manager.ReloadIfStale()

	var lastErr error
	for attempt := 0; attempt < maxRetries; attempt++ {
		tokenInfo := manager.GetTokenForVideo(resolution, videoLength, s.models.PoolCandidatesForModel(modelID))
		if tokenInfo == nil {
			if lastErr != nil {
				return nil, lastErr
			}
			return nil, &VideoError{StatusCode: http.StatusTooManyRequests, Message: "No available tokens. Please try again later.", Code: "rate_limit_exceeded"}
		}
		tokenValue := token.NormalizeToken(tokenInfo.Token)
		result, err := s.generateWithToken(ctx, generateExecutionParams{
			ModelID:         modelID,
			ModelInfo:       modelInfo,
			Token:           tokenValue,
			Prompt:          prompt,
			ImageReferences: imageReferences,
			Stream:          params.Stream,
			ShowThink:       showThink,
			AspectRatio:     aspectRatio,
			VideoLength:     videoLength,
			Resolution:      resolution,
			Preset:          preset,
		})
		if err == nil {
			return result, nil
		}
		lastErr = err

		var upstreamErr *reverse.UpstreamError
		if errors.As(err, &upstreamErr) {
			if upstreamErr.StatusCode == http.StatusTooManyRequests {
				_ = manager.MarkRateLimited(tokenValue)
			}
			if upstreamErr.StatusCode == http.StatusUnauthorized || upstreamErr.StatusCode == http.StatusForbidden || upstreamErr.StatusCode == http.StatusTooManyRequests || upstreamErr.StatusCode == http.StatusBadGateway {
				continue
			}
		}

		var serviceErr *VideoError
		if errors.As(err, &serviceErr) && serviceErr.StatusCode < 500 && serviceErr.StatusCode != http.StatusTooManyRequests {
			return nil, err
		}
		if attempt == maxRetries-1 {
			break
		}
	}

	if lastErr != nil {
		return nil, lastErr
	}
	return nil, &VideoError{StatusCode: http.StatusBadGateway, Message: "video generation failed", Code: "upstream_error"}
}

type generateExecutionParams struct {
	ModelID         string
	ModelInfo       model.ModelInfo
	Token           string
	Prompt          string
	ImageReferences []string
	Stream          bool
	ShowThink       bool
	AspectRatio     string
	VideoLength     int
	Resolution      string
	Preset          string
}

func (s *VideoService) generateWithToken(ctx context.Context, params generateExecutionParams) (*VideoCompletionResult, error) {
	poolName := s.poolName(params.Token)
	isSuperPool := poolName != token.BasicPoolName
	shouldUpscale := params.Resolution == "720p" && poolName == token.BasicPoolName
	generationResolution := params.Resolution
	if shouldUpscale {
		generationResolution = "480p"
	}
	upscaleTiming := "complete"
	if shouldUpscale {
		upscaleTiming = s.resolveUpscaleTiming()
	}

	roundPlan := BuildRoundPlan(params.VideoLength, isSuperPool)
	seedPostID, err := s.CreatePost(ctx, params.Token, params.Prompt, mediaTypeVideo, "")
	if err != nil {
		return nil, err
	}
	message := buildMessage(params.Prompt, params.Preset)
	builder := newVideoSSEBuilder(params.ModelID, params.ShowThink)

	seedID := seedPostID
	lastID := seedID
	originalID := seedID
	finalResult := VideoRoundResult{PostIDRank: 999}

	for _, plan := range roundPlan {
		configOverride, err := buildRoundConfig(plan, seedID, lastID, originalID, params.Prompt, params.AspectRatio, generationResolution, firstRoundReferences(plan.RoundIndex, params.ImageReferences))
		if err != nil {
			return nil, err
		}
		roundResult, err := s.runRound(ctx, params.Token, message, configOverride, func(progress any) {
			if params.Stream {
				builder.Append(builder.EmitProgress(plan.RoundIndex, plan.TotalRounds, progress)...)
			}
		})
		if err != nil {
			return nil, err
		}
		if err := ensureRoundResult(roundResult, plan.RoundIndex, plan.TotalRounds, plan.RoundIndex == plan.TotalRounds); err != nil {
			return nil, err
		}

		if shouldUpscale && upscaleTiming == "single" && roundResult.VideoURL != "" {
			if params.Stream {
				builder.Append(builder.EmitNote(fmt.Sprintf("[round=%d/%d] 正在对当前轮结果进行超分辨率\n", plan.RoundIndex, plan.TotalRounds))...)
			}
			if upscaledURL, ok := s.upscaleVideoURL(ctx, params.Token, roundResult.VideoURL); ok {
				roundResult.VideoURL = upscaledURL
			} else {
				slog.Warn("video upscale failed in single mode, falling back to 480p result")
			}
		}

		if plan.RoundIndex == 1 && roundResult.PostID != "" {
			originalID = roundResult.PostID
		}
		if roundResult.PostID != "" {
			lastID = roundResult.PostID
		}
		if plan.RoundIndex == plan.TotalRounds {
			finalResult = roundResult
		}
	}

	if finalResult.VideoURL == "" {
		return nil, &reverse.UpstreamError{Message: "video generation produced no final round", StatusCode: http.StatusBadGateway}
	}

	finalVideoURL := finalResult.VideoURL
	if shouldUpscale && upscaleTiming == "complete" {
		if params.Stream {
			builder.Append(builder.EmitNote("正在对视频进行超分辨率\n")...)
		}
		if upscaledURL, ok := s.upscaleVideoURL(ctx, params.Token, finalVideoURL); ok {
			finalVideoURL = upscaledURL
		} else {
			slog.Warn("video upscale failed, falling back to 480p result")
		}
	}

	if s.publicAssetEnabled() {
		if params.Stream {
			builder.Append(builder.EmitNote("正在生成可公开访问链接\n")...)
		}
		finalVideoURL = s.createPublicVideoLink(ctx, params.Token, finalVideoURL)
	}

	rendered := s.renderVideo(finalVideoURL, finalResult.ThumbnailURL)
	consumeEffort := token.EffortLow
	if params.ModelInfo.Cost == model.CostHigh {
		consumeEffort = token.EffortHigh
	}
	if err := token.GetInstance().Consume(params.Token, consumeEffort); err != nil {
		slog.Warn("record video usage failed", "error", err)
	}

	responseID := finalResult.ResponseID
	if responseID == "" {
		responseID = "chatcmpl-" + strings.ReplaceAll(uuid.NewString(), "-", "")[:24]
	}
	result := &VideoCompletionResult{
		FinalVideoURL: finalVideoURL,
		ThumbnailURL:  finalResult.ThumbnailURL,
		Rendered:      rendered,
		ResponseID:    responseID,
	}
	if params.Stream {
		builder.Append(builder.EmitContent(rendered)...)
		builder.Append(builder.Finish()...)
		result.StreamChunks = append([]string(nil), builder.chunks...)
		return result, nil
	}

	result.Response = map[string]any{
		"id":      responseID,
		"object":  "chat.completion",
		"created": time.Now().Unix(),
		"model":   params.ModelID,
		"choices": []map[string]any{{
			"index": 0,
			"message": map[string]any{
				"role":    "assistant",
				"content": rendered,
				"refusal": nil,
			},
			"finish_reason": "stop",
		}},
		"usage": map[string]any{
			"prompt_tokens":     0,
			"completion_tokens": 0,
			"total_tokens":      0,
		},
	}
	return result, nil
}

func (s *VideoService) CreatePost(ctx context.Context, tokenValue, prompt, mediaType, mediaURL string) (string, error) {
	if mediaType == "" {
		mediaType = mediaTypeVideo
	}
	s.acquire()
	defer s.release()
	session := reverse.NewResettableSession(reverse.SessionOptions{})
	defer session.Close()
	return s.mediaPost.Request(ctx, session, tokenValue, mediaType, mediaURL, prompt)
}

func BuildRoundPlan(targetLength int, isSuper bool) []VideoRoundPlan {
	roundLength := chooseRoundLength(targetLength, isSuper)
	extensionRounds := int(math.Ceil(float64(max(targetLength-roundLength, 0)) / float64(roundLength)))
	totalRounds := 1 + extensionRounds
	plan := []VideoRoundPlan{{RoundIndex: 1, TotalRounds: totalRounds, IsExtension: false, VideoLength: roundLength}}
	for i := 1; i <= extensionRounds; i++ {
		roundTarget := min(targetLength, roundLength*(i+1))
		start := float64(roundTarget - roundLength)
		plan = append(plan, VideoRoundPlan{RoundIndex: i + 1, TotalRounds: totalRounds, IsExtension: true, VideoLength: roundLength, ExtensionStartTime: &start})
	}
	return plan
}

func chooseRoundLength(targetLength int, isSuper bool) int {
	if !isSuper {
		return 6
	}
	if targetLength >= 10 {
		return 10
	}
	return 6
}

func buildBaseConfig(parentPostID, aspectRatio, resolutionName string, videoLength int) map[string]any {
	return map[string]any{
		"modelMap": map[string]any{
			"videoGenModelConfig": map[string]any{
				"aspectRatio":    aspectRatio,
				"parentPostId":   parentPostID,
				"resolutionName": resolutionName,
				"videoLength":    videoLength,
			},
		},
	}
}

func buildExtensionConfig(parentPostID, extendPostID, originalPostID, originalPrompt, aspectRatio, resolutionName string, videoLength int, startTime float64) map[string]any {
	return map[string]any{
		"modelMap": map[string]any{
			"videoGenModelConfig": map[string]any{
				"isVideoExtension":        true,
				"videoExtensionStartTime": startTime,
				"extendPostId":            extendPostID,
				"stitchWithExtendPostId":  true,
				"originalPrompt":          originalPrompt,
				"originalPostId":          originalPostID,
				"originalRefType":         "ORIGINAL_REF_TYPE_VIDEO_EXTENSION",
				"mode":                    "custom",
				"aspectRatio":             aspectRatio,
				"videoLength":             videoLength,
				"resolutionName":          resolutionName,
				"parentPostId":            parentPostID,
				"isVideoEdit":             false,
			},
		},
	}
}

func buildRoundConfig(plan VideoRoundPlan, seedPostID, lastPostID, originalPostID, prompt, aspectRatio, resolutionName string, imageReferences []string) (map[string]any, error) {
	if !plan.IsExtension {
		config := buildBaseConfig(seedPostID, aspectRatio, resolutionName, plan.VideoLength)
		if len(imageReferences) > 0 {
			videoConfig := config["modelMap"].(map[string]any)["videoGenModelConfig"].(map[string]any)
			videoConfig["imageReferences"] = append([]string(nil), imageReferences...)
			videoConfig["isReferenceToVideo"] = true
		}
		return config, nil
	}
	if originalPostID == "" {
		return nil, &reverse.UpstreamError{Message: "video extension missing original post id", StatusCode: http.StatusBadGateway}
	}
	startTime := 0.0
	if plan.ExtensionStartTime != nil {
		startTime = *plan.ExtensionStartTime
	}
	return buildExtensionConfig(lastPostID, lastPostID, originalPostID, prompt, aspectRatio, resolutionName, plan.VideoLength, startTime), nil
}

func (s *VideoService) runRound(ctx context.Context, tokenValue, message string, modelConfigOverride map[string]any, onProgress func(any)) (VideoRoundResult, error) {
	s.acquire()
	defer s.release()
	session := reverse.NewResettableSession(reverse.SessionOptions{})
	defer session.Close()
	body, err := s.appChat.Request(ctx, session, tokenValue, message, appChatModel, "", nil, map[string]any{"videoGen": true}, modelConfigOverride, nil)
	if err != nil {
		return VideoRoundResult{}, err
	}
	defer body.Close()
	return s.collectRoundResult(ctx, body, onProgress)
}

func (s *VideoService) collectRoundResult(ctx context.Context, body io.ReadCloser, onProgress func(any)) (VideoRoundResult, error) {
	result := VideoRoundResult{PostIDRank: 999}
	idleTimeout := 60 * time.Second
	if s.cfg != nil {
		idleTimeout = time.Duration(s.cfg.GetInt("video.stream_timeout", 60)) * time.Second
		if idleTimeout <= 0 {
			idleTimeout = 60 * time.Second
		}
	}
	if err := iterateResponseLines(ctx, body, idleTimeout, func(line []byte) error {
		line = trimJSONLine(line)
		if len(line) == 0 {
			return nil
		}
		var payload map[string]any
		if err := json.Unmarshal(line, &payload); err != nil {
			return nil
		}
		root, _ := payload["result"].(map[string]any)
		resp, _ := root["response"].(map[string]any)
		if len(resp) == 0 {
			return nil
		}

		if responseID := pickString(resp["responseId"]); responseID != "" {
			result.ResponseID = responseID
		}
		appendUniqueErrors(&result.StreamErrors, resp["streamErrors"])
		if modelResp, _ := resp["modelResponse"].(map[string]any); len(modelResp) > 0 {
			if responseID := pickString(modelResp["responseId"]); responseID != "" {
				result.ResponseID = responseID
			}
			appendUniqueErrors(&result.StreamErrors, modelResp["streamErrors"])
		}
		applyPostIDCandidates(&result, extractPostIDCandidates(resp))

		if videoResp, _ := resp["streamingVideoGenerationResponse"].(map[string]any); len(videoResp) > 0 {
			result.SawVideoEvent = true
			if progress, ok := videoResp["progress"]; ok {
				result.LastProgress = progress
				if onProgress != nil {
					onProgress(progress)
				}
			}
			if videoURL := pickString(videoResp["videoUrl"]); videoURL != "" {
				result.VideoURL = videoURL
			}
			if thumbnailURL := pickString(videoResp["thumbnailImageUrl"]); thumbnailURL != "" {
				result.ThumbnailURL = thumbnailURL
			}
		}
		if result.PostID == "" && result.VideoURL != "" {
			if postID := extractPostIDFromVideoURL(result.VideoURL); postID != "" {
				result.PostID = postID
				result.PostIDRank = 6
			}
		}
		return nil
	}); err != nil {
		return VideoRoundResult{}, err
	}
	if result.PostID == "" && result.VideoURL != "" {
		if postID := extractPostIDFromVideoURL(result.VideoURL); postID != "" {
			result.PostID = postID
			result.PostIDRank = 6
		}
	}
	return result, nil
}

func iterateResponseLines(ctx context.Context, body io.ReadCloser, idleTimeout time.Duration, onLine func([]byte) error) error {
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()
	type readResult struct {
		line []byte
		err  error
	}
	results := make(chan readResult, 1)
	send := func(result readResult) bool {
		select {
		case results <- result:
			return true
		case <-ctx.Done():
			return false
		}
	}
	go func() {
		defer close(results)
		reader := bufio.NewReader(body)
		for {
			line, err := reader.ReadBytes('\n')
			if len(line) > 0 {
				if !send(readResult{line: append([]byte(nil), line...)}) {
					return
				}
			}
			if err != nil {
				send(readResult{err: err})
				return
			}
		}
	}()

	timer := time.NewTimer(idleTimeout)
	defer timer.Stop()
	for {
		select {
		case <-ctx.Done():
			_ = body.Close()
			return ctx.Err()
		case <-timer.C:
			_ = body.Close()
			return &reverse.UpstreamError{Message: fmt.Sprintf("video stream idle timeout after %ds", int(idleTimeout.Seconds())), StatusCode: http.StatusGatewayTimeout}
		case result, ok := <-results:
			if !ok {
				return nil
			}
			if result.line != nil {
				if !timer.Stop() {
					select {
					case <-timer.C:
					default:
					}
				}
				timer.Reset(idleTimeout)
				if err := onLine(result.line); err != nil {
					return err
				}
			}
			if result.err != nil {
				if errors.Is(result.err, io.EOF) {
					return nil
				}
				return &reverse.UpstreamError{Message: fmt.Sprintf("video stream read failed: %v", result.err), StatusCode: http.StatusBadGateway, Cause: result.err}
			}
		}
	}
}

func (s *VideoService) upscaleVideoURL(ctx context.Context, tokenValue, videoURL string) (string, bool) {
	session := reverse.NewResettableSession(reverse.SessionOptions{})
	defer session.Close()
	upscaledURL, err := s.videoUpscale.Request(ctx, session, tokenValue, videoURL)
	if err != nil || strings.TrimSpace(upscaledURL) == "" {
		if err != nil {
			slog.Warn("video upscale failed", "error", err)
		}
		return videoURL, false
	}
	return upscaledURL, true
}

func (s *VideoService) createPublicVideoLink(ctx context.Context, tokenValue, videoURL string) string {
	if !s.publicAssetEnabled() || strings.TrimSpace(videoURL) == "" {
		return videoURL
	}
	videoID := reverse.NormalizeVideoID(videoURL)
	if videoID == "" {
		slog.Warn("video public link skipped: unable to extract video id")
		return videoURL
	}
	session := reverse.NewResettableSession(reverse.SessionOptions{})
	defer session.Close()
	shareLink, err := s.mediaPostLink.Request(ctx, session, tokenValue, videoID)
	if err != nil {
		slog.Warn("video public link failed", "error", err)
		return videoURL
	}
	if strings.HasSuffix(strings.ToLower(shareLink), ".mp4") {
		return shareLink
	}
	if shareLink != "" {
		return fmt.Sprintf("https://imagine-public.x.ai/imagine-public/share-videos/%s.mp4?cache=1", videoID)
	}
	return videoURL
}

func (s *VideoService) publicAssetEnabled() bool {
	return s.cfg != nil && s.cfg.GetBool("video.enable_public_asset", false)
}

func (s *VideoService) resolveUpscaleTiming() string {
	if s.cfg == nil {
		return "complete"
	}
	value := strings.ToLower(strings.TrimSpace(s.cfg.GetString("video.upscale_timing", "complete")))
	if value == "single" || value == "complete" {
		return value
	}
	slog.Warn("invalid video upscale timing, falling back to complete", "value", value)
	return "complete"
}

func (s *VideoService) renderVideo(videoURL, thumbnailURL string) string {
	format := "url"
	if s.cfg != nil {
		format = strings.ToLower(strings.TrimSpace(s.cfg.GetString("app.video_format", "url")))
	}
	switch format {
	case "markdown":
		return fmt.Sprintf("[video](%s)", videoURL)
	case "html":
		posterAttr := ""
		if strings.TrimSpace(thumbnailURL) != "" {
			posterAttr = fmt.Sprintf(` poster="%s"`, html.EscapeString(thumbnailURL))
		}
		return fmt.Sprintf("<video id=\"video\" controls=\"\" preload=\"none\"%s>\n  <source id=\"mp4\" src=\"%s\" type=\"video/mp4\">\n</video>", posterAttr, html.EscapeString(videoURL))
	default:
		return videoURL + "\n"
	}
}

func (s *VideoService) poolName(tokenValue string) string {
	poolName := token.GetInstance().GetPoolNameForToken(tokenValue)
	if poolName == "" {
		return token.BasicPoolName
	}
	return poolName
}

func (s *VideoService) acquire() {
	s.semaphore <- struct{}{}
}

func (s *VideoService) release() {
	<-s.semaphore
}

func extractLastUserPromptAndImages(messages []map[string]any) (string, []string) {
	for i := len(messages) - 1; i >= 0; i-- {
		msg := messages[i]
		if role := pickString(msg["role"]); role != "" && role != "user" {
			continue
		}
		switch typed := msg["content"].(type) {
		case string:
			return strings.TrimSpace(typed), nil
		case []any:
			return extractPromptFromBlocks(typed)
		case []map[string]any:
			items := make([]any, 0, len(typed))
			for _, item := range typed {
				items = append(items, item)
			}
			return extractPromptFromBlocks(items)
		case map[string]any:
			return extractPromptFromBlocks([]any{typed})
		}
	}
	return "", nil
}

func extractPromptFromBlocks(blocks []any) (string, []string) {
	parts := make([]string, 0, len(blocks))
	imageURLs := make([]string, 0)
	for _, raw := range blocks {
		block, _ := raw.(map[string]any)
		if len(block) == 0 {
			continue
		}
		switch pickString(block["type"]) {
		case "text":
			if text := pickString(block["text"]); text != "" {
				parts = append(parts, text)
			}
		case "image_url":
			switch value := block["image_url"].(type) {
			case map[string]any:
				if url := pickString(value["url"]); url != "" {
					imageURLs = append(imageURLs, url)
				}
			case string:
				if url := strings.TrimSpace(value); url != "" {
					imageURLs = append(imageURLs, url)
				}
			}
		}
	}
	prompt := strings.TrimSpace(strings.Join(parts, "\n"))
	if prompt == "" && len(imageURLs) > 0 {
		prompt = "Refer to the following content:"
	}
	return prompt, imageURLs
}

func buildMessage(prompt, preset string) string {
	modeMap := map[string]string{
		"fun":    "--mode=extremely-crazy",
		"normal": "--mode=normal",
		"spicy":  "--mode=extremely-spicy-or-crazy",
		"custom": "--mode=custom",
	}
	mode := modeMap[strings.ToLower(strings.TrimSpace(preset))]
	if mode == "" {
		mode = "--mode=custom"
	}
	return strings.TrimSpace(strings.TrimSpace(prompt) + " " + mode)
}

func firstRoundReferences(roundIndex int, imageReferences []string) []string {
	if roundIndex != 1 || len(imageReferences) == 0 {
		return nil
	}
	return append([]string(nil), imageReferences...)
}

func ensureRoundResult(result VideoRoundResult, roundIndex, totalRounds int, finalRound bool) error {
	if result.PostID == "" {
		errType := "missing_post_id"
		if len(result.StreamErrors) > 0 {
			errType = "moderated_or_stream_errors"
		}
		return &reverse.UpstreamError{Message: fmt.Sprintf("video round %d/%d missing post id", roundIndex, totalRounds), StatusCode: http.StatusBadGateway, Body: errType}
	}
	if !finalRound {
		return nil
	}
	if result.VideoURL != "" {
		return nil
	}
	errType := "empty_video_stream"
	if len(result.StreamErrors) > 0 {
		errType = "moderated_or_stream_errors"
	} else if result.SawVideoEvent {
		errType = "missing_video_url"
	}
	return &reverse.UpstreamError{Message: fmt.Sprintf("video round %d/%d missing final video url", roundIndex, totalRounds), StatusCode: http.StatusBadGateway, Body: errType}
}

type postIDCandidate struct {
	Rank  int
	Value string
}

func extractPostIDCandidates(resp map[string]any) []postIDCandidate {
	var candidates []postIDCandidate
	if modelResp, _ := resp["modelResponse"].(map[string]any); len(modelResp) > 0 {
		if attachments, _ := modelResp["fileAttachments"].([]any); len(attachments) > 0 {
			if value := pickString(attachments[0]); value != "" {
				candidates = append(candidates, postIDCandidate{Rank: 1, Value: value})
			}
		}
	}
	if videoResp, _ := resp["streamingVideoGenerationResponse"].(map[string]any); len(videoResp) > 0 {
		if value := pickString(videoResp["videoPostId"]); value != "" {
			candidates = append(candidates, postIDCandidate{Rank: 2, Value: value})
		}
		if value := pickString(videoResp["postId"]); value != "" {
			candidates = append(candidates, postIDCandidate{Rank: 3, Value: value})
		}
	}
	if post, _ := resp["post"].(map[string]any); len(post) > 0 {
		if value := pickString(post["id"]); value != "" {
			candidates = append(candidates, postIDCandidate{Rank: 4, Value: value})
		}
	}
	for _, key := range []string{"postId", "post_id", "parentPostId", "originalPostId"} {
		if value := pickString(resp[key]); value != "" {
			candidates = append(candidates, postIDCandidate{Rank: 5, Value: value})
		}
	}
	return candidates
}

func applyPostIDCandidates(result *VideoRoundResult, candidates []postIDCandidate) {
	for _, candidate := range candidates {
		if candidate.Rank < result.PostIDRank {
			result.PostIDRank = candidate.Rank
			result.PostID = candidate.Value
		}
	}
}

func appendUniqueErrors(bucket *[]string, raw any) {
	switch typed := raw.(type) {
	case nil:
		return
	case []any:
		for _, item := range typed {
			appendUniqueErrors(bucket, item)
		}
	default:
		text := strings.TrimSpace(fmt.Sprint(typed))
		if text == "" || text == "<nil>" {
			return
		}
		for _, existing := range *bucket {
			if existing == text {
				return
			}
		}
		*bucket = append(*bucket, text)
	}
}

func trimJSONLine(line []byte) []byte {
	return []byte(strings.TrimSpace(string(line)))
}

func pickString(value any) string {
	if text, ok := value.(string); ok {
		return strings.TrimSpace(text)
	}
	return ""
}

func extractPostIDFromVideoURL(videoURL string) string {
	match := postIDURLPattern.FindStringSubmatch(strings.TrimSpace(videoURL))
	if len(match) >= 2 {
		return strings.TrimSpace(match[1])
	}
	return ""
}

func defaultString(value, fallback string) string {
	if strings.TrimSpace(value) == "" {
		return fallback
	}
	return strings.TrimSpace(value)
}

type videoSSEBuilder struct {
	model      string
	showThink  bool
	created    int64
	responseID string
	roleSent   bool
	thinkOpen  bool
	chunks     []string
}

func newVideoSSEBuilder(model string, showThink bool) *videoSSEBuilder {
	return &videoSSEBuilder{
		model:      model,
		showThink:  showThink,
		created:    time.Now().Unix(),
		responseID: "chatcmpl-" + strings.ReplaceAll(uuid.NewString(), "-", "")[:24],
	}
}

func (b *videoSSEBuilder) Append(chunks ...string) {
	b.chunks = append(b.chunks, chunks...)
}

func (b *videoSSEBuilder) EmitProgress(roundIndex, totalRounds int, progress any) []string {
	if !b.showThink {
		return nil
	}
	chunks := b.ensureRole()
	if !b.thinkOpen {
		b.thinkOpen = true
		chunks = append(chunks, b.sseChunk("<think>\n", "", ""))
	}
	chunks = append(chunks, b.sseChunk(fmt.Sprintf("[round=%d/%d] progress=%s%%\n", roundIndex, totalRounds, formatProgress(progress)), "", ""))
	return chunks
}

func (b *videoSSEBuilder) EmitNote(text string) []string {
	if !b.showThink {
		return nil
	}
	chunks := b.ensureRole()
	if !b.thinkOpen {
		b.thinkOpen = true
		chunks = append(chunks, b.sseChunk("<think>\n", "", ""))
	}
	chunks = append(chunks, b.sseChunk(text, "", ""))
	return chunks
}

func (b *videoSSEBuilder) EmitContent(text string) []string {
	chunks := b.ensureRole()
	if b.thinkOpen {
		b.thinkOpen = false
		chunks = append(chunks, b.sseChunk("\n</think>\n", "", ""))
	}
	if text != "" {
		chunks = append(chunks, b.sseChunk(text, "", ""))
	}
	return chunks
}

func (b *videoSSEBuilder) Finish() []string {
	chunks := b.ensureRole()
	if b.thinkOpen {
		b.thinkOpen = false
		chunks = append(chunks, b.sseChunk("\n</think>\n", "", ""))
	}
	chunks = append(chunks, b.sseChunk("", "", "stop"), "data: [DONE]\n\n")
	return chunks
}

func (b *videoSSEBuilder) ensureRole() []string {
	if b.roleSent {
		return nil
	}
	b.roleSent = true
	return []string{b.sseChunk("", "assistant", "")}
}

func (b *videoSSEBuilder) sseChunk(content, role, finish string) string {
	delta := map[string]any{}
	if role != "" {
		delta["role"] = role
		delta["content"] = ""
	} else if content != "" {
		delta["content"] = content
	}
	chunk := map[string]any{
		"id":      b.responseID,
		"object":  "chat.completion.chunk",
		"created": b.created,
		"model":   b.model,
		"choices": []map[string]any{{
			"index":         0,
			"delta":         delta,
			"logprobs":      nil,
			"finish_reason": emptyToNilString(finish),
		}},
	}
	data, _ := json.Marshal(chunk)
	return "data: " + string(data) + "\n\n"
}

func formatProgress(value any) string {
	switch typed := value.(type) {
	case nil:
		return "0"
	case float64:
		if typed == math.Trunc(typed) {
			return fmt.Sprintf("%.0f", typed)
		}
		return strings.TrimRight(strings.TrimRight(fmt.Sprintf("%.2f", typed), "0"), ".")
	case float32:
		value := float64(typed)
		if value == math.Trunc(value) {
			return fmt.Sprintf("%.0f", value)
		}
		return strings.TrimRight(strings.TrimRight(fmt.Sprintf("%.2f", value), "0"), ".")
	case int, int8, int16, int32, int64, uint, uint8, uint16, uint32, uint64:
		return fmt.Sprint(typed)
	case string:
		if strings.TrimSpace(typed) != "" {
			return strings.TrimSpace(typed)
		}
	}
	return fmt.Sprint(value)
}

func emptyToNilString(value string) any {
	if value == "" {
		return nil
	}
	return value
}
