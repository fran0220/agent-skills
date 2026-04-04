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

const mediaPostAPI = "https://grok.com/rest/media/post/create"

type MediaPostReverse struct {
	API string
}

func NewMediaPostReverse() *MediaPostReverse {
	return &MediaPostReverse{API: mediaPostAPI}
}

func (m *MediaPostReverse) Request(ctx context.Context, session *ResettableSession, tokenValue, mediaType, mediaURL, prompt string) (string, error) {
	payload := map[string]any{"mediaType": mediaType}
	if strings.TrimSpace(mediaURL) != "" {
		payload["mediaUrl"] = mediaURL
	}
	if strings.TrimSpace(prompt) != "" {
		payload["prompt"] = prompt
	}
	body, err := json.Marshal(payload)
	if err != nil {
		return "", fmt.Errorf("marshal media post payload: %w", err)
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
				Message:    fmt.Sprintf("media post create failed: %d", resp.StatusCode),
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
		m.recordUnauthorized(tokenValue, err, "media_post_auth_failed")
		return "", err
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("read media post response: %w", err)
	}

	var parsed struct {
		Post struct {
			ID string `json:"id"`
		} `json:"post"`
	}
	if err := json.Unmarshal(data, &parsed); err != nil {
		return "", fmt.Errorf("decode media post response: %w", err)
	}
	if strings.TrimSpace(parsed.Post.ID) == "" {
		return "", &UpstreamError{
			Message:    "media post create missing post id",
			StatusCode: http.StatusBadGateway,
			Body:       string(data),
		}
	}
	return strings.TrimSpace(parsed.Post.ID), nil
}

func (m *MediaPostReverse) recordUnauthorized(tokenValue string, err error, reason string) {
	var upstreamErr *UpstreamError
	if !errors.As(err, &upstreamErr) || upstreamErr.StatusCode != http.StatusUnauthorized {
		return
	}
	_ = token.GetInstance().RecordFail(tokenValue, upstreamErr.StatusCode, reason)
}
