package reverse

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
	"github.com/fran0220/grok2api-go/internal/token"
)

const mediaPostLinkAPI = "https://grok.com/rest/media/post/create-link"

type MediaPostLinkReverse struct {
	API string
}

func NewMediaPostLinkReverse() *MediaPostLinkReverse {
	return &MediaPostLinkReverse{API: mediaPostLinkAPI}
}

func (m *MediaPostLinkReverse) Request(ctx context.Context, session *ResettableSession, tokenValue, postID string) (string, error) {
	payload := map[string]any{
		"postId":   strings.TrimSpace(postID),
		"source":   "post-page",
		"platform": "web",
	}
	body, err := json.Marshal(payload)
	if err != nil {
		return "", fmt.Errorf("marshal media post link payload: %w", err)
	}
	headers := BuildHeaders(tokenValue, "application/json", "https://grok.com", "https://grok.com")
	requestTimeout := requestTimeoutDuration(getConfigFloat64("video.timeout", 60))

	var activeProxyKey string
	requestFn := func(callCtx context.Context) (*http.Response, error) {
		requestCtx, cancel := withOptionalTimeout(callCtx, requestTimeout)
		defer cancel()

		var proxyURL string
		activeProxyKey, proxyURL = proxycfg.GetCurrentProxyFrom("proxy.base_proxy_url")
		req, err := http.NewRequestWithContext(requestCtx, http.MethodPost, m.API, bytes.NewReader(body))
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
				Message:    fmt.Sprintf("media post create link failed: %d", resp.StatusCode),
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
		m.recordUnauthorized(tokenValue, err, "media_post_link_auth_failed")
		return "", err
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("read media post link response: %w", err)
	}

	var parsed struct {
		ShareLink string `json:"shareLink"`
	}
	if err := json.Unmarshal(data, &parsed); err != nil {
		return "", fmt.Errorf("decode media post link response: %w", err)
	}
	return strings.TrimSpace(parsed.ShareLink), nil
}

func (m *MediaPostLinkReverse) recordUnauthorized(tokenValue string, err error, reason string) {
	var upstreamErr *UpstreamError
	if !errors.As(err, &upstreamErr) || upstreamErr.StatusCode != http.StatusUnauthorized {
		return
	}
	_ = token.GetInstance().RecordFail(tokenValue, upstreamErr.StatusCode, reason)
}
