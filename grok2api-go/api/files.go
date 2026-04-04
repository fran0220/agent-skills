package api

import (
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"github.com/fran0220/grok2api-go/internal/storage"
)

func handleImageFile() http.HandlerFunc {
	imageDir := filepath.Join(storage.DataDir(), "tmp", "image")
	return handleLocalFile("/v1/files/image/", imageDir, map[string]string{".png": "image/png", ".webp": "image/webp", ".jpg": "image/jpeg", ".jpeg": "image/jpeg"}, "Image not found")
}

func handleVideoFile() http.HandlerFunc {
	videoDir := filepath.Join(storage.DataDir(), "tmp", "video")
	return handleLocalFile("/v1/files/video/", videoDir, map[string]string{".mp4": "video/mp4", ".webm": "video/webm"}, "Video not found")
}

func handleLocalFile(prefix, baseDir string, contentTypes map[string]string, notFound string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w, http.MethodGet)
			return
		}
		name := strings.ReplaceAll(strings.TrimPrefix(r.URL.Path, prefix), "/", "-")
		if name == "" {
			writeJSON(w, http.StatusNotFound, map[string]string{"error": notFound})
			return
		}
		path := filepath.Join(baseDir, name)
		info, err := os.Stat(path)
		if err != nil || !info.Mode().IsRegular() {
			writeJSON(w, http.StatusNotFound, map[string]string{"error": notFound})
			return
		}
		contentType := contentTypes[strings.ToLower(filepath.Ext(path))]
		if contentType == "" {
			contentType = http.DetectContentType([]byte(filepath.Base(path)))
		}
		w.Header().Set("Cache-Control", "public, max-age=31536000, immutable")
		w.Header().Set("Content-Type", contentType)
		http.ServeFile(w, r, path)
	}
}
