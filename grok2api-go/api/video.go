package api

import (
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"sort"
	"strings"
	"time"

	"github.com/google/uuid"

	"github.com/fran0220/grok2api-go/internal/model"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/service"
)

const videoGenerationModelID = "grok-imagine-1.0-video"

var sizeToAspect = map[string]string{
	"1280x720":  "16:9",
	"720x1280":  "9:16",
	"1792x1024": "3:2",
	"1024x1792": "2:3",
	"1024x1024": "1:1",
}

var qualityToResolution = map[string]string{
	"standard": "480p",
	"high":     "720p",
}

type videoGenerationRequest struct {
	Prompt         string `json:"prompt"`
	Model          string `json:"model"`
	Size           string `json:"size"`
	Seconds        int    `json:"seconds"`
	Quality        string `json:"quality"`
	Image          string `json:"image,omitempty"`
	ImageReference any    `json:"image_reference,omitempty"`
}

func handleVideoGenerations(videoService *service.VideoService, modelService *model.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		defer r.Body.Close()

		var req videoGenerationRequest
		decoder := json.NewDecoder(r.Body)
		if err := decoder.Decode(&req); err != nil {
			writeVideoError(w, &service.VideoError{StatusCode: http.StatusBadRequest, Message: "Invalid JSON in request body. Please check for trailing commas or syntax errors.", Param: "body", Code: "json_invalid"})
			return
		}

		modelID, err := normalizeVideoModel(req.Model, modelService)
		if err != nil {
			writeVideoError(w, err)
			return
		}
		size, aspectRatio, err := normalizeVideoSize(req.Size)
		if err != nil {
			writeVideoError(w, err)
			return
		}
		quality, resolution, err := normalizeVideoQuality(req.Quality)
		if err != nil {
			writeVideoError(w, err)
			return
		}
		seconds, err := normalizeVideoSeconds(req.Seconds)
		if err != nil {
			writeVideoError(w, err)
			return
		}
		if strings.TrimSpace(req.Prompt) == "" {
			writeVideoError(w, &service.VideoError{StatusCode: http.StatusBadRequest, Message: "prompt is required", Param: "prompt", Code: "invalid_request_error"})
			return
		}
		imageReferences, err := normalizeVideoImageReferences(req.Image, req.ImageReference)
		if err != nil {
			writeVideoError(w, err)
			return
		}

		result, err := videoService.Completions(r.Context(), service.CompletionParams{
			Model:       modelID,
			Messages:    buildVideoMessages(strings.TrimSpace(req.Prompt), imageReferences),
			AspectRatio: aspectRatio,
			VideoLength: seconds,
			Resolution:  resolution,
			Preset:      "custom",
		})
		if err != nil {
			writeVideoError(w, err)
			return
		}

		now := time.Now().Unix()
		writeJSON(w, http.StatusOK, map[string]any{
			"id":           "video_" + strings.ReplaceAll(uuid.NewString(), "-", "")[:24],
			"object":       "video",
			"created_at":   now,
			"completed_at": now,
			"status":       "completed",
			"model":        modelID,
			"prompt":       strings.TrimSpace(req.Prompt),
			"size":         size,
			"seconds":      fmt.Sprintf("%d", seconds),
			"quality":      quality,
			"url":          result.FinalVideoURL,
		})
	}
}

func normalizeVideoModel(requested string, modelService *model.Service) (string, error) {
	modelID := strings.TrimSpace(requested)
	if modelID == "" {
		modelID = videoGenerationModelID
	}
	if modelID != videoGenerationModelID {
		return "", &service.VideoError{StatusCode: http.StatusBadRequest, Message: fmt.Sprintf("The model `%s` is required for video generation.", videoGenerationModelID), Param: "model", Code: "model_not_supported"}
	}
	modelInfo, ok := modelService.Get(modelID)
	if !ok || !modelInfo.IsVideo {
		return "", &service.VideoError{StatusCode: http.StatusBadRequest, Message: fmt.Sprintf("The model `%s` is not supported for video generation.", modelID), Param: "model", Code: "model_not_supported"}
	}
	return modelID, nil
}

func normalizeVideoSize(size string) (string, string, error) {
	value := strings.TrimSpace(size)
	if value == "" {
		value = "1792x1024"
	}
	aspectRatio, ok := sizeToAspect[value]
	if !ok {
		return "", "", &service.VideoError{StatusCode: http.StatusBadRequest, Message: fmt.Sprintf("size must be one of %v", sortedKeys(sizeToAspect)), Param: "size", Code: "invalid_size"}
	}
	return value, aspectRatio, nil
}

func normalizeVideoQuality(quality string) (string, string, error) {
	value := strings.ToLower(strings.TrimSpace(quality))
	if value == "" {
		value = "standard"
	}
	resolution, ok := qualityToResolution[value]
	if !ok {
		return "", "", &service.VideoError{StatusCode: http.StatusBadRequest, Message: fmt.Sprintf("quality must be one of %v", sortedKeys(qualityToResolution)), Param: "quality", Code: "invalid_quality"}
	}
	return value, resolution, nil
}

func normalizeVideoSeconds(seconds int) (int, error) {
	value := seconds
	if value == 0 {
		value = 6
	}
	if value < 6 || value > 30 {
		return 0, &service.VideoError{StatusCode: http.StatusBadRequest, Message: "seconds must be between 6 and 30", Param: "seconds", Code: "invalid_seconds"}
	}
	return value, nil
}

func normalizeVideoImageReferences(legacyImage string, raw any) ([]string, error) {
	references, err := parseVideoImageReferences(raw)
	if err != nil {
		return nil, err
	}
	if value := strings.TrimSpace(legacyImage); value != "" {
		legacyRef, err := validateVideoImageReference(value, "image")
		if err != nil {
			return nil, err
		}
		references = append(references, legacyRef)
	}
	return dedupeStrings(references), nil
}

func parseVideoImageReferences(raw any) ([]string, error) {
	if raw == nil {
		return nil, nil
	}
	switch typed := raw.(type) {
	case string:
		trimmed := strings.TrimSpace(typed)
		if trimmed == "" {
			return nil, nil
		}
		if strings.HasPrefix(trimmed, "[") {
			var parsed []any
			if err := json.Unmarshal([]byte(trimmed), &parsed); err != nil {
				return nil, &service.VideoError{StatusCode: http.StatusBadRequest, Message: "image_reference must be a JSON array string", Param: "image_reference", Code: "invalid_reference"}
			}
			return parseVideoImageReferences(parsed)
		}
		ref, err := validateVideoImageReference(trimmed, "image_reference")
		if err != nil {
			return nil, err
		}
		return []string{ref}, nil
	case []any:
		refs := make([]string, 0, len(typed))
		for index, item := range typed {
			ref, err := parseVideoImageReferenceItem(item, index)
			if err != nil {
				return nil, err
			}
			if ref != "" {
				refs = append(refs, ref)
			}
		}
		return refs, nil
	default:
		return nil, &service.VideoError{StatusCode: http.StatusBadRequest, Message: "image_reference must be a URL, data URI, or array of image_url blocks", Param: "image_reference", Code: "invalid_reference"}
	}
}

func parseVideoImageReferenceItem(value any, index int) (string, error) {
	param := fmt.Sprintf("image_reference[%d]", index)
	switch typed := value.(type) {
	case string:
		trimmed := strings.TrimSpace(typed)
		if trimmed == "" {
			return "", &service.VideoError{StatusCode: http.StatusBadRequest, Message: param + " cannot be empty", Param: param, Code: "invalid_reference"}
		}
		return validateVideoImageReference(trimmed, param)
	case map[string]any:
		if strings.TrimSpace(anyString(typed["type"])) != "image_url" {
			return "", &service.VideoError{StatusCode: http.StatusBadRequest, Message: param + " must have type=\"image_url\"", Param: param + ".type", Code: "invalid_reference"}
		}
		imageURL, ok := typed["image_url"].(map[string]any)
		if !ok {
			return "", &service.VideoError{StatusCode: http.StatusBadRequest, Message: param + ".image_url must be an object with a url field", Param: param + ".image_url", Code: "invalid_reference"}
		}
		urlValue := strings.TrimSpace(anyString(imageURL["url"]))
		if urlValue == "" {
			return "", &service.VideoError{StatusCode: http.StatusBadRequest, Message: param + ".image_url.url cannot be empty", Param: param + ".image_url.url", Code: "invalid_reference"}
		}
		return validateVideoImageReference(urlValue, param+".image_url.url")
	default:
		return "", &service.VideoError{StatusCode: http.StatusBadRequest, Message: param + " must be a URL string or image_url content block", Param: param, Code: "invalid_reference"}
	}
}

func validateVideoImageReference(value, param string) (string, error) {
	trimmed := strings.TrimSpace(value)
	if strings.HasPrefix(trimmed, "http://") || strings.HasPrefix(trimmed, "https://") || strings.HasPrefix(trimmed, "data:") {
		return trimmed, nil
	}
	return "", &service.VideoError{StatusCode: http.StatusBadRequest, Message: param + " must be a URL or data URI", Param: param, Code: "invalid_reference"}
}

func buildVideoMessages(prompt string, imageReferences []string) []map[string]any {
	if len(imageReferences) == 0 {
		return []map[string]any{{"role": "user", "content": prompt}}
	}
	content := []map[string]any{{"type": "text", "text": prompt}}
	for _, imageReference := range imageReferences {
		content = append(content, map[string]any{"type": "image_url", "image_url": map[string]any{"url": imageReference}})
	}
	return []map[string]any{{"role": "user", "content": content}}
}

func dedupeStrings(values []string) []string {
	if len(values) == 0 {
		return nil
	}
	seen := make(map[string]bool, len(values))
	result := make([]string, 0, len(values))
	for _, value := range values {
		if value == "" || seen[value] {
			continue
		}
		seen[value] = true
		result = append(result, value)
	}
	return result
}

func writeVideoError(w http.ResponseWriter, err error) {
	status := http.StatusBadGateway
	body := map[string]any{
		"error": map[string]any{
			"message": err.Error(),
			"type":    "upstream_error",
			"param":   nil,
			"code":    "upstream_error",
		},
	}

	var serviceErr *service.VideoError
	if errors.As(err, &serviceErr) {
		status = serviceErr.StatusCode
		body["error"] = map[string]any{
			"message": serviceErr.Message,
			"type":    defaultErrorCode(serviceErr.Code, "invalid_request_error"),
			"param":   emptyToNilString(serviceErr.Param),
			"code":    defaultErrorCode(serviceErr.Code, "invalid_request_error"),
		}
		writeJSON(w, status, body)
		return
	}

	var upstreamErr *reverse.UpstreamError
	if errors.As(err, &upstreamErr) {
		status = upstreamErr.StatusCode
		if status == 0 {
			status = http.StatusBadGateway
		}
		code := "upstream_error"
		if status == http.StatusTooManyRequests {
			code = "rate_limit_exceeded"
		}
		body["error"] = map[string]any{
			"message": upstreamErr.Error(),
			"type":    code,
			"param":   nil,
			"code":    code,
		}
	}
	writeJSON(w, status, body)
}

func defaultErrorCode(value, fallback string) string {
	if strings.TrimSpace(value) == "" {
		return fallback
	}
	return value
}

func emptyToNilString(value string) any {
	if strings.TrimSpace(value) == "" {
		return nil
	}
	return value
}

func sortedKeys[T any](items map[string]T) []string {
	keys := make([]string, 0, len(items))
	for key := range items {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	return keys
}
