package service

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"strings"
	"time"

	"github.com/google/uuid"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/model"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/token"
)

var validReasoningEfforts = map[string]struct{}{
	"":        {},
	"none":    {},
	"minimal": {},
	"low":     {},
	"medium":  {},
	"high":    {},
	"xhigh":   {},
}

// Message is the OpenAI-compatible chat message payload.
type Message struct {
	Role       string     `json:"role"`
	Content    any        `json:"content"`
	ToolCalls  []ToolCall `json:"tool_calls,omitempty"`
	ToolCallID string     `json:"tool_call_id,omitempty"`
	Name       string     `json:"name,omitempty"`
}

// ChatOptions controls chat completion behavior.
type ChatOptions struct {
	Stream            bool
	ReasoningEffort   string
	Temperature       float64
	TopP              float64
	Tools             []Tool
	ToolChoice        any
	ParallelToolCalls bool
}

// ChatCompletionResult returns either a streaming channel or a JSON payload.
type ChatCompletionResult struct {
	Stream   <-chan StreamChunk
	Response map[string]any
}

// ChatError represents an OpenAI-compatible API failure.
type ChatError struct {
	StatusCode int
	Type       string
	Code       string
	Param      string
	Message    string
	Cause      error
}

func (e *ChatError) Error() string {
	if e == nil {
		return "<nil>"
	}
	if e.Message != "" {
		return e.Message
	}
	if e.Cause != nil {
		return e.Cause.Error()
	}
	return "service_error"
}

func (e *ChatError) Unwrap() error {
	if e == nil {
		return nil
	}
	return e.Cause
}

func (e *ChatError) SSEErrorPayload() map[string]any {
	return map[string]any{
		"error": map[string]any{
			"message": e.Error(),
			"type":    e.TypeOr("server_error"),
			"code":    e.CodeOr("stream_error"),
		},
	}
}

func (e *ChatError) TypeOr(fallback string) string {
	if e == nil || e.Type == "" {
		return fallback
	}
	return e.Type
}

func (e *ChatError) CodeOr(fallback string) string {
	if e == nil || e.Code == "" {
		return fallback
	}
	return e.Code
}

func NewValidationError(message, code string) error {
	return &ChatError{
		StatusCode: http.StatusBadRequest,
		Type:       "invalid_request_error",
		Code:       code,
		Message:    message,
	}
}

func NewStatusError(statusCode int, message, errorType, code string, cause error) error {
	return &ChatError{
		StatusCode: statusCode,
		Type:       errorType,
		Code:       code,
		Message:    message,
		Cause:      cause,
	}
}

func AsChatError(err error) *ChatError {
	var svcErr *ChatError
	if errors.As(err, &svcErr) {
		return svcErr
	}
	if err == nil {
		return nil
	}
	return &ChatError{
		StatusCode: http.StatusInternalServerError,
		Type:       "server_error",
		Code:       "internal_error",
		Message:    err.Error(),
		Cause:      err,
	}
}

func IsValidReasoningEffort(value string) bool {
	_, ok := validReasoningEfforts[value]
	return ok
}

type extractedMessage struct {
	Role string
	Text string
}

// ExtractMessages converts OpenAI chat messages into a single Grok prompt.
func ExtractMessages(messages []Message, tools []Tool) (string, []string, []string) {
	if len(tools) > 0 {
		messages = FormatToolHistory(messages)
	}
	textParts := make([]string, 0, len(messages))
	fileAttachments := make([]string, 0)
	imageAttachments := make([]string, 0)
	extracted := make([]extractedMessage, 0, len(messages))

	for _, msg := range messages {
		role := strings.TrimSpace(msg.Role)
		if role == "" {
			role = "user"
		}
		parts := extractContentParts(msg.Content, &fileAttachments, &imageAttachments)
		if role == "assistant" && len(parts) == 0 && len(msg.ToolCalls) > 0 {
			for _, call := range msg.ToolCalls {
				name := "tool"
				arguments := ""
				if function, ok := call["function"].(map[string]any); ok {
					if rawName, ok := function["name"].(string); ok && strings.TrimSpace(rawName) != "" {
						name = strings.TrimSpace(rawName)
					}
					arguments = stringify(function["arguments"])
				} else {
					if rawName, ok := call["name"].(string); ok && strings.TrimSpace(rawName) != "" {
						name = strings.TrimSpace(rawName)
					}
					arguments = stringify(call["arguments"])
				}
				parts = append(parts, strings.TrimSpace("[tool_call] "+name+" "+arguments))
			}
		}
		if len(parts) == 0 {
			continue
		}
		roleLabel := role
		if role == "tool" {
			if name := strings.TrimSpace(msg.Name); name != "" {
				roleLabel = fmt.Sprintf("tool[%s]", name)
			}
			if callID := strings.TrimSpace(msg.ToolCallID); callID != "" {
				roleLabel = fmt.Sprintf("%s#%s", roleLabel, callID)
			}
		}
		extracted = append(extracted, extractedMessage{Role: roleLabel, Text: strings.Join(parts, "\n")})
	}

	lastUserIndex := -1
	for i := len(extracted) - 1; i >= 0; i-- {
		if extracted[i].Role == "user" {
			lastUserIndex = i
			break
		}
	}

	for i, item := range extracted {
		if i == lastUserIndex {
			textParts = append(textParts, item.Text)
			continue
		}
		textParts = append(textParts, fmt.Sprintf("%s: %s", item.Role, item.Text))
	}

	combined := strings.Join(textParts, "\n\n")
	if strings.TrimSpace(combined) == "" && (len(fileAttachments) > 0 || len(imageAttachments) > 0) {
		combined = "Refer to the following content:"
	}
	return combined, fileAttachments, imageAttachments
}

func extractContentParts(content any, fileAttachments, imageAttachments *[]string) []string {
	parts := make([]string, 0)
	switch typed := content.(type) {
	case string:
		if text := strings.TrimSpace(typed); text != "" {
			parts = append(parts, text)
		}
	case map[string]any:
		parts = append(parts, extractBlocks([]any{typed}, fileAttachments, imageAttachments)...)
	case []any:
		parts = append(parts, extractBlocks(typed, fileAttachments, imageAttachments)...)
	case []map[string]any:
		items := make([]any, 0, len(typed))
		for _, item := range typed {
			items = append(items, item)
		}
		parts = append(parts, extractBlocks(items, fileAttachments, imageAttachments)...)
	}
	return parts
}

func extractBlocks(items []any, fileAttachments, imageAttachments *[]string) []string {
	parts := make([]string, 0)
	for _, item := range items {
		block, ok := item.(map[string]any)
		if !ok {
			continue
		}
		switch strings.TrimSpace(stringValue(block["type"])) {
		case "text":
			if text := strings.TrimSpace(stringValue(block["text"])); text != "" {
				parts = append(parts, text)
			}
		case "image_url":
			if image, ok := block["image_url"].(map[string]any); ok {
				if url := strings.TrimSpace(stringValue(image["url"])); url != "" {
					*imageAttachments = append(*imageAttachments, url)
				}
			} else if url := strings.TrimSpace(stringValue(block["image_url"])); url != "" {
				*imageAttachments = append(*imageAttachments, url)
			}
		case "input_audio":
			if audio, ok := block["input_audio"].(map[string]any); ok {
				if data := strings.TrimSpace(stringValue(audio["data"])); data != "" {
					*fileAttachments = append(*fileAttachments, data)
				}
			}
		case "file":
			if fileData, ok := block["file"].(map[string]any); ok {
				if raw := strings.TrimSpace(stringValue(fileData["file_data"])); raw != "" {
					*fileAttachments = append(*fileAttachments, raw)
				}
			}
		}
	}
	return parts
}

func stringify(value any) string {
	switch typed := value.(type) {
	case nil:
		return ""
	case string:
		return strings.TrimSpace(typed)
	default:
		payload, err := json.Marshal(typed)
		if err != nil {
			return strings.TrimSpace(fmt.Sprint(typed))
		}
		return string(payload)
	}
}

func stringValue(value any) string {
	if raw, ok := value.(string); ok {
		return raw
	}
	return ""
}

// GrokChatService wraps the reverse app-chat request construction.
type GrokChatService struct {
	models  *model.Service
	reverse *reverse.AppChatReverse
}

func NewGrokChatService(models *model.Service, tokenMgr *token.TokenManager) *GrokChatService {
	rev := reverse.NewAppChatReverse()
	rev.RecordTokenFailure = func(ctx context.Context, tokenValue string, status int, reason string) error {
		if tokenMgr == nil {
			return nil
		}
		return tokenMgr.RecordFail(tokenValue, status, reason)
	}
	return &GrokChatService{models: models, reverse: rev}
}

func (s *GrokChatService) ChatOpenAI(ctx context.Context, session *reverse.ResettableSession, tokenValue, modelID string, messages []Message, opts ChatOptions) (io.ReadCloser, int, error) {
	if s == nil || s.models == nil {
		return nil, 0, NewStatusError(http.StatusInternalServerError, "model service unavailable", "server_error", "internal_error", nil)
	}
	modelInfo, ok := s.models.Get(modelID)
	if !ok {
		return nil, 0, NewValidationError("invalid model", "invalid_model")
	}
	promptText, fileAttachments, _ := ExtractMessages(messages, opts.Tools)
	if toolPrompt := BuildToolPrompt(opts.Tools, opts.ToolChoice, opts.ParallelToolCalls); strings.TrimSpace(toolPrompt) != "" {
		if strings.TrimSpace(promptText) == "" {
			promptText = toolPrompt
		} else {
			promptText = toolPrompt + "\n\n" + promptText
		}
	}
	promptTokens := EstimatePromptTokens(promptText)
	body, err := s.reverse.Request(
		ctx,
		session,
		tokenValue,
		promptText,
		modelInfo.GrokModel,
		modelInfo.ModelMode,
		fileAttachments,
		map[string]any{},
		map[string]any{},
		map[string]any{},
	)
	if err != nil {
		return nil, promptTokens, err
	}
	return body, promptTokens, nil
}

// ChatService orchestrates model lookup, token selection, retries, and parsing.
type ChatService struct {
	cfg      *config.Config
	models   *model.Service
	tokenMgr *token.TokenManager
	grok     *GrokChatService
}

func NewChatService(cfg *config.Config, models *model.Service) *ChatService {
	tokenMgr := token.GetInstance()
	return &ChatService{
		cfg:      cfg,
		models:   models,
		tokenMgr: tokenMgr,
		grok:     NewGrokChatService(models, tokenMgr),
	}
}

func (s *ChatService) Completions(ctx context.Context, modelID string, messages []Message, opts ChatOptions) (*ChatCompletionResult, error) {
	maxRetry := 3
	if s.cfg != nil {
		maxRetry = s.cfg.GetInt("retry.max_retry", 3)
	}
	if maxRetry < 0 {
		maxRetry = 0
	}

	exclude := make(map[string]bool)
	var lastErr error
	for attempt := 0; attempt <= maxRetry; attempt++ {
		tokenValue := s.pickToken(modelID, exclude)
		if tokenValue == "" {
			if lastErr != nil {
				return nil, normalizeChatError(lastErr)
			}
			return nil, NewStatusError(
				http.StatusTooManyRequests,
				"No available tokens. Please try again later.",
				"rate_limit_error",
				"rate_limit_exceeded",
				nil,
			)
		}

		session := reverse.NewResettableSession(reverse.SessionOptions{})
		body, promptTokens, err := s.grok.ChatOpenAI(ctx, session, tokenValue, modelID, messages, opts)
		if err != nil {
			session.Close()
			exclude[tokenValue] = true
			if s.handleRetryableError(tokenValue, err) {
				lastErr = err
				continue
			}
			return nil, normalizeChatError(err)
		}

		if opts.Stream {
			processor := NewStreamProcessor(s.cfg, modelID, promptTokens, opts.Tools)
			stream := processor.Process(body, session)
			return &ChatCompletionResult{Stream: WrapStreamWithUsage(stream, s.tokenMgr, tokenValue, modelID)}, nil
		}

		processor := NewCollectProcessor(s.cfg, modelID, promptTokens, opts.Tools)
		response, collectErr := processor.Process(body, session)
		if collectErr != nil {
			exclude[tokenValue] = true
			if s.handleRetryableError(tokenValue, collectErr) {
				lastErr = collectErr
				continue
			}
			return nil, normalizeChatError(collectErr)
		}
		if err := s.tokenMgr.Consume(tokenValue, effortForModelID(modelID)); err != nil {
			slog.Warn("record non-stream usage failed", "model", modelID, "error", err)
		}
		return &ChatCompletionResult{Response: response}, nil
	}

	if lastErr != nil {
		return nil, normalizeChatError(lastErr)
	}
	return nil, NewStatusError(http.StatusTooManyRequests, "No available tokens. Please try again later.", "rate_limit_error", "rate_limit_exceeded", nil)
}

func (s *ChatService) pickToken(modelID string, exclude map[string]bool) string {
	if s.tokenMgr == nil || s.models == nil {
		return ""
	}
	s.tokenMgr.ReloadIfStale()
	for _, poolName := range s.models.PoolCandidatesForModel(modelID) {
		if tokenValue := s.tokenMgr.GetToken(poolName, exclude, nil); tokenValue != "" {
			return tokenValue
		}
	}
	s.tokenMgr.RefreshCoolingTokensOnDemand("chat_completion")
	for _, poolName := range s.models.PoolCandidatesForModel(modelID) {
		if tokenValue := s.tokenMgr.GetToken(poolName, exclude, nil); tokenValue != "" {
			return tokenValue
		}
	}
	return ""
}

func (s *ChatService) handleRetryableError(tokenValue string, err error) bool {
	status := reverse.ExtractStatusForRetry(err)
	if status == nil {
		return false
	}
	if *status == http.StatusTooManyRequests {
		if markErr := s.tokenMgr.MarkRateLimited(tokenValue); markErr != nil {
			slog.Warn("mark token rate limited failed", "error", markErr)
		}
		return true
	}
	if *status == http.StatusBadGateway {
		return true
	}
	return false
}

func normalizeChatError(err error) error {
	if err == nil {
		return nil
	}
	if svcErr := AsChatError(err); svcErr != nil && (svcErr.StatusCode != http.StatusInternalServerError || svcErr.Type != "server_error") {
		return svcErr
	}
	var upstreamErr *reverse.UpstreamError
	if errors.As(err, &upstreamErr) {
		switch upstreamErr.StatusCode {
		case http.StatusUnauthorized:
			return NewStatusError(http.StatusUnauthorized, "Upstream authentication failed", "auth_error", "upstream_auth_failed", err)
		case http.StatusTooManyRequests:
			return NewStatusError(http.StatusTooManyRequests, "Upstream rate limit exceeded", "rate_limit_error", "rate_limit_exceeded", err)
		default:
			return NewStatusError(http.StatusBadGateway, fmt.Sprintf("Upstream request failed: %d", upstreamErr.StatusCode), "upstream_error", "upstream_error", err)
		}
	}
	if status := reverse.ExtractStatusForRetry(err); status != nil && *status == http.StatusBadGateway {
		return NewStatusError(http.StatusBadGateway, "Upstream connection failed", "upstream_error", "bad_gateway", err)
	}
	return AsChatError(err)
}

type grokEnvelope struct {
	Result struct {
		Response grokResponse `json:"response"`
	} `json:"result"`
}

type grokResponse struct {
	Token                            *string                 `json:"token"`
	IsThinking                       bool                    `json:"isThinking"`
	ModelResponse                    json.RawMessage         `json:"modelResponse"`
	ResponseID                       string                  `json:"responseId"`
	RolloutID                        any                     `json:"rolloutId"`
	LLMInfo                          *grokLLMInfo            `json:"llmInfo"`
	StreamingImageGenerationResponse *imageProgressResponse  `json:"streamingImageGenerationResponse"`
	CardAttachment                   *cardAttachmentResponse `json:"cardAttachment"`
}

type grokLLMInfo struct {
	ModelHash string `json:"modelHash"`
}

type imageProgressResponse struct {
	ImageIndex int `json:"imageIndex"`
	Progress   int `json:"progress"`
}

type cardAttachmentResponse struct {
	JSONData string `json:"jsonData"`
}

type processorCallbacks struct {
	EmitRole     func() error
	EmitContent  func(string) error
	EmitToolCall func(ToolCall, int) error
	EmitFinish   func(map[string]any) error
}

type processorState struct {
	model           string
	created         int64
	id              string
	responseID      string
	fingerprint     string
	promptTokens    int
	showThink       bool
	filterTags      []string
	roleSent        bool
	thinkOpened     bool
	thinkClosedOnce bool
	imageThink      bool
	tools           []Tool
	toolCalls       []ToolCall
	toolMode        bool
	toolRemainder   string
	toolBuffer      strings.Builder
	content         strings.Builder
	callbacks       processorCallbacks
}

func newProcessorState(cfg *config.Config, modelID string, promptTokens int, tools []Tool) *processorState {
	showThink := true
	filterTags := []string{"xaiartifact", "xai:tool_usage_card", "grok:render"}
	if cfg != nil {
		showThink = cfg.GetBool("app.thinking", true)
		if len(cfg.App.FilterTags) > 0 {
			filterTags = append([]string(nil), cfg.App.FilterTags...)
		}
	}
	return &processorState{
		model:        modelID,
		created:      time.Now().Unix(),
		id:           "chatcmpl-" + strings.ReplaceAll(uuid.NewString(), "-", ""),
		promptTokens: promptTokens,
		showThink:    showThink,
		filterTags:   filterTags,
		tools:        append([]Tool(nil), tools...),
	}
}

// StreamProcessor converts Grok ndjson into OpenAI SSE chunks.
type StreamProcessor struct {
	state *processorState
}

func NewStreamProcessor(cfg *config.Config, modelID string, promptTokens int, tools []Tool) *StreamProcessor {
	return &StreamProcessor{state: newProcessorState(cfg, modelID, promptTokens, tools)}
}

func (p *StreamProcessor) Process(body io.ReadCloser, session *reverse.ResettableSession) <-chan StreamChunk {
	out := make(chan StreamChunk)
	go func() {
		defer close(out)
		defer closeResources(body, session)
		state := p.state
		state.callbacks = processorCallbacks{
			EmitRole: func() error {
				chunk, err := p.emitRole(state)
				if err == nil {
					out <- StreamChunk{Data: chunk}
				}
				return err
			},
			EmitContent: func(content string) error {
				chunk, err := p.emitContent(state, content)
				if err == nil && chunk != "" {
					out <- StreamChunk{Data: chunk}
				}
				return err
			},
			EmitToolCall: func(toolCall ToolCall, index int) error {
				chunk, err := p.emitToolCall(state, toolCall, index)
				if err == nil && chunk != "" {
					out <- StreamChunk{Data: chunk}
				}
				return err
			},
			EmitFinish: func(usage map[string]any) error {
				chunk, err := p.emitFinish(state, usage)
				if err == nil {
					out <- StreamChunk{Data: chunk}
				}
				return err
			},
		}
		if err := state.process(body); err != nil {
			out <- StreamChunk{Err: normalizeChatError(err)}
			return
		}
		out <- StreamChunk{Done: true}
	}()
	return out
}

func (p *StreamProcessor) emitRole(state *processorState) (string, error) {
	return marshalChunk(state, map[string]any{"role": "assistant", "content": ""}, nil, nil)
}

func (p *StreamProcessor) emitContent(state *processorState, content string) (string, error) {
	if content == "" {
		return "", nil
	}
	return marshalChunk(state, map[string]any{"content": content}, nil, nil)
}

func (p *StreamProcessor) emitToolCall(state *processorState, toolCall ToolCall, index int) (string, error) {
	function, _ := toolCall["function"].(map[string]any)
	delta := map[string]any{
		"tool_calls": []map[string]any{{
			"index": index,
			"id":    toolCall["id"],
			"type":  toolCall["type"],
			"function": map[string]any{
				"name":      function["name"],
				"arguments": function["arguments"],
			},
		}},
	}
	return marshalChunk(state, delta, nil, nil)
}

func (p *StreamProcessor) emitFinish(state *processorState, usage map[string]any) (string, error) {
	finish := state.finishReason()
	return marshalChunk(state, map[string]any{}, &finish, usage)
}

// CollectProcessor accumulates the upstream stream into a single chat response.
type CollectProcessor struct {
	state *processorState
}

func NewCollectProcessor(cfg *config.Config, modelID string, promptTokens int, tools []Tool) *CollectProcessor {
	return &CollectProcessor{state: newProcessorState(cfg, modelID, promptTokens, tools)}
}

func (p *CollectProcessor) Process(body io.ReadCloser, session *reverse.ResettableSession) (map[string]any, error) {
	defer closeResources(body, session)
	if err := p.state.process(body); err != nil {
		return nil, normalizeChatError(err)
	}
	content := p.state.content.String()
	usage := BuildChatUsage(p.state.promptTokens, EstimateCompletionTokens(content, p.state.toolCalls))
	message := map[string]any{"role": "assistant", "content": content}
	if len(p.state.toolCalls) > 0 {
		message["tool_calls"] = p.state.toolCalls
		if strings.TrimSpace(content) == "" {
			message["content"] = nil
		}
	}
	response := map[string]any{
		"id":      p.state.chunkID(),
		"object":  "chat.completion",
		"created": p.state.created,
		"model":   p.state.model,
		"choices": []map[string]any{{
			"index":         0,
			"message":       message,
			"finish_reason": p.state.finishReason(),
		}},
		"usage": usage,
	}
	if p.state.fingerprint != "" {
		response["system_fingerprint"] = p.state.fingerprint
	}
	return response, nil
}

func (p *processorState) process(body io.Reader) error {
	reader := bufio.NewReader(body)
	for {
		line, err := reader.ReadBytes('\n')
		if len(line) > 0 {
			if normalized := normalizeLine(line); normalized != "" {
				var envelope grokEnvelope
				if unmarshalErr := json.Unmarshal([]byte(normalized), &envelope); unmarshalErr == nil {
					if err := p.handleResponse(envelope.Result.Response); err != nil {
						return err
					}
				}
			}
		}
		if err != nil {
			if errors.Is(err, io.EOF) {
				break
			}
			return err
		}
	}
	return p.finalize()
}

func (p *processorState) handleResponse(resp grokResponse) error {
	if resp.LLMInfo != nil && p.fingerprint == "" {
		p.fingerprint = strings.TrimSpace(resp.LLMInfo.ModelHash)
	}
	if strings.TrimSpace(resp.ResponseID) != "" {
		p.responseID = strings.TrimSpace(resp.ResponseID)
	}
	if !p.roleSent && p.callbacks.EmitRole != nil {
		if err := p.callbacks.EmitRole(); err != nil {
			return err
		}
		p.roleSent = true
	}

	if progress := resp.StreamingImageGenerationResponse; progress != nil {
		if !p.showThink {
			return nil
		}
		p.imageThink = true
		if !p.thinkOpened {
			if err := p.emitContent("<think>\n"); err != nil {
				return err
			}
			p.thinkOpened = true
		}
		return p.emitContent(fmt.Sprintf("正在生成第%d张图片中，当前进度%d%%\n", progress.ImageIndex+1, progress.Progress))
	}

	if len(resp.ModelResponse) > 0 && string(resp.ModelResponse) != "null" {
		if p.imageThink && p.thinkOpened {
			if err := p.emitContent("\n</think>\n"); err != nil {
				return err
			}
			p.thinkOpened = false
			p.thinkClosedOnce = true
		}
		p.imageThink = false
		for _, url := range collectImages(resp.ModelResponse) {
			if err := p.emitContent(fmt.Sprintf("![image](%s)\n", url)); err != nil {
				return err
			}
		}
	}

	if card := resp.CardAttachment; card != nil && strings.TrimSpace(card.JSONData) != "" {
		if rendered := parseCardAttachment(card.JSONData); rendered != "" {
			if err := p.emitContent(rendered); err != nil {
				return err
			}
		}
	}

	if resp.Token == nil || *resp.Token == "" {
		return nil
	}
	if resp.IsThinking && p.thinkClosedOnce && !p.imageThink {
		return nil
	}
	filtered := p.filterToken(*resp.Token)
	if filtered == "" {
		return nil
	}
	inThink := (resp.IsThinking && !p.thinkClosedOnce) || p.imageThink
	if inThink {
		if !p.showThink {
			return nil
		}
		if !p.thinkOpened {
			if err := p.emitContent("<think>\n"); err != nil {
				return err
			}
			p.thinkOpened = true
		}
	} else if p.thinkOpened {
		if err := p.emitContent("\n</think>\n"); err != nil {
			return err
		}
		p.thinkOpened = false
		p.thinkClosedOnce = true
	}
	return p.handleModelToken(filtered)
}

func (p *processorState) finalize() error {
	if err := p.flushToolRemainder(true); err != nil {
		return err
	}
	if p.thinkOpened {
		if err := p.emitContent("\n</think>\n"); err != nil {
			return err
		}
		p.thinkOpened = false
	}
	if p.callbacks.EmitFinish == nil {
		return nil
	}
	usage := BuildChatUsage(p.promptTokens, EstimateCompletionTokens(p.content.String(), p.toolCalls))
	return p.callbacks.EmitFinish(usage)
}

func (p *processorState) finishReason() string {
	if len(p.toolCalls) > 0 {
		return "tool_calls"
	}
	return "stop"
}

func (p *processorState) handleModelToken(value string) error {
	if value == "" || len(p.tools) == 0 {
		return p.emitContent(value)
	}
	p.toolRemainder += value
	return p.flushToolRemainder(false)
}

func (p *processorState) flushToolRemainder(final bool) error {
	for {
		if !p.toolMode {
			index := strings.Index(p.toolRemainder, toolCallOpenTag)
			if index >= 0 {
				if err := p.emitContent(p.toolRemainder[:index]); err != nil {
					return err
				}
				p.toolRemainder = p.toolRemainder[index+len(toolCallOpenTag):]
				p.toolMode = true
				p.toolBuffer.Reset()
				continue
			}
			keep := 0
			if !final {
				keep = partialTagSuffix(p.toolRemainder, toolCallOpenTag)
			}
			emitText := p.toolRemainder[:len(p.toolRemainder)-keep]
			p.toolRemainder = p.toolRemainder[len(p.toolRemainder)-keep:]
			return p.emitContent(emitText)
		}

		index := strings.Index(p.toolRemainder, toolCallCloseTag)
		if index >= 0 {
			p.toolBuffer.WriteString(p.toolRemainder[:index])
			raw := p.toolBuffer.String()
			p.toolRemainder = p.toolRemainder[index+len(toolCallCloseTag):]
			p.toolMode = false
			p.toolBuffer.Reset()
			if err := p.emitParsedToolCall(raw); err != nil {
				return err
			}
			continue
		}

		keep := 0
		if !final {
			keep = partialTagSuffix(p.toolRemainder, toolCallCloseTag)
		}
		p.toolBuffer.WriteString(p.toolRemainder[:len(p.toolRemainder)-keep])
		p.toolRemainder = p.toolRemainder[len(p.toolRemainder)-keep:]
		if final {
			raw := p.toolBuffer.String() + p.toolRemainder
			p.toolBuffer.Reset()
			p.toolRemainder = ""
			p.toolMode = false
			return p.emitContent(toolCallOpenTag + raw)
		}
		return nil
	}
}

func (p *processorState) emitParsedToolCall(raw string) error {
	toolCall := ParseToolCallBlock(raw, p.tools)
	if toolCall == nil {
		return p.emitContent(toolCallOpenTag + raw + toolCallCloseTag)
	}
	p.toolCalls = append(p.toolCalls, *toolCall)
	if p.callbacks.EmitToolCall != nil {
		return p.callbacks.EmitToolCall(*toolCall, len(p.toolCalls)-1)
	}
	return nil
}

func (p *processorState) emitContent(content string) error {
	if content == "" {
		return nil
	}
	p.content.WriteString(content)
	if p.callbacks.EmitContent != nil {
		return p.callbacks.EmitContent(content)
	}
	return nil
}

func (p *processorState) filterToken(value string) string {
	for _, tag := range p.filterTags {
		if tag == "" {
			continue
		}
		if strings.Contains(value, "<"+tag) || strings.Contains(value, "</"+tag) {
			return ""
		}
	}
	return value
}

func (p *processorState) chunkID() string {
	if p.responseID != "" {
		return p.responseID
	}
	return p.id
}

func partialTagSuffix(value, tag string) int {
	max := len(value)
	if len(tag)-1 < max {
		max = len(tag) - 1
	}
	for size := max; size > 0; size-- {
		if strings.HasSuffix(value, tag[:size]) {
			return size
		}
	}
	return 0
}

func marshalChunk(state *processorState, delta map[string]any, finish *string, usage map[string]any) (string, error) {
	choice := map[string]any{
		"index":         0,
		"delta":         delta,
		"finish_reason": nil,
	}
	if finish != nil {
		choice["finish_reason"] = *finish
	}
	chunk := map[string]any{
		"id":      state.chunkID(),
		"object":  "chat.completion.chunk",
		"created": state.created,
		"model":   state.model,
		"choices": []map[string]any{choice},
	}
	if state.fingerprint != "" {
		chunk["system_fingerprint"] = state.fingerprint
	}
	if usage != nil {
		chunk["usage"] = usage
	}
	payload, err := json.Marshal(chunk)
	if err != nil {
		return "", err
	}
	return string(payload), nil
}

func normalizeLine(line []byte) string {
	text := strings.TrimSpace(string(line))
	if text == "" {
		return ""
	}
	if strings.HasPrefix(text, "data:") {
		text = strings.TrimSpace(strings.TrimPrefix(text, "data:"))
	}
	if text == "[DONE]" {
		return ""
	}
	return text
}

func collectImages(raw json.RawMessage) []string {
	var value any
	if err := json.Unmarshal(raw, &value); err != nil {
		return nil
	}
	urls := make([]string, 0)
	seen := make(map[string]struct{})
	var walk func(any)
	walk = func(item any) {
		switch typed := item.(type) {
		case map[string]any:
			for key, nested := range typed {
				switch key {
				case "generatedImageUrls", "imageUrls", "imageURLs":
					switch list := nested.(type) {
					case []any:
						for _, entry := range list {
							if url, ok := entry.(string); ok {
								trimmed := strings.TrimSpace(url)
								if trimmed != "" {
									if _, exists := seen[trimmed]; !exists {
										seen[trimmed] = struct{}{}
										urls = append(urls, trimmed)
									}
								}
							}
						}
					case string:
						trimmed := strings.TrimSpace(list)
						if trimmed != "" {
							if _, exists := seen[trimmed]; !exists {
								seen[trimmed] = struct{}{}
								urls = append(urls, trimmed)
							}
						}
					}
				default:
					walk(nested)
				}
			}
		case []any:
			for _, nested := range typed {
				walk(nested)
			}
		}
	}
	walk(value)
	return urls
}

func parseCardAttachment(raw string) string {
	var payload struct {
		Image struct {
			Original string `json:"original"`
			Title    string `json:"title"`
		} `json:"image"`
	}
	if err := json.Unmarshal([]byte(raw), &payload); err != nil {
		return ""
	}
	url := strings.TrimSpace(payload.Image.Original)
	if url == "" {
		return ""
	}
	title := strings.TrimSpace(strings.ReplaceAll(payload.Image.Title, "\n", " "))
	if title == "" {
		title = "image"
	}
	return fmt.Sprintf("![%s](%s)\n", title, url)
}

func closeResources(body io.Closer, session *reverse.ResettableSession) {
	if body != nil {
		_ = body.Close()
	}
	if session != nil {
		session.Close()
	}
}
