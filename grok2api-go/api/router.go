package api

import (
	"log/slog"
	"net/http"

	adminapi "github.com/fran0220/grok2api-go/api/admin"
	functionapi "github.com/fran0220/grok2api-go/api/function"
	"github.com/fran0220/grok2api-go/internal/auth"
	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/middleware"
	"github.com/fran0220/grok2api-go/internal/model"
	"github.com/fran0220/grok2api-go/internal/proxy"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/service"
)

func SetupRouter(cfg *config.Config, logger *slog.Logger) http.Handler {
	reverse.SetConfigProvider(cfg)
	proxy.SetConfigProvider(cfg)
	service.ConfigureTokenRefresh(cfg)

	mux := http.NewServeMux()
	modelService := model.NewService()
	videoService := service.NewVideoService(cfg, modelService)
	pageHandler := NewPageHandler(cfg, logger)
	adminHandler := adminapi.NewHandler(cfg)
	functionHandler := functionapi.NewHandler(cfg, logger, modelService)

	mux.Handle("/v1/models", auth.VerifyAPIKey(cfg)(handleModels(modelService)))
	mux.Handle("/v1/chat/completions", auth.VerifyAPIKey(cfg)(handleChatCompletions(cfg, modelService)))
	mux.Handle("/v1/responses", auth.VerifyAPIKey(cfg)(handleResponses(cfg, modelService)))
	mux.Handle("/v1/images/generations", auth.VerifyAPIKey(cfg)(handleImageGenerations(cfg, modelService)))
	mux.Handle("/v1/images/edits", auth.VerifyAPIKey(cfg)(handleImageEdits(cfg, modelService)))
	mux.Handle("/v1/video/generations", auth.VerifyAPIKey(cfg)(handleVideoGenerations(videoService, modelService)))
	mux.Handle("/v1/videos", auth.VerifyAPIKey(cfg)(handleVideoGenerations(videoService, modelService)))
	mux.Handle("/v1/files/image/", handleImageFile())
	mux.Handle("/v1/files/video/", handleVideoFile())
	mux.Handle("/v1/admin/verify", auth.VerifyAppKey(cfg)(adminHandler.Verify()))
	mux.Handle("/v1/admin/storage", auth.VerifyAppKey(cfg)(adminHandler.Storage()))
	mux.Handle("/v1/admin/config", auth.VerifyAppKey(cfg)(adminHandler.Config()))
	mux.Handle("/v1/admin/cache", auth.VerifyAppKey(cfg)(adminHandler.Cache()))
	mux.Handle("/v1/admin/cache/list", auth.VerifyAppKey(cfg)(adminHandler.CacheList()))
	mux.Handle("/v1/admin/cache/clear", auth.VerifyAppKey(cfg)(adminHandler.CacheClear()))
	mux.Handle("/v1/admin/cache/item/delete", auth.VerifyAppKey(cfg)(adminHandler.CacheDeleteItem()))
	mux.Handle("/v1/admin/cache/online/clear", auth.VerifyAppKey(cfg)(adminHandler.CacheOnlineClear()))
	mux.Handle("/v1/admin/cache/online/clear/async", auth.VerifyAppKey(cfg)(adminHandler.CacheOnlineClearAsync()))
	mux.Handle("/v1/admin/cache/online/load/async", auth.VerifyAppKey(cfg)(adminHandler.CacheOnlineLoadAsync()))
	mux.Handle("/v1/admin/batch/", adminHandler.Batch())
	mux.Handle("/v1/admin/tokens", auth.VerifyAppKey(cfg)(adminHandler.Tokens()))
	mux.Handle("/v1/admin/tokens/refresh", auth.VerifyAppKey(cfg)(adminHandler.RefreshTokens()))
	mux.Handle("/v1/admin/tokens/move", auth.VerifyAppKey(cfg)(adminHandler.MoveToken()))
	mux.Handle("/v1/admin/tokens/nsfw/enable", auth.VerifyAppKey(cfg)(adminHandler.NsfwEnable()))
	mux.Handle("/v1/admin/tokens/nsfw/enable/async", auth.VerifyAppKey(cfg)(adminHandler.NsfwEnableAsync()))
	mux.Handle("/v1/function/verify", auth.VerifyFunctionKey(cfg)(pageHandler.FunctionVerify()))
	mux.Handle("/v1/function/chat/completions", auth.VerifyFunctionKey(cfg)(handleChatCompletions(cfg, modelService)))
	mux.Handle("/v1/function/imagine/config", functionHandler.ImagineConfig())
	mux.Handle("/v1/function/imagine/start", auth.VerifyFunctionKey(cfg)(functionHandler.ImagineStart()))
	mux.Handle("/v1/function/imagine/stop", auth.VerifyFunctionKey(cfg)(functionHandler.ImagineStop()))
	mux.Handle("/v1/function/imagine/sse", functionHandler.ImagineSSE())
	mux.Handle("/v1/function/imagine/ws", functionHandler.ImagineWS())
	mux.Handle("/v1/function/video/start", auth.VerifyFunctionKey(cfg)(functionHandler.VideoStart()))
	mux.Handle("/v1/function/video/stop", auth.VerifyFunctionKey(cfg)(functionHandler.VideoStop()))
	mux.Handle("/v1/function/video/sse", functionHandler.VideoSSE())
	mux.Handle("/v1/function/voice/token", auth.VerifyFunctionKey(cfg)(functionHandler.VoiceToken()))
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w, http.MethodGet)
			return
		}
		writeJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	})
	mux.Handle("/static/", pageHandler.Static())
	mux.HandleFunc("/favicon.ico", pageHandler.Favicon())
	mux.HandleFunc("/", pageHandler.Root())
	mux.HandleFunc("/login", pageHandler.FunctionPage("static/function/pages/login.html"))
	mux.HandleFunc("/imagine", pageHandler.FunctionPage("static/function/pages/imagine.html"))
	mux.HandleFunc("/voice", pageHandler.FunctionPage("static/function/pages/voice.html"))
	mux.HandleFunc("/video", pageHandler.FunctionPage("static/function/pages/video.html"))
	mux.HandleFunc("/chat", pageHandler.FunctionPage("static/function/pages/chat.html"))
	mux.HandleFunc("/admin", pageHandler.AdminRoot())
	mux.HandleFunc("/admin/login", pageHandler.AdminPage("static/admin/pages/login.html"))
	mux.HandleFunc("/admin/config", pageHandler.AdminPage("static/admin/pages/config.html"))
	mux.HandleFunc("/admin/cache", pageHandler.AdminPage("static/admin/pages/cache.html"))
	mux.HandleFunc("/admin/token", pageHandler.AdminPage("static/admin/pages/token.html"))

	handler := middleware.CORS(middleware.RequestID(middleware.Logger(cfg, logger)(mux)))
	return handler
}
