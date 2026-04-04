package api

import (
	"encoding/json"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/model"
	"github.com/fran0220/grok2api-go/internal/service"
	"github.com/fran0220/grok2api-go/internal/token"
	"github.com/fran0220/grok2api-go/pkg/sse"
)

var validRoles = map[string]struct{}{
	"developer": {},
	"system":    {},
	"user":      {},
	"assistant": {},
	"tool":      {},
}

var chatVideoRatioMap = map[string]string{
	"1280x720":  "16:9",
	"720x1280":  "9:16",
	"1792x1024": "3:2",
	"1024x1792": "2:3",
	"1024x1024": "1:1",
	"16:9":      "16:9",
	"9:16":      "9:16",
	"3:2":       "3:2",
	"2:3":       "2:3",
	"1:1":       "1:1",
}

type ChatCompletionRequest struct {
	Model             string            `json:"model"`
	Messages          []service.Message `json:"messages"`
	Stream            *bool             `json:"stream,omitempty"`
	ReasoningEffort   string            `json:"reasoning_effort,omitempty"`
	Temperature       *float64          `json:"temperature,omitempty"`
	TopP              *float64          `json:"top_p,omitempty"`
	VideoConfig       map[string]any    `json:"video_config,omitempty"`
	ImageConfig       map[string]any    `json:"image_config,omitempty"`
	Tools             []service.Tool    `json:"tools,omitempty"`
	ToolChoice        any               `json:"tool_choice,omitempty"`
	ParallelToolCalls *bool             `json:"parallel_tool_calls,omitempty"`
}

type chatVideoConfig struct {
	AspectRatio string
	VideoLength int
	Resolution  string
	Preset      string
}

func handleChatCompletions(cfg *config.Config, modelService *model.Service) http.HandlerFunc {
	chatService := service.NewChatService(cfg, modelService)
	videoService := service.NewVideoService(cfg, modelService)
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}

		var request ChatCompletionRequest
		decoder := json.NewDecoder(r.Body)
		if err := decoder.Decode(&request); err != nil {
			writeChatError(w, false, service.NewValidationError("Invalid JSON body", "invalid_json"))
			return
		}

		stream := cfg.GetBool("app.stream", true)
		if request.Stream != nil {
			stream = *request.Stream
		}

		if err := validateChatRequest(&request, modelService); err != nil {
			writeChatError(w, stream, err)
			return
		}

		modelInfo, _ := modelService.Get(request.Model)
		if modelInfo.IsVideo {
			videoCfg, err := normalizeChatVideoConfig(request.VideoConfig)
			if err != nil {
				writeChatError(w, stream, service.NewValidationError(err.Error(), "invalid_video_config"))
				return
			}
			result, err := videoService.Completions(r.Context(), service.CompletionParams{
				Model:           request.Model,
				Messages:        messageMaps(request.Messages),
				Stream:          stream,
				ReasoningEffort: request.ReasoningEffort,
				AspectRatio:     videoCfg.AspectRatio,
				VideoLength:     videoCfg.VideoLength,
				Resolution:      videoCfg.Resolution,
				Preset:          videoCfg.Preset,
			})
			if err != nil {
				writeChatError(w, stream, err)
				return
			}
			if stream {
				if err := writeRawSSEChunks(w, result.StreamChunks); err != nil {
					return
				}
				return
			}
			writeJSON(w, http.StatusOK, result.Response)
			return
		}
		if modelInfo.IsImageEdit {
			prompt, fileAttachments, imageAttachments := service.ExtractMessages(request.Messages, request.Tools)
			images := append([]string{}, imageAttachments...)
			images = append(images, fileAttachments...)
			if len(images) == 0 {
				writeChatError(w, stream, service.NewValidationError("image input is required for image edit", "invalid_image_input"))
				return
			}
			if strings.TrimSpace(prompt) == "" {
				prompt = "Edit the provided image."
			}
			tokenMgr := token.GetInstance()
			tokenMgr.ReloadIfStale()
			seedToken := pickSeedToken(tokenMgr, modelService, request.Model)
			if seedToken == "" {
				writeChatError(w, stream, service.NewStatusError(http.StatusTooManyRequests, "No available tokens. Please try again later.", "rate_limit_error", "rate_limit_exceeded", nil))
				return
			}
			imageEditService := service.NewImageEditService(cfg)
			result, err := imageEditService.Edit(
				r.Context(),
				tokenMgr,
				seedToken,
				modelInfo,
				prompt,
				images,
				1,
				resolveImageResponseFormat(cfg, ""),
				stream,
				true,
			)
			if err != nil {
				writeChatError(w, stream, err)
				return
			}
			if stream {
				streamSSE(w, r, result.StreamData)
				return
			}
			content := strings.Join(result.Data, "\n")
			writeJSON(w, http.StatusOK, map[string]any{
				"id":      "chatcmpl-image-edit",
				"object":  "chat.completion",
				"created": 0,
				"model":   request.Model,
				"choices": []map[string]any{{
					"index": 0,
					"message": map[string]any{
						"role":    "assistant",
						"content": content,
						"refusal": nil,
					},
					"finish_reason": "stop",
				}},
				"usage": result.Usage,
			})
			return
		}
		if modelInfo.IsImage {
			prompt, _, imageAttachments := service.ExtractMessages(request.Messages, request.Tools)
			if strings.TrimSpace(prompt) == "" && len(imageAttachments) == 0 {
				writeChatError(w, stream, service.NewValidationError("prompt cannot be empty", "empty_prompt"))
				return
			}
			tokenMgr := token.GetInstance()
			tokenMgr.ReloadIfStale()
			seedToken := pickSeedToken(tokenMgr, modelService, request.Model)
			if seedToken == "" {
				writeChatError(w, stream, service.NewStatusError(http.StatusTooManyRequests, "No available tokens. Please try again later.", "rate_limit_error", "rate_limit_exceeded", nil))
				return
			}
			imageN := 1
			imageSize := "1024x1024"
			imageRespFmt := resolveImageResponseFormat(cfg, "")
			if request.ImageConfig != nil {
				if n, ok := request.ImageConfig["n"]; ok {
					if nf, ok := n.(float64); ok {
						imageN = int(nf)
					}
				}
				if s, ok := request.ImageConfig["size"].(string); ok && s != "" {
					imageSize = s
				}
				if rf, ok := request.ImageConfig["response_format"].(string); ok && rf != "" {
					imageRespFmt = resolveImageResponseFormat(cfg, rf)
				}
			}
			imageService := service.NewImageGenerationService(cfg)
			aspectRatio := imageSizeToAspectRatio(imageSize)
			result, err := imageService.Generate(
				r.Context(), tokenMgr, seedToken, modelInfo,
				prompt, imageN, imageRespFmt, imageSize, aspectRatio, stream, nil,
			)
			if err != nil {
				writeChatError(w, stream, err)
				return
			}
			if stream {
				streamSSE(w, r, result.StreamData)
				return
			}
			content := strings.Join(result.Data, "\n")
			writeJSON(w, http.StatusOK, map[string]any{
				"id":      "chatcmpl-image",
				"object":  "chat.completion",
				"created": time.Now().Unix(),
				"model":   request.Model,
				"choices": []map[string]any{{
					"index": 0,
					"message": map[string]any{
						"role":    "assistant",
						"content": content,
					},
					"finish_reason": "stop",
				}},
				"usage": result.Usage,
			})
			return
		}

		temperature := 0.8
		if request.Temperature != nil {
			temperature = *request.Temperature
		}
		topP := 0.95
		if request.TopP != nil {
			topP = *request.TopP
		}
		parallelToolCalls := true
		if request.ParallelToolCalls != nil {
			parallelToolCalls = *request.ParallelToolCalls
		}

		result, err := chatService.Completions(r.Context(), request.Model, request.Messages, service.ChatOptions{
			Stream:            stream,
			ReasoningEffort:   request.ReasoningEffort,
			Temperature:       temperature,
			TopP:              topP,
			Tools:             request.Tools,
			ToolChoice:        request.ToolChoice,
			ParallelToolCalls: parallelToolCalls,
		})
		if err != nil {
			writeChatError(w, stream, err)
			return
		}

		if stream {
			writer := sse.NewWriter(w)
			writer.SetHeaders()
			for chunk := range result.Stream {
				if chunk.Err != nil {
					_ = writer.WriteError(chunk.Err)
					_ = writer.WriteDone()
					return
				}
				if chunk.Done {
					_ = writer.WriteDone()
					return
				}
				if err := writer.WriteEvent(chunk.Data); err != nil {
					return
				}
			}
			return
		}

		writeJSON(w, http.StatusOK, result.Response)
	}
}

func validateChatRequest(request *ChatCompletionRequest, modelService *model.Service) error {
	if request == nil {
		return service.NewValidationError("Request body is required", "invalid_request")
	}
	if strings.TrimSpace(request.Model) == "" {
		return service.NewValidationError("model is required", "invalid_model")
	}
	if _, ok := modelService.Get(request.Model); !ok {
		return service.NewValidationError("invalid model", "invalid_model")
	}
	if len(request.Messages) == 0 {
		return service.NewValidationError("messages must not be empty", "invalid_messages")
	}
	for idx, message := range request.Messages {
		if _, ok := validRoles[strings.TrimSpace(message.Role)]; !ok {
			return service.NewValidationError("invalid role at messages["+strconv.Itoa(idx)+"]", "invalid_role")
		}
	}
	if request.Temperature != nil && (*request.Temperature < 0 || *request.Temperature > 2) {
		return service.NewValidationError("temperature must be between 0 and 2", "invalid_temperature")
	}
	if request.TopP != nil && (*request.TopP < 0 || *request.TopP > 1) {
		return service.NewValidationError("top_p must be between 0 and 1", "invalid_top_p")
	}
	if !service.IsValidReasoningEffort(strings.TrimSpace(request.ReasoningEffort)) {
		return service.NewValidationError("reasoning_effort must be one of none/minimal/low/medium/high/xhigh", "invalid_reasoning_effort")
	}
	return nil
}

func normalizeChatVideoConfig(raw map[string]any) (chatVideoConfig, error) {
	cfg := chatVideoConfig{
		AspectRatio: "3:2",
		VideoLength: 6,
		Resolution:  "480p",
		Preset:      "custom",
	}
	if len(raw) == 0 {
		return cfg, nil
	}
	if value := strings.TrimSpace(anyString(raw["aspect_ratio"])); value != "" {
		mapped, ok := chatVideoRatioMap[value]
		if !ok {
			return cfg, errString("aspect_ratio must be one of 1280x720, 720x1280, 1792x1024, 1024x1792, 1024x1024, 16:9, 9:16, 3:2, 2:3, 1:1")
		}
		cfg.AspectRatio = mapped
	}
	if value, ok := anyInt(raw["video_length"]); ok {
		if value < 6 || value > 30 {
			return cfg, errString("video_length must be between 6 and 30 seconds")
		}
		cfg.VideoLength = value
	}
	if value := strings.TrimSpace(anyString(raw["resolution_name"])); value != "" {
		if value != "480p" && value != "720p" {
			return cfg, errString("resolution_name must be one of 480p, 720p")
		}
		cfg.Resolution = value
	}
	if value := strings.ToLower(strings.TrimSpace(anyString(raw["preset"]))); value != "" {
		if value != "fun" && value != "normal" && value != "spicy" && value != "custom" {
			return cfg, errString("preset must be one of fun, normal, spicy, custom")
		}
		cfg.Preset = value
	}
	return cfg, nil
}

func messageMaps(messages []service.Message) []map[string]any {
	result := make([]map[string]any, 0, len(messages))
	for _, message := range messages {
		result = append(result, map[string]any{
			"role":         message.Role,
			"content":      message.Content,
			"tool_calls":   message.ToolCalls,
			"tool_call_id": message.ToolCallID,
			"name":         message.Name,
		})
	}
	return result
}

func writeRawSSEChunks(w http.ResponseWriter, chunks []string) error {
	writer := sse.NewWriter(w)
	writer.SetHeaders()
	for _, chunk := range chunks {
		if _, err := w.Write([]byte(chunk)); err != nil {
			return err
		}
		if flusher, ok := w.(http.Flusher); ok {
			flusher.Flush()
		}
	}
	return nil
}

func writeChatError(w http.ResponseWriter, stream bool, err error) {
	if stream {
		writer := sse.NewWriter(w)
		writer.SetHeaders()
		_ = writer.WriteError(err)
		_ = writer.WriteDone()
		return
	}
	serviceErr := service.AsChatError(err)
	status := http.StatusInternalServerError
	payload := map[string]any{
		"error": map[string]any{
			"message": errMessage(err),
			"type":    "server_error",
			"code":    "internal_error",
		},
	}
	if serviceErr != nil {
		status = serviceErr.StatusCode
		payload = serviceErr.SSEErrorPayload()
	}
	writeJSON(w, status, payload)
}

func errMessage(err error) string {
	if err == nil {
		return "internal_error"
	}
	return err.Error()
}

func anyString(value any) string {
	if raw, ok := value.(string); ok {
		return raw
	}
	return ""
}

func anyInt(value any) (int, bool) {
	switch typed := value.(type) {
	case int:
		return typed, true
	case int8:
		return int(typed), true
	case int16:
		return int(typed), true
	case int32:
		return int(typed), true
	case int64:
		return int(typed), true
	case float64:
		return int(typed), true
	default:
		return 0, false
	}
}

func imageSizeToAspectRatio(size string) string {
	switch size {
	case "1280x720":
		return "16:9"
	case "720x1280":
		return "9:16"
	case "1792x1024":
		return "7:4"
	case "1024x1792":
		return "4:7"
	default:
		return "1:1"
	}
}

type errString string

func (e errString) Error() string { return string(e) }
