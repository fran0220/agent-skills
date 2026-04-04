package functionapi

import (
	"context"
	"crypto/subtle"
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"strings"
	"time"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/model"
)

type Handler struct {
	cfg    *config.Config
	logger *slog.Logger
	models *model.Service
}

func NewHandler(cfg *config.Config, logger *slog.Logger, models *model.Service) *Handler {
	return &Handler{cfg: cfg, logger: logger, models: models}
}

func writeJSON(w http.ResponseWriter, status int, payload any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(payload)
}

func methodNotAllowed(w http.ResponseWriter, allowed ...string) {
	if len(allowed) > 0 {
		w.Header().Set("Allow", strings.Join(allowed, ", "))
	}
	writeJSON(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
}

func unauthorized(w http.ResponseWriter, message string) {
	w.Header().Set("WWW-Authenticate", "Bearer")
	writeJSON(w, http.StatusUnauthorized, map[string]string{"error": message})
}

func badRequest(w http.ResponseWriter, message string) {
	writeJSON(w, http.StatusBadRequest, map[string]string{"error": message})
}

func (h *Handler) authorizeFunctionRequest(r *http.Request) bool {
	functionKey := strings.TrimSpace(h.cfg.GetString("app.function_key"))
	if functionKey == "" {
		return h.cfg.GetBool("app.function_enabled")
	}
	if token, ok := bearerToken(r); ok && secureCompare(token, functionKey) {
		return true
	}
	queryKey := strings.TrimSpace(r.URL.Query().Get("function_key"))
	return queryKey != "" && secureCompare(queryKey, functionKey)
}

func bearerToken(r *http.Request) (string, bool) {
	header := strings.TrimSpace(r.Header.Get("Authorization"))
	if header == "" {
		return "", false
	}
	parts := strings.SplitN(header, " ", 2)
	if len(parts) != 2 || !strings.EqualFold(parts[0], "Bearer") {
		return "", false
	}
	token := strings.TrimSpace(parts[1])
	return token, token != ""
}

func secureCompare(left, right string) bool {
	return subtle.ConstantTimeCompare([]byte(left), []byte(right)) == 1
}

func normalizeImagineAspectRatio(value string) string {
	switch strings.TrimSpace(value) {
	case "1:1", "2:3", "3:2", "9:16", "16:9":
		return strings.TrimSpace(value)
	default:
		return "2:3"
	}
}

func normalizeVideoAspectRatio(value string) string {
	switch strings.TrimSpace(value) {
	case "1280x720", "16:9":
		return "16:9"
	case "720x1280", "9:16":
		return "9:16"
	case "1792x1024", "3:2":
		return "3:2"
	case "1024x1792", "2:3":
		return "2:3"
	case "1024x1024", "1:1":
		return "1:1"
	default:
		return ""
	}
}

func stringsTrim(value string) string {
	return strings.TrimSpace(value)
}

func stringsReplaceAll(value, old, new string) string {
	return strings.ReplaceAll(value, old, new)
}

func stringsSplitLines(value string) []string {
	return strings.Split(value, "\n")
}

func stringsHasPrefix(value, prefix string) bool {
	return strings.HasPrefix(value, prefix)
}

func stringsJoin(values []string, sep string) string {
	return strings.Join(values, sep)
}

func anyString(value any) string {
	switch typed := value.(type) {
	case string:
		return typed
	case fmt.Stringer:
		return typed.String()
	default:
		return ""
	}
}

func sleepContext(ctx context.Context, delay time.Duration) bool {
	if delay <= 0 {
		return ctx == nil || ctx.Err() == nil
	}
	timer := time.NewTimer(delay)
	defer timer.Stop()
	select {
	case <-ctx.Done():
		return false
	case <-timer.C:
		return true
	}
}
