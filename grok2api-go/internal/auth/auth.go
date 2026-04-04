package auth

import (
	"crypto/subtle"
	"encoding/json"
	"net/http"
	"strings"

	"github.com/fran0220/grok2api-go/internal/config"
)

func VerifyAPIKey(cfg *config.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			keys := normalizeKeys(cfg.GetString("app.api_key"))
			if len(keys) == 0 {
				next.ServeHTTP(w, r)
				return
			}

			token, ok := bearerToken(r)
			if !ok {
				writeUnauthorized(w, "Missing authentication token")
				return
			}
			for _, key := range keys {
				if secureCompare(token, key) {
					next.ServeHTTP(w, r)
					return
				}
			}
			writeUnauthorized(w, "Invalid authentication token")
		})
	}
}

func VerifyAppKey(cfg *config.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			appKey := strings.TrimSpace(cfg.GetString("app.app_key", "grok2api"))
			if appKey == "" {
				writeUnauthorized(w, "App key is not configured")
				return
			}

			token, ok := bearerToken(r)
			if !ok {
				writeUnauthorized(w, "Missing authentication token")
				return
			}
			if !secureCompare(token, appKey) {
				writeUnauthorized(w, "Invalid authentication token")
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}

func VerifyFunctionKey(cfg *config.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			functionKey := strings.TrimSpace(cfg.GetString("app.function_key"))
			functionEnabled := cfg.GetBool("app.function_enabled")

			if functionKey == "" {
				if functionEnabled {
					next.ServeHTTP(w, r)
					return
				}
				writeUnauthorized(w, "Function access is disabled")
				return
			}

			token, ok := bearerToken(r)
			if !ok {
				writeUnauthorized(w, "Missing authentication token")
				return
			}
			if !secureCompare(token, functionKey) {
				writeUnauthorized(w, "Invalid authentication token")
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}

func normalizeKeys(raw string) []string {
	if strings.TrimSpace(raw) == "" {
		return nil
	}
	parts := strings.Split(raw, ",")
	keys := make([]string, 0, len(parts))
	for _, part := range parts {
		trimmed := strings.TrimSpace(part)
		if trimmed == "" {
			continue
		}
		keys = append(keys, trimmed)
	}
	return keys
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
	if token == "" {
		return "", false
	}
	return token, true
}

func secureCompare(left, right string) bool {
	return subtle.ConstantTimeCompare([]byte(left), []byte(right)) == 1
}

func writeUnauthorized(w http.ResponseWriter, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("WWW-Authenticate", "Bearer")
	w.WriteHeader(http.StatusUnauthorized)
	_ = json.NewEncoder(w).Encode(map[string]string{"error": message})
}
