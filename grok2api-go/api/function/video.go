package functionapi

import (
	"encoding/json"
	"net/http"
	"sync"
	"time"

	"github.com/fran0220/grok2api-go/internal/service"
	"github.com/google/uuid"
)

const videoSessionTTL = 10 * time.Minute

type videoSession struct {
	Prompt          string
	AspectRatio     string
	VideoLength     int
	ResolutionName  string
	Preset          string
	ImageURLs       []string
	ReasoningEffort string
	CreatedAt       time.Time
}

var videoSessions = struct {
	sync.Mutex
	items map[string]videoSession
}{items: map[string]videoSession{}}

type videoStartRequest struct {
	Prompt          string   `json:"prompt"`
	AspectRatio     string   `json:"aspect_ratio"`
	VideoLength     int      `json:"video_length"`
	ResolutionName  string   `json:"resolution_name"`
	Preset          string   `json:"preset"`
	ImageURLs       []string `json:"image_urls"`
	ReasoningEffort string   `json:"reasoning_effort"`
}

type videoStopRequest struct {
	TaskIDs []string `json:"task_ids"`
}

func (h *Handler) VideoStart() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		defer r.Body.Close()
		var req videoStartRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			badRequest(w, "invalid JSON body")
			return
		}
		prompt := stringsTrim(req.Prompt)
		if prompt == "" {
			badRequest(w, "Prompt cannot be empty")
			return
		}
		aspectRatio := normalizeVideoAspectRatio(req.AspectRatio)
		if aspectRatio == "" {
			badRequest(w, "aspect_ratio must be one of ['16:9','9:16','3:2','2:3','1:1']")
			return
		}
		videoLength := req.VideoLength
		if videoLength == 0 {
			videoLength = 6
		}
		if videoLength < 6 || videoLength > 30 {
			badRequest(w, "video_length must be between 6 and 30 seconds")
			return
		}
		resolution := defaultVideoResolution(req.ResolutionName)
		if resolution != "480p" && resolution != "720p" {
			badRequest(w, "resolution_name must be one of ['480p','720p']")
			return
		}
		preset := defaultVideoPreset(req.Preset)
		if preset != "fun" && preset != "normal" && preset != "spicy" && preset != "custom" {
			badRequest(w, "preset must be one of ['fun','normal','spicy','custom']")
			return
		}
		if len(req.ImageURLs) > 7 {
			badRequest(w, "image_urls supports at most 7 references")
			return
		}
		taskID := newVideoSession(videoSession{
			Prompt:          prompt,
			AspectRatio:     aspectRatio,
			VideoLength:     videoLength,
			ResolutionName:  resolution,
			Preset:          preset,
			ImageURLs:       append([]string(nil), req.ImageURLs...),
			ReasoningEffort: stringsTrim(req.ReasoningEffort),
			CreatedAt:       time.Now(),
		})
		writeJSON(w, http.StatusOK, map[string]any{"task_id": taskID, "aspect_ratio": aspectRatio})
	})
}

func (h *Handler) VideoStop() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		defer r.Body.Close()
		var req videoStopRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			badRequest(w, "invalid JSON body")
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{"status": "success", "removed": dropVideoSessions(req.TaskIDs)})
	})
}

func (h *Handler) VideoSSE() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w, http.MethodGet)
			return
		}
		if !h.authorizeFunctionRequest(r) {
			unauthorized(w, "Invalid authentication token")
			return
		}
		taskID := stringsTrim(r.URL.Query().Get("task_id"))
		session, ok := getVideoSession(taskID)
		if !ok {
			writeJSON(w, http.StatusNotFound, map[string]string{"error": "Task not found"})
			return
		}
		defer dropVideoSession(taskID)

		flusher, ok := w.(http.Flusher)
		if !ok {
			writeJSON(w, http.StatusInternalServerError, map[string]string{"error": "streaming is not supported"})
			return
		}
		w.Header().Set("Content-Type", "text/event-stream")
		w.Header().Set("Cache-Control", "no-cache")
		w.Header().Set("Connection", "keep-alive")
		w.Header().Set("X-Accel-Buffering", "no")

		videoService := service.NewVideoService(h.cfg, h.models)
		result, err := videoService.Completions(r.Context(), service.CompletionParams{
			Model:           "grok-imagine-1.0-video",
			Messages:        buildVideoMessages(session.Prompt, session.ImageURLs),
			Stream:          true,
			ReasoningEffort: session.ReasoningEffort,
			AspectRatio:     session.AspectRatio,
			VideoLength:     session.VideoLength,
			Resolution:      session.ResolutionName,
			Preset:          session.Preset,
		})
		if err != nil {
			payload, _ := json.Marshal(map[string]any{"error": err.Error(), "code": "internal_error"})
			_, _ = w.Write([]byte("data: " + string(payload) + "\n\n"))
			_, _ = w.Write([]byte("data: [DONE]\n\n"))
			flusher.Flush()
			return
		}
		for _, chunk := range result.StreamChunks {
			if _, err := w.Write([]byte(chunk)); err != nil {
				return
			}
			flusher.Flush()
		}
	})
}

func buildVideoMessages(prompt string, imageURLs []string) []map[string]any {
	if len(imageURLs) == 0 {
		return []map[string]any{{"role": "user", "content": prompt}}
	}
	content := []map[string]any{{"type": "text", "text": prompt}}
	for _, imageURL := range imageURLs {
		content = append(content, map[string]any{"type": "image_url", "image_url": map[string]any{"url": imageURL}})
	}
	return []map[string]any{{"role": "user", "content": content}}
}

func defaultVideoResolution(value string) string {
	trimmed := stringsTrim(value)
	if trimmed == "" {
		return "480p"
	}
	return trimmed
}

func defaultVideoPreset(value string) string {
	trimmed := stringsTrim(value)
	if trimmed == "" {
		return "normal"
	}
	return trimmed
}

func newVideoSession(session videoSession) string {
	trimExpiredVideoSessions(time.Now())
	taskID := stringsReplaceAll(uuid.NewString(), "-", "")
	videoSessions.Lock()
	videoSessions.items[taskID] = session
	videoSessions.Unlock()
	return taskID
}

func getVideoSession(taskID string) (videoSession, bool) {
	trimExpiredVideoSessions(time.Now())
	videoSessions.Lock()
	defer videoSessions.Unlock()
	session, ok := videoSessions.items[taskID]
	return session, ok
}

func dropVideoSession(taskID string) {
	videoSessions.Lock()
	delete(videoSessions.items, taskID)
	videoSessions.Unlock()
}

func dropVideoSessions(taskIDs []string) int {
	removed := 0
	videoSessions.Lock()
	for _, taskID := range taskIDs {
		if _, ok := videoSessions.items[taskID]; ok {
			delete(videoSessions.items, taskID)
			removed++
		}
	}
	videoSessions.Unlock()
	return removed
}

func trimExpiredVideoSessions(now time.Time) {
	videoSessions.Lock()
	for taskID, session := range videoSessions.items {
		if now.Sub(session.CreatedAt) > videoSessionTTL {
			delete(videoSessions.items, taskID)
		}
	}
	videoSessions.Unlock()
}
