package api

import (
	"net/http"

	"chatgpt2api/internal/auth"
	"chatgpt2api/internal/config"
	"chatgpt2api/internal/middleware"
)

func SetupRouter(cfg *config.Config) http.Handler {
	apiKey := cfg.GetString("app.api_key", "")
	appKey := cfg.GetString("app.app_key", "chatgpt2api")

	mux := http.NewServeMux()

	mux.Handle("POST /v1/images/generations", auth.VerifyAPIKey(apiKey)(handleImageGenerations()))
	mux.Handle("POST /v1/images/edits", auth.VerifyAPIKey(apiKey)(handleImageEdits()))
	mux.Handle("GET /v1/models", auth.VerifyAPIKey(apiKey)(handleModels()))

	mux.Handle("/v1/admin/tokens", auth.VerifyAppKey(appKey)(handleTokens()))

	// Static file serving (cached images)
	mux.Handle("GET /v1/files/image/", handleImageFile())

	mux.HandleFunc("GET /health", handleHealth)

	return middleware.CORS(middleware.RequestID(middleware.Logger(mux)))
}

func handleHealth(w http.ResponseWriter, r *http.Request) {
	writeJSON(w, http.StatusOK, map[string]any{"status": "ok"})
}
