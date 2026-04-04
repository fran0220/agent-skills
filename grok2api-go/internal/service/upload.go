package service

import (
	"context"
	"encoding/base64"
	"fmt"
	"io"
	"mime"
	"net/http"
	"net/url"
	"os"
	"path"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	"github.com/fran0220/grok2api-go/internal/config"
	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/storage"
)

type UploadService struct {
	cfg     *config.Config
	session *reverse.ResettableSession
}

func NewUploadService(cfg *config.Config) *UploadService {
	return &UploadService{
		cfg:     cfg,
		session: reverse.NewResettableSession(reverse.SessionOptions{Browser: cfg.GetString("proxy.browser", "")}),
	}
}

func (u *UploadService) Close() {
	if u != nil && u.session != nil {
		u.session.Close()
	}
}

func (u *UploadService) UploadFile(fileInput, token string) (string, string, error) {
	filename, base64Data, mimeType, err := u.CheckFormat(fileInput)
	if err != nil {
		return "", "", err
	}
	return reverse.AssetsUploadReverse{}.Request(context.Background(), u.session, token, filename, mimeType, base64Data)
}

func (u *UploadService) CheckFormat(fileInput string) (string, string, string, error) {
	trimmed := strings.TrimSpace(fileInput)
	if trimmed == "" {
		return "", "", "", fmt.Errorf("invalid file input: empty content")
	}
	if isURL(trimmed) {
		return u.ParseB64(trimmed)
	}
	if strings.HasPrefix(trimmed, "data:") {
		return u.FormatB64(trimmed)
	}
	return "", "", "", fmt.Errorf("invalid file input: must be URL or base64")
}

func (u *UploadService) ParseB64(rawURL string) (string, string, string, error) {
	if filename, data, mimeType, ok, err := u.readLocalFile(rawURL); ok || err != nil {
		return filename, data, mimeType, err
	}
	timeout := time.Duration(u.cfg.GetInt("asset.upload_timeout", 60)) * time.Second
	var activeProxyKey string
	resp, err := reverse.RetryOnStatus(context.Background(), func(callCtx context.Context) (*http.Response, error) {
		requestCtx := callCtx
		var cancel context.CancelFunc
		if timeout > 0 {
			requestCtx, cancel = context.WithTimeout(callCtx, timeout)
			defer cancel()
		}
		activeProxyKey, _ = proxycfg.GetCurrentProxyFrom("proxy.base_proxy_url")
		activeProxyURL := ""
		if activeProxyKey != "" {
			activeProxyURL = proxycfg.GetCurrentProxy(activeProxyKey)
		}
		request, err := http.NewRequestWithContext(requestCtx, http.MethodGet, rawURL, nil)
		if err != nil {
			return nil, err
		}
		request.Header.Set("User-Agent", u.cfg.GetString("proxy.user_agent", "Mozilla/5.0"))
		response, err := u.session.DoWithProxy(request, activeProxyURL)
		if err != nil {
			return nil, err
		}
		if response.StatusCode >= http.StatusBadRequest {
			content := readLimitedBody(response.Body)
			return nil, &reverse.UpstreamError{Message: fmt.Sprintf("fetch failed: %d", response.StatusCode), StatusCode: response.StatusCode, Body: content, Headers: response.Header.Clone()}
		}
		return response, nil
	}, reverse.RetryOptions{
		OnRetry: func(ctx context.Context, attempt, status int, err error, delay time.Duration) {
			if activeProxyKey != "" && proxycfg.ShouldRotateProxy(status) {
				proxycfg.RotateProxy(activeProxyKey)
			}
		},
	})
	if err != nil {
		return "", "", "", err
	}
	defer resp.Body.Close()
	raw, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", "", "", err
	}
	filename := path.Base(strings.SplitN(rawURL, "?", 2)[0])
	if filename == "." || filename == "/" || filename == "" {
		filename = "download"
	}
	contentType := strings.TrimSpace(strings.SplitN(resp.Header.Get("Content-Type"), ";", 2)[0])
	if contentType == "" {
		contentType = inferMime(filename)
	}
	return filename, base64.StdEncoding.EncodeToString(raw), contentType, nil
}

func (u *UploadService) FormatB64(dataURI string) (string, string, string, error) {
	if !strings.HasPrefix(dataURI, "data:") {
		return "", "", "", fmt.Errorf("invalid file input: not a data URI")
	}
	parts := strings.SplitN(dataURI, ",", 2)
	if len(parts) != 2 {
		return "", "", "", fmt.Errorf("invalid data URI format")
	}
	header := parts[0]
	payload := regexp.MustCompile(`\s+`).ReplaceAllString(parts[1], "")
	if !strings.Contains(header, ";base64") {
		return "", "", "", fmt.Errorf("invalid data URI: missing base64 marker")
	}
	mimeType := strings.TrimPrefix(strings.SplitN(header, ";", 2)[0], "data:")
	if mimeType == "" || payload == "" {
		return "", "", "", fmt.Errorf("invalid data URI: empty content")
	}
	ext := "bin"
	if slash := strings.Index(mimeType, "/"); slash >= 0 && slash+1 < len(mimeType) {
		ext = mimeType[slash+1:]
	}
	return "file." + ext, payload, mimeType, nil
}

func (u *UploadService) readLocalFile(rawURL string) (string, string, string, bool, error) {
	appURL := strings.TrimSpace(u.cfg.GetString("app.app_url", ""))
	if appURL == "" || !isURL(rawURL) {
		return "", "", "", false, nil
	}
	parsed, err := url.Parse(rawURL)
	if err != nil {
		return "", "", "", false, nil
	}
	appParsed, err := url.Parse(appURL)
	if err != nil {
		return "", "", "", false, nil
	}
	if parsed.Scheme != appParsed.Scheme || parsed.Host != appParsed.Host || !strings.HasPrefix(parsed.Path, "/v1/files/") {
		return "", "", "", false, nil
	}
	parts := strings.Split(strings.TrimPrefix(parsed.Path, "/v1/files/"), "/")
	if len(parts) < 2 {
		return "", "", "", false, fmt.Errorf("invalid local file path")
	}
	mediaType := parts[0]
	name := strings.Join(parts[1:], "-")
	baseDir := filepath.Join(storage.DataDir(), "tmp")
	localDir := filepath.Join(baseDir, mediaType)
	localPath := filepath.Join(localDir, name)
	info, err := os.Stat(localPath)
	if err != nil {
		return "", "", "", false, err
	}
	if !info.Mode().IsRegular() {
		return "", "", "", false, fmt.Errorf("invalid local file")
	}
	raw, err := os.ReadFile(localPath)
	if err != nil {
		return "", "", "", false, err
	}
	return name, base64.StdEncoding.EncodeToString(raw), inferMime(name), true, nil
}

func isURL(value string) bool {
	parsed, err := url.Parse(value)
	return err == nil && parsed.Scheme != "" && parsed.Host != "" && (parsed.Scheme == "http" || parsed.Scheme == "https")
}

func inferMime(filename string) string {
	if detected := mime.TypeByExtension(strings.ToLower(filepath.Ext(filename))); detected != "" {
		return detected
	}
	return "application/octet-stream"
}

func readLimitedBody(body io.ReadCloser) string {
	defer body.Close()
	data, _ := io.ReadAll(io.LimitReader(body, 4096))
	return string(data)
}
