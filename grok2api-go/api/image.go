package api

import (
	"encoding/json"
	"fmt"
	"net/http"
	"sort"
	"strings"
	"time"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/model"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/service"
	"github.com/fran0220/grok2api-go/internal/token"
)

var (
	allowedImageSizes  = map[string]string{"1280x720": "16:9", "720x1280": "9:16", "1792x1024": "3:2", "1024x1792": "2:3", "1024x1024": "1:1"}
	allowedAspectRatio = map[string]bool{"1:1": true, "2:3": true, "3:2": true, "9:16": true, "16:9": true}
)

type imageGenerationRequest struct {
	Prompt         string `json:"prompt"`
	Model          string `json:"model"`
	N              int    `json:"n"`
	Size           string `json:"size"`
	Quality        string `json:"quality"`
	ResponseFormat string `json:"response_format"`
	Style          string `json:"style"`
	Stream         bool   `json:"stream"`
	AspectRatio    string `json:"aspect_ratio"`
}

func handleImageGenerations(cfg *config.Config, models *model.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		defer r.Body.Close()
		var req imageGenerationRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			writeAPIError(w, http.StatusBadRequest, "invalid_request", "Invalid request body")
			return
		}
		if req.Model == "" {
			req.Model = "grok-imagine-1.0"
		}
		if req.N == 0 {
			req.N = 1
		}
		if req.Size == "" {
			req.Size = "1024x1024"
		}
		if req.ResponseFormat == "" {
			req.ResponseFormat = resolveImageResponseFormat(cfg, "")
		}
		if req.ResponseFormat == "base64" {
			req.ResponseFormat = "b64_json"
		}
		if err := validateImageRequest(req, models); err != nil {
			writeAPIError(w, http.StatusBadRequest, "invalid_request", err.Error())
			return
		}
		modelInfo, _ := models.Get(req.Model)
		tokenMgr := token.GetInstance()
		tokenMgr.ReloadIfStale()
		seedToken := pickSeedToken(tokenMgr, models, req.Model)
		if seedToken == "" {
			writeAPIError(w, http.StatusTooManyRequests, "rate_limit_exceeded", "No available tokens. Please try again later.")
			return
		}
		aspectRatio := resolveAspectRatio(req.Size, req.AspectRatio)
		imageService := service.NewImageGenerationService(cfg)
		result, err := imageService.Generate(r.Context(), tokenMgr, seedToken, modelInfo, req.Prompt, req.N, req.ResponseFormat, req.Size, aspectRatio, req.Stream, nil)
		if err != nil {
			status, code, message := normalizeAPIError(err)
			writeAPIError(w, status, code, message)
			return
		}
		if result.Stream {
			streamSSE(w, r, result.StreamData)
			return
		}
		field := responseField(req.ResponseFormat)
		data := make([]map[string]string, 0, len(result.Data))
		for _, item := range result.Data {
			data = append(data, map[string]string{field: item})
		}
		writeJSON(w, http.StatusOK, map[string]any{"created": time.Now().Unix(), "data": data, "usage": result.Usage})
	}
}

func validateImageRequest(req imageGenerationRequest, models *model.Service) error {
	if strings.TrimSpace(req.Prompt) == "" {
		return fmt.Errorf("prompt cannot be empty")
	}
	if req.N < 1 || req.N > 10 {
		return fmt.Errorf("n must be between 1 and 10")
	}
	if req.Stream && req.N != 1 && req.N != 2 {
		return fmt.Errorf("streaming is only supported when n=1 or n=2")
	}
	if _, ok := models.Get(req.Model); !ok {
		return fmt.Errorf("the model `%s` is not supported", req.Model)
	}
	modelInfo, _ := models.Get(req.Model)
	if !modelInfo.IsImage {
		return fmt.Errorf("the model `%s` is not supported for image generation", req.Model)
	}
	if _, ok := allowedImageSizes[req.Size]; !ok {
		keys := make([]string, 0, len(allowedImageSizes))
		for key := range allowedImageSizes {
			keys = append(keys, key)
		}
		sort.Strings(keys)
		return fmt.Errorf("size must be one of %v", keys)
	}
	if req.ResponseFormat != "" && req.ResponseFormat != "url" && req.ResponseFormat != "b64_json" && req.ResponseFormat != "base64" {
		return fmt.Errorf("response_format must be one of [b64_json base64 url]")
	}
	if req.AspectRatio != "" && !allowedAspectRatio[strings.TrimSpace(req.AspectRatio)] {
		return fmt.Errorf("aspect_ratio must be one of [1:1 2:3 3:2 9:16 16:9]")
	}
	return nil
}

func resolveImageResponseFormat(cfg *config.Config, incoming string) string {
	value := strings.TrimSpace(strings.ToLower(incoming))
	if value == "" {
		value = strings.TrimSpace(strings.ToLower(cfg.GetString("app.image_format", "url")))
	}
	if value == "url" || value == "b64_json" || value == "base64" {
		return value
	}
	return "url"
}

func resolveAspectRatio(size, explicit string) string {
	if allowedAspectRatio[strings.TrimSpace(explicit)] {
		return strings.TrimSpace(explicit)
	}
	if ratio, ok := allowedImageSizes[strings.TrimSpace(size)]; ok {
		return ratio
	}
	return "2:3"
}

func responseField(format string) string {
	if format == "url" {
		return "url"
	}
	if format == "base64" {
		return "base64"
	}
	return "b64_json"
}

func pickSeedToken(tokenMgr *token.TokenManager, models *model.Service, modelID string) string {
	for _, poolName := range models.PoolCandidatesForModel(modelID) {
		if tokenValue := tokenMgr.GetToken(poolName, nil, nil); tokenValue != "" {
			return tokenValue
		}
	}
	return ""
}

func streamSSE(w http.ResponseWriter, r *http.Request, stream <-chan string) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		writeAPIError(w, http.StatusInternalServerError, "stream_not_supported", "streaming is not supported")
		return
	}
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	w.WriteHeader(http.StatusOK)
	for {
		select {
		case <-r.Context().Done():
			return
		case chunk, ok := <-stream:
			if !ok {
				return
			}
			_, _ = w.Write([]byte(chunk))
			flusher.Flush()
		}
	}
}

func handleImageEdits(cfg *config.Config, models *model.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		writeJSON(w, http.StatusNotImplemented, map[string]any{
			"error": map[string]string{
				"message": "image edits not yet implemented",
				"type":    "not_implemented",
				"code":    "not_implemented",
			},
		})
	}
}

func writeAPIError(w http.ResponseWriter, status int, code, message string) {
	writeJSON(w, status, map[string]any{"error": map[string]any{"message": message, "type": "server_error", "code": code}})
}

func normalizeAPIError(err error) (int, string, string) {
	message := err.Error()
	status := http.StatusInternalServerError
	code := "server_error"
	if strings.Contains(strings.ToLower(message), "no available tokens") {
		return http.StatusTooManyRequests, "rate_limit_exceeded", message
	}
	if typed, ok := err.(*reverse.UpstreamError); ok {
		if typed.StatusCode > 0 {
			status = typed.StatusCode
		}
		if status == http.StatusTooManyRequests {
			code = "rate_limit_exceeded"
		}
		if status == http.StatusBadGateway {
			code = "bad_gateway"
		}
		if typed.Message != "" {
			message = typed.Message
		}
	}
	return status, code, message
}
