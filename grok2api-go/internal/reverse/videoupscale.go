package reverse

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"regexp"
	"strings"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
	"github.com/fran0220/grok2api-go/internal/token"
)

const videoUpscaleAPI = "https://grok.com/rest/media/video/upscale"

var videoIDPattern = regexp.MustCompile(`/generated/([0-9a-fA-F-]{32,36})/|/([0-9a-fA-F-]{32,36})/generated_video`)

type VideoUpscaleReverse struct {
	API string
}

func NewVideoUpscaleReverse() *VideoUpscaleReverse {
	return &VideoUpscaleReverse{API: videoUpscaleAPI}
}

func (v *VideoUpscaleReverse) Request(ctx context.Context, session *ResettableSession, tokenValue, videoRef string) (string, error) {
	videoID := NormalizeVideoID(videoRef)
	if videoID == "" {
		return "", fmt.Errorf("video upscale requires a valid video id or url")
	}
	payload := map[string]any{"videoId": videoID}
	body, err := json.Marshal(payload)
	if err != nil {
		return "", fmt.Errorf("marshal video upscale payload: %w", err)
	}
	headers := BuildHeaders(tokenValue, "application/json", "https://grok.com", "https://grok.com")
	requestTimeout := requestTimeoutDuration(getConfigFloat64("video.timeout", 60))

	var activeProxyKey string
	requestFn := func(callCtx context.Context) (*http.Response, error) {
		requestCtx, cancel := withOptionalTimeout(callCtx, requestTimeout)
		defer cancel()

		var proxyURL string
		activeProxyKey, proxyURL = proxycfg.GetCurrentProxyFrom("proxy.base_proxy_url")
		req, err := http.NewRequestWithContext(requestCtx, http.MethodPost, v.API, bytes.NewReader(body))
		if err != nil {
			return nil, err
		}
		for key, value := range headers {
			req.Header.Set(key, value)
		}

		resp, err := session.DoWithProxy(req, proxyURL)
		if err != nil {
			return nil, err
		}
		if resp.StatusCode != http.StatusOK {
			content := readErrorBody(resp)
			return nil, &UpstreamError{
				Message:    fmt.Sprintf("video upscale failed: %d", resp.StatusCode),
				StatusCode: resp.StatusCode,
				Body:       content,
				Headers:    resp.Header.Clone(),
			}
		}
		return resp, nil
	}

	resp, err := RetryOnStatus(ctx, requestFn, RetryOptions{
		OnRetry: func(ctx context.Context, attempt, status int, err error, delay time.Duration) {
			if activeProxyKey != "" && proxycfg.ShouldRotateProxy(status) {
				proxycfg.RotateProxy(activeProxyKey)
			}
		},
	})
	if err != nil {
		v.recordUnauthorized(tokenValue, err, "video_upscale_auth_failed")
		return "", err
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("read video upscale response: %w", err)
	}

	var parsed struct {
		HDMediaURL string `json:"hdMediaUrl"`
	}
	if err := json.Unmarshal(data, &parsed); err != nil {
		return "", fmt.Errorf("decode video upscale response: %w", err)
	}
	if strings.TrimSpace(parsed.HDMediaURL) == "" {
		return "", &UpstreamError{
			Message:    "video upscale missing hd media url",
			StatusCode: http.StatusBadGateway,
			Body:       string(data),
		}
	}
	return strings.TrimSpace(parsed.HDMediaURL), nil
}

func NormalizeVideoID(videoRef string) string {
	trimmed := strings.TrimSpace(videoRef)
	if trimmed == "" {
		return ""
	}
	match := videoIDPattern.FindStringSubmatch(trimmed)
	if len(match) >= 3 {
		if strings.TrimSpace(match[1]) != "" {
			return strings.TrimSpace(match[1])
		}
		if strings.TrimSpace(match[2]) != "" {
			return strings.TrimSpace(match[2])
		}
	}
	return trimmed
}

func (v *VideoUpscaleReverse) recordUnauthorized(tokenValue string, err error, reason string) {
	var upstreamErr *UpstreamError
	if !errors.As(err, &upstreamErr) || upstreamErr.StatusCode != http.StatusUnauthorized {
		return
	}
	_ = token.GetInstance().RecordFail(tokenValue, upstreamErr.StatusCode, reason)
}

func requestTimeoutDuration(seconds float64) time.Duration {
	if seconds <= 0 {
		return 0
	}
	return time.Duration(seconds * float64(time.Second))
}

func withOptionalTimeout(ctx context.Context, timeout time.Duration) (context.Context, context.CancelFunc) {
	if timeout <= 0 {
		return ctx, func() {}
	}
	return context.WithTimeout(ctx, timeout)
}
