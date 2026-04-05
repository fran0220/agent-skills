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
	Prompt  string `json:"prompt"`
	Model   string `json:"model"`
	Size    string `json:"size"`
	Seconds int    `json:"seconds"`
	Quality string `json:"quality"`
	Image   string `json:"image,omitempty"`
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

		result, err := videoService.Generate(r.Context(), service.GenerateParams{
			Model:       modelID,
			Prompt:      req.Prompt,
			Image:       strings.TrimSpace(req.Image),
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
