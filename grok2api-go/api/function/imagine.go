package functionapi

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"sync"
	"time"

	"github.com/fran0220/grok2api-go/internal/model"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/service"
	"github.com/fran0220/grok2api-go/internal/token"
	"github.com/google/uuid"
	"github.com/gorilla/websocket"
)

const imagineSessionTTL = 10 * time.Minute

type imagineSession struct {
	Prompt      string
	AspectRatio string
	NSFW        *bool
	CreatedAt   time.Time
}

var imagineSessions = struct {
	sync.Mutex
	items map[string]imagineSession
}{items: map[string]imagineSession{}}

type imagineStartRequest struct {
	Prompt      string `json:"prompt"`
	AspectRatio string `json:"aspect_ratio"`
	NSFW        *bool  `json:"nsfw"`
}

type imagineStopRequest struct {
	TaskIDs []string `json:"task_ids"`
}

func (h *Handler) ImagineConfig() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w, http.MethodGet)
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{
			"final_min_bytes":  h.cfg.GetInt("image.final_min_bytes", 0),
			"medium_min_bytes": h.cfg.GetInt("image.medium_min_bytes", 0),
			"nsfw":             h.cfg.GetBool("image.nsfw", true),
		})
	})
}

func (h *Handler) ImagineStart() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		defer r.Body.Close()
		var req imagineStartRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			badRequest(w, "invalid JSON body")
			return
		}
		prompt := stringsTrim(req.Prompt)
		if prompt == "" {
			badRequest(w, "Prompt cannot be empty")
			return
		}
		ratio := normalizeImagineAspectRatio(req.AspectRatio)
		taskID := newImagineSession(imagineSession{Prompt: prompt, AspectRatio: ratio, NSFW: req.NSFW, CreatedAt: time.Now()})
		writeJSON(w, http.StatusOK, map[string]any{"task_id": taskID, "aspect_ratio": ratio})
	})
}

func (h *Handler) ImagineStop() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		defer r.Body.Close()
		var req imagineStopRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			badRequest(w, "invalid JSON body")
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{"status": "success", "removed": dropImagineSessions(req.TaskIDs)})
	})
}

func (h *Handler) ImagineSSE() http.Handler {
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
		session, ok := getImagineSession(taskID)
		if !ok {
			writeJSON(w, http.StatusNotFound, map[string]string{"error": "Task not found"})
			return
		}
		defer dropImagineSession(taskID)

		flusher, ok := w.(http.Flusher)
		if !ok {
			writeJSON(w, http.StatusInternalServerError, map[string]string{"error": "streaming is not supported"})
			return
		}
		w.Header().Set("Content-Type", "text/event-stream")
		w.Header().Set("Cache-Control", "no-cache")
		w.Header().Set("Connection", "keep-alive")
		w.Header().Set("X-Accel-Buffering", "no")

		writePayload := func(payload map[string]any) bool {
			data, err := json.Marshal(payload)
			if err != nil {
				return false
			}
			if _, err := w.Write([]byte("data: " + string(data) + "\n\n")); err != nil {
				return false
			}
			flusher.Flush()
			return true
		}

		modelInfo, ok := h.models.Get("grok-imagine-1.0")
		if !ok || !modelInfo.IsImage {
			_ = writePayload(map[string]any{"type": "error", "message": "Image model is not available.", "code": "model_not_supported"})
			return
		}

		runID := uuid.NewString()
		_ = writePayload(map[string]any{"type": "status", "status": "running", "prompt": session.Prompt, "aspect_ratio": session.AspectRatio, "run_id": runID})
		imageService := service.NewImageGenerationService(h.cfg)
		tokenMgr := token.GetInstance()

		for r.Context().Err() == nil {
			if _, alive := getImagineSession(taskID); !alive {
				break
			}
			tokenMgr.ReloadIfStale()
			seedToken := pickImagineToken(tokenMgr, h.models, modelInfo, session.NSFW)
			if seedToken == "" {
				if !writePayload(map[string]any{"type": "error", "message": "No available tokens. Please try again later.", "code": "rate_limit_exceeded"}) {
					return
				}
				if !sleepContext(r.Context(), 2*time.Second) {
					return
				}
				continue
			}
			result, err := imageService.Generate(r.Context(), tokenMgr, seedToken, modelInfo, session.Prompt, 6, "b64_json", "1024x1024", session.AspectRatio, true, session.NSFW)
			if err != nil {
				status, code, message := normalizeImagineError(err)
				if !writePayload(map[string]any{"type": "error", "message": message, "code": code, "status": status}) {
					return
				}
				if !sleepContext(r.Context(), 1500*time.Millisecond) {
					return
				}
				continue
			}
			if result.Stream {
				for chunk := range result.StreamData {
					payload := parseImagineChunk(chunk)
					if payload == nil {
						continue
					}
					payload["run_id"] = runID
					if !writePayload(payload) {
						return
					}
					if r.Context().Err() != nil {
						return
					}
				}
			}
		}
		_ = writePayload(map[string]any{"type": "status", "status": "stopped", "run_id": runID})
	})
}

func (h *Handler) ImagineWS() http.Handler {
	upgrader := websocket.Upgrader{CheckOrigin: func(r *http.Request) bool { return true }}
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !h.authorizeFunctionRequest(r) {
			unauthorized(w, "Invalid authentication token")
			return
		}
		conn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			return
		}
		defer conn.Close()

		taskID := stringsTrim(r.URL.Query().Get("task_id"))
		storedSession, _ := getImagineSession(taskID)
		var writeMu sync.Mutex
		send := func(payload map[string]any) bool {
			data, err := json.Marshal(payload)
			if err != nil {
				return false
			}
			writeMu.Lock()
			defer writeMu.Unlock()
			return conn.WriteMessage(websocket.TextMessage, data) == nil
		}

		var runCancel context.CancelFunc
		var runDone chan struct{}
		stopRun := func() {
			if runCancel != nil {
				runCancel()
				<-runDone
				runCancel = nil
				runDone = nil
			}
		}
		defer stopRun()

		for {
			_, message, err := conn.ReadMessage()
			if err != nil {
				break
			}
			var payload map[string]any
			if err := json.Unmarshal(message, &payload); err != nil {
				if !send(map[string]any{"type": "error", "message": "Invalid message format.", "code": "invalid_payload"}) {
					break
				}
				continue
			}
			action, _ := payload["type"].(string)
			switch action {
			case "start":
				stopRun()
				prompt := stringsTrim(anyString(payload["prompt"]))
				if prompt == "" {
					prompt = storedSession.Prompt
				}
				if prompt == "" {
					if !send(map[string]any{"type": "error", "message": "Prompt cannot be empty.", "code": "invalid_prompt"}) {
						return
					}
					continue
				}
				ratio := normalizeImagineAspectRatio(anyString(payload["aspect_ratio"]))
				if ratio == "2:3" && storedSession.AspectRatio != "" && stringsTrim(anyString(payload["aspect_ratio"])) == "" {
					ratio = storedSession.AspectRatio
				}
				nsfw := storedSession.NSFW
				if value, ok := payload["nsfw"].(bool); ok {
					nsfw = &value
				}
				var runCtx context.Context
				runCtx, runCancel = context.WithCancel(r.Context())
				runDone = make(chan struct{})
				go func() {
					defer close(runDone)
					h.runImagineLoop(runCtx, prompt, ratio, nsfw, taskID, send)
				}()
			case "stop":
				stopRun()
				if !send(map[string]any{"type": "status", "status": "stopped"}) {
					return
				}
			default:
				if !send(map[string]any{"type": "error", "message": "Unknown action.", "code": "invalid_action"}) {
					return
				}
			}
		}
		if taskID != "" {
			dropImagineSession(taskID)
		}
	})
}

func (h *Handler) runImagineLoop(ctx context.Context, prompt, aspectRatio string, nsfw *bool, taskID string, send func(map[string]any) bool) {
	modelInfo, ok := h.models.Get("grok-imagine-1.0")
	if !ok || !modelInfo.IsImage {
		send(map[string]any{"type": "error", "message": "Image model is not available.", "code": "model_not_supported"})
		return
	}
	imageService := service.NewImageGenerationService(h.cfg)
	tokenMgr := token.GetInstance()
	runID := uuid.NewString()
	if !send(map[string]any{"type": "status", "status": "running", "prompt": prompt, "aspect_ratio": aspectRatio, "run_id": runID}) {
		return
	}

	for ctx.Err() == nil {
		if taskID != "" {
			if _, ok := getImagineSession(taskID); !ok {
				break
			}
		}
		tokenMgr.ReloadIfStale()
		seedToken := pickImagineToken(tokenMgr, h.models, modelInfo, nsfw)
		if seedToken == "" {
			if !send(map[string]any{"type": "error", "message": "No available tokens. Please try again later.", "code": "rate_limit_exceeded"}) {
				return
			}
			if !sleepContext(ctx, 2*time.Second) {
				return
			}
			continue
		}
		result, err := imageService.Generate(ctx, tokenMgr, seedToken, modelInfo, prompt, 6, "b64_json", "1024x1024", aspectRatio, true, nsfw)
		if err != nil {
			_, code, message := normalizeImagineError(err)
			if !send(map[string]any{"type": "error", "message": message, "code": code}) {
				return
			}
			if !sleepContext(ctx, 1500*time.Millisecond) {
				return
			}
			continue
		}
		if result.Stream {
			for chunk := range result.StreamData {
				payload := parseImagineChunk(chunk)
				if payload == nil {
					continue
				}
				payload["run_id"] = runID
				if !send(payload) {
					return
				}
			}
		}
	}
	send(map[string]any{"type": "status", "status": "stopped", "run_id": runID})
}

func pickImagineToken(manager *token.TokenManager, models *model.Service, modelInfo model.ModelInfo, nsfw *bool) string {
	preferTags := map[string]bool{}
	if nsfw != nil && *nsfw {
		preferTags["nsfw"] = true
	}
	for _, poolName := range models.PoolCandidatesForModel(modelInfo.ModelID) {
		if tokenValue := manager.GetToken(poolName, nil, preferTags); tokenValue != "" {
			return tokenValue
		}
	}
	return ""
}

func parseImagineChunk(chunk string) map[string]any {
	if chunk == "" {
		return nil
	}
	var event string
	dataLines := make([]string, 0, 2)
	for _, raw := range stringsSplitLines(chunk) {
		line := stringsTrim(raw)
		if line == "" {
			continue
		}
		if stringsHasPrefix(line, "event:") {
			event = stringsTrim(line[len("event:"):])
			continue
		}
		if stringsHasPrefix(line, "data:") {
			dataLines = append(dataLines, stringsTrim(line[len("data:"):]))
		}
	}
	if len(dataLines) == 0 {
		return nil
	}
	data := stringsJoin(dataLines, "\n")
	if data == "[DONE]" {
		return nil
	}
	payload := map[string]any{}
	if err := json.Unmarshal([]byte(data), &payload); err != nil {
		return nil
	}
	if _, ok := payload["type"]; !ok && event != "" {
		payload["type"] = event
	}
	return payload
}

func normalizeImagineError(err error) (int, string, string) {
	status := http.StatusBadGateway
	code := "internal_error"
	message := err.Error()
	var upstreamErr *reverse.UpstreamError
	if errors.As(err, &upstreamErr) {
		if upstreamErr.StatusCode > 0 {
			status = upstreamErr.StatusCode
		}
		if status == http.StatusTooManyRequests {
			code = "rate_limit_exceeded"
		}
	}
	return status, code, message
}

func newImagineSession(session imagineSession) string {
	trimExpiredImagineSessions(time.Now())
	taskID := stringsReplaceAll(uuid.NewString(), "-", "")
	imagineSessions.Lock()
	imagineSessions.items[taskID] = session
	imagineSessions.Unlock()
	return taskID
}

func getImagineSession(taskID string) (imagineSession, bool) {
	trimExpiredImagineSessions(time.Now())
	imagineSessions.Lock()
	defer imagineSessions.Unlock()
	session, ok := imagineSessions.items[taskID]
	return session, ok
}

func dropImagineSession(taskID string) {
	imagineSessions.Lock()
	delete(imagineSessions.items, taskID)
	imagineSessions.Unlock()
}

func dropImagineSessions(taskIDs []string) int {
	removed := 0
	imagineSessions.Lock()
	for _, taskID := range taskIDs {
		if _, ok := imagineSessions.items[taskID]; ok {
			delete(imagineSessions.items, taskID)
			removed++
		}
	}
	imagineSessions.Unlock()
	return removed
}

func trimExpiredImagineSessions(now time.Time) {
	imagineSessions.Lock()
	for taskID, session := range imagineSessions.items {
		if now.Sub(session.CreatedAt) > imagineSessionTTL {
			delete(imagineSessions.items, taskID)
		}
	}
	imagineSessions.Unlock()
}
