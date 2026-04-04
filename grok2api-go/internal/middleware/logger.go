package middleware

import (
	"log/slog"
	"net/http"
	"time"

	"github.com/fran0220/grok2api-go/internal/config"
)

func Logger(cfg *config.Config, logger *slog.Logger) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			start := time.Now()
			wrapped := &statusRecorder{ResponseWriter: w, statusCode: http.StatusOK}

			next.ServeHTTP(wrapped, r)

			duration := time.Since(start)
			if !shouldLog(cfg, r.URL.Path, wrapped.statusCode, duration) {
				return
			}

			attrs := []any{
				slog.String("request_id", RequestIDFromContext(r.Context())),
				slog.String("method", r.Method),
				slog.String("path", r.URL.Path),
				slog.Int("status", wrapped.statusCode),
				slog.Float64("duration_ms", float64(duration)/float64(time.Millisecond)),
			}

			switch {
			case wrapped.statusCode >= http.StatusInternalServerError:
				logger.Error("request completed", attrs...)
			case wrapped.statusCode >= http.StatusBadRequest:
				logger.Warn("request completed", attrs...)
			default:
				logger.Info("request completed", attrs...)
			}
		})
	}
}

func shouldLog(cfg *config.Config, path string, statusCode int, duration time.Duration) bool {
	if path == "/health" && !cfg.GetBool("log.log_health_requests", false) {
		return false
	}
	if cfg.GetBool("log.log_all_requests", false) {
		return true
	}
	slowMS := cfg.GetInt("log.request_slow_ms", 3000)
	return statusCode >= http.StatusBadRequest || duration >= time.Duration(slowMS)*time.Millisecond
}

type statusRecorder struct {
	http.ResponseWriter
	statusCode int
}

func (r *statusRecorder) WriteHeader(statusCode int) {
	r.statusCode = statusCode
	r.ResponseWriter.WriteHeader(statusCode)
}

func (r *statusRecorder) Write(data []byte) (int, error) {
	if r.statusCode == 0 {
		r.statusCode = http.StatusOK
	}
	return r.ResponseWriter.Write(data)
}
