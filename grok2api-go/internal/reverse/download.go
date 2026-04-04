package reverse

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
	"strings"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
	"github.com/fran0220/grok2api-go/internal/storage"
)

const AssetsDownloadAPI = "https://assets.grok.com"

type DownloadService struct {
	session  *ResettableSession
	imageDir string
	videoDir string
}

func NewDownloadService() *DownloadService {
	baseDir := filepath.Join(storage.DataDir(), "tmp")
	imageDir := filepath.Join(baseDir, "image")
	videoDir := filepath.Join(baseDir, "video")
	_ = os.MkdirAll(imageDir, 0o755)
	_ = os.MkdirAll(videoDir, 0o755)
	return &DownloadService{
		session:  NewResettableSession(SessionOptions{Browser: getConfigString("proxy.browser", "")}),
		imageDir: imageDir,
		videoDir: videoDir,
	}
}

func (d *DownloadService) Close() {
	if d != nil && d.session != nil {
		d.session.Close()
	}
}

func (d *DownloadService) ResolveURL(pathOrURL, token, mediaType string) (string, error) {
	assetURL, assetPath, parsed, err := normalizeAssetURL(pathOrURL)
	if err != nil {
		return "", err
	}
	appURL := strings.TrimSpace(getConfigString("app.app_url", ""))
	if appURL == "" {
		return assetURL, nil
	}
	if parsed != nil && parsed.Host != "" && !strings.EqualFold(parsed.Host, "assets.grok.com") {
		return assetURL, nil
	}
	filename, _, err := d.DownloadFile(assetPath, token, mediaType)
	if err != nil {
		return "", err
	}
	return fmt.Sprintf("%s/v1/files/%s/%s", strings.TrimRight(appURL, "/"), mediaType, filename), nil
}

func (d *DownloadService) RenderImage(rawURL, token, imageID string) (string, error) {
	format := strings.ToLower(strings.TrimSpace(getConfigString("app.image_format", "url")))
	if format != "base64" && format != "markdown" && format != "url" {
		format = "url"
	}
	if format == "base64" {
		dataURI, err := d.ParseB64(rawURL, token, "image")
		if err != nil {
			return "", err
		}
		return fmt.Sprintf("![%s](%s)", imageID, dataURI), nil
	}
	resolved, err := d.ResolveURL(rawURL, token, "image")
	if err != nil {
		return "", err
	}
	return fmt.Sprintf("![%s](%s)", imageID, resolved), nil
}

func (d *DownloadService) ParseB64(pathOrURL, token, mediaType string) (string, error) {
	resp, err := d.fetch(pathOrURL, token)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()
	raw, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}
	contentType := strings.TrimSpace(strings.SplitN(resp.Header.Get("Content-Type"), ";", 2)[0])
	if contentType == "" {
		contentType = guessContentType(pathOrURL, mediaType)
	}
	return fmt.Sprintf("data:%s;base64,%s", contentType, base64.StdEncoding.EncodeToString(raw)), nil
}

func (d *DownloadService) DownloadFile(pathOrURL, token, mediaType string) (string, string, error) {
	resp, err := d.fetch(pathOrURL, token)
	if err != nil {
		return "", "", err
	}
	defer resp.Body.Close()

	_, assetPath, _, err := normalizeAssetURL(pathOrURL)
	if err != nil {
		return "", "", err
	}
	cacheDir := d.imageDir
	if mediaType == "video" {
		cacheDir = d.videoDir
	}
	cacheName := strings.TrimLeft(strings.ReplaceAll(strings.SplitN(assetPath, "?", 2)[0], "/", "-"), "-")
	if cacheName == "" {
		cacheName = fmt.Sprintf("%s-%d", mediaType, time.Now().UnixMilli())
	}
	cachePath := filepath.Join(cacheDir, cacheName)
	tmpPath := cachePath + ".tmp"
	file, err := os.Create(tmpPath)
	if err != nil {
		return "", "", err
	}
	if _, err := io.Copy(file, resp.Body); err != nil {
		file.Close()
		_ = os.Remove(tmpPath)
		return "", "", err
	}
	if err := file.Close(); err != nil {
		_ = os.Remove(tmpPath)
		return "", "", err
	}
	if err := os.Rename(tmpPath, cachePath); err != nil {
		_ = os.Remove(tmpPath)
		return "", "", err
	}
	contentType := strings.TrimSpace(strings.SplitN(resp.Header.Get("Content-Type"), ";", 2)[0])
	if contentType == "" {
		contentType = guessContentType(cacheName, mediaType)
	}
	return cacheName, contentType, nil
}

func (d *DownloadService) fetch(pathOrURL, token string) (*http.Response, error) {
	urlStr, requestPath, parsed, err := normalizeAssetURL(pathOrURL)
	if err != nil {
		return nil, err
	}
	origin := "https://assets.grok.com"
	referer := "https://grok.com/"
	if parsed != nil && parsed.Scheme != "" && parsed.Host != "" {
		origin = fmt.Sprintf("%s://%s", parsed.Scheme, parsed.Host)
		referer = origin + "/"
	}
	contentType := guessContentType(requestPath, "image")
	headers := BuildHeaders(token, contentType, origin, referer)
	headers["Cache-Control"] = "no-cache"
	headers["Pragma"] = "no-cache"
	headers["Priority"] = "u=0, i"
	headers["Sec-Fetch-Mode"] = "navigate"
	headers["Sec-Fetch-User"] = "?1"
	headers["Upgrade-Insecure-Requests"] = "1"
	timeout := time.Duration(getConfigInt("asset.download_timeout", 60)) * time.Second
	var activeProxyKey string

	return RetryOnStatus(context.Background(), func(callCtx context.Context) (*http.Response, error) {
		requestCtx := callCtx
		var cancel context.CancelFunc
		if timeout > 0 {
			requestCtx, cancel = context.WithTimeout(callCtx, timeout)
			defer cancel()
		}
		activeProxyKey, _ = proxycfg.GetCurrentProxyFrom("proxy.asset_proxy_url", "proxy.base_proxy_url")
		activeProxyURL := ""
		if activeProxyKey != "" {
			activeProxyURL = proxycfg.GetCurrentProxy(activeProxyKey)
		}
		request, err := http.NewRequestWithContext(requestCtx, http.MethodGet, urlStr, nil)
		if err != nil {
			return nil, err
		}
		for key, value := range headers {
			request.Header.Set(key, value)
		}
		response, err := d.session.DoWithProxy(request, activeProxyURL)
		if err != nil {
			return nil, err
		}
		if response.StatusCode != http.StatusOK {
			content := readResponseBody(response)
			return nil, &UpstreamError{
				Message:    fmt.Sprintf("download request failed: %d", response.StatusCode),
				StatusCode: response.StatusCode,
				Body:       content,
				Headers:    response.Header.Clone(),
			}
		}
		return response, nil
	}, RetryOptions{
		OnRetry: func(ctx context.Context, attempt, status int, err error, delay time.Duration) {
			if activeProxyKey != "" && proxycfg.ShouldRotateProxy(status) {
				proxycfg.RotateProxy(activeProxyKey)
			}
		},
	})
}

func normalizeAssetURL(pathOrURL string) (string, string, *url.URL, error) {
	value := strings.TrimSpace(pathOrURL)
	if value == "" || strings.HasPrefix(value, "data:") {
		return "", "", nil, fmt.Errorf("invalid asset path")
	}
	parsed, err := url.Parse(value)
	if err == nil && parsed.Scheme != "" && parsed.Host != "" {
		requestPath := parsed.Path
		if requestPath == "" {
			requestPath = "/"
		}
		if parsed.RawQuery != "" {
			requestPath += "?" + parsed.RawQuery
		}
		return value, requestPath, parsed, nil
	}
	if !strings.HasPrefix(value, "/") {
		value = "/" + value
	}
	return AssetsDownloadAPI + value, value, nil, nil
}

func guessContentType(value, mediaType string) string {
	ext := strings.ToLower(path.Ext(strings.SplitN(value, "?", 2)[0]))
	if mimeType := mime.TypeByExtension(ext); mimeType != "" {
		return mimeType
	}
	if mediaType == "video" {
		return "video/mp4"
	}
	return "image/jpeg"
}
