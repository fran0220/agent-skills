package api

import (
	"errors"
	"io/fs"
	"log/slog"
	"net/http"
	"os"
	"path/filepath"

	"github.com/fran0220/grok2api-go/internal/config"
)

type PageHandler struct {
	cfg      *config.Config
	logger   *slog.Logger
	publicFS fs.FS
	staticFS fs.FS
}

func NewPageHandler(cfg *config.Config, logger *slog.Logger) *PageHandler {
	publicDir := resolvePublicDir(cfg)
	var publicFS fs.FS
	var staticFS fs.FS
	if publicDir != "" {
		publicFS = os.DirFS(publicDir)
		if sub, err := fs.Sub(publicFS, "static"); err == nil {
			staticFS = sub
		} else if logger != nil {
			logger.Warn("resolve static assets", slog.Any("error", err))
		}
	} else if logger != nil {
		logger.Warn("public assets directory not found")
	}

	return &PageHandler{cfg: cfg, logger: logger, publicFS: publicFS, staticFS: staticFS}
}

func (h *PageHandler) Static() http.Handler {
	if h.staticFS == nil {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			http.NotFound(w, r)
		})
	}
	return http.StripPrefix("/static/", http.FileServer(http.FS(h.staticFS)))
}

func (h *PageHandler) Favicon() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			pageMethodNotAllowed(w, http.MethodGet)
			return
		}
		http.Redirect(w, r, "/static/common/img/favicon/favicon.ico", http.StatusTemporaryRedirect)
	}
}

func (h *PageHandler) Root() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/" {
			http.NotFound(w, r)
			return
		}
		if r.Method != http.MethodGet {
			pageMethodNotAllowed(w, http.MethodGet)
			return
		}
		if h.cfg.GetBool("app.function_enabled") {
			http.Redirect(w, r, "/login", http.StatusTemporaryRedirect)
			return
		}
		http.Redirect(w, r, "/admin/login", http.StatusTemporaryRedirect)
	}
}

func (h *PageHandler) AdminRoot() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			pageMethodNotAllowed(w, http.MethodGet)
			return
		}
		http.Redirect(w, r, "/admin/login", http.StatusTemporaryRedirect)
	}
}

func (h *PageHandler) AdminPage(name string) http.HandlerFunc {
	return h.servePublicPage(name, false)
}

func (h *PageHandler) FunctionPage(name string) http.HandlerFunc {
	return h.servePublicPage(name, true)
}

func (h *PageHandler) FunctionVerify() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			pageMethodNotAllowed(w, http.MethodGet)
			return
		}
		writeJSON(w, http.StatusOK, map[string]string{"status": "success"})
	})
}

func (h *PageHandler) servePublicPage(name string, requireFunctionEnabled bool) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			pageMethodNotAllowed(w, http.MethodGet)
			return
		}
		if requireFunctionEnabled && !h.cfg.GetBool("app.function_enabled") {
			http.NotFound(w, r)
			return
		}
		if h.publicFS == nil {
			http.NotFound(w, r)
			return
		}
		if _, err := fs.Stat(h.publicFS, name); err != nil {
			if errors.Is(err, fs.ErrNotExist) {
				http.NotFound(w, r)
				return
			}
			if h.logger != nil {
				h.logger.Warn("stat public asset", slog.String("path", name), slog.Any("error", err))
			}
			http.Error(w, http.StatusText(http.StatusInternalServerError), http.StatusInternalServerError)
			return
		}
		http.ServeFileFS(w, r, h.publicFS, name)
	}
}

func resolvePublicDir(cfg *config.Config) string {
	paths := cfg.Paths()
	defaultsRoot := filepath.Dir(paths.Defaults)
	workingDir, _ := os.Getwd()
	exePath, _ := os.Executable()
	pathsToCheck := []string{
		filepath.Join(defaultsRoot, "_public"),
		filepath.Join(workingDir, "_public"),
		filepath.Join(filepath.Dir(exePath), "_public"),
	}
	for _, candidate := range pathsToCheck {
		if candidate == "" {
			continue
		}
		if info, err := os.Stat(candidate); err == nil && info.IsDir() {
			return candidate
		}
	}
	return ""
}

func pageMethodNotAllowed(w http.ResponseWriter, allowed string) {
	w.Header().Set("Allow", allowed)
	http.Error(w, http.StatusText(http.StatusMethodNotAllowed), http.StatusMethodNotAllowed)
}
