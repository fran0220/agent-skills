package reverse

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"strings"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
)

const RateLimitsAPI = "https://grok.com/rest/rate-limits"

var rateLimitAuthKeywords = []string{"unauthorized", "not logged in", "unauthenticated", "bad-credentials", "auth"}

type RateLimitsReverse struct {
	API string
}

func NewRateLimitsReverse() *RateLimitsReverse {
	return &RateLimitsReverse{API: RateLimitsAPI}
}

func (r *RateLimitsReverse) requestTimeout() time.Duration {
	timeout := getConfigFloat64("usage.timeout", 60)
	if timeout <= 0 {
		return 0
	}
	return time.Duration(timeout * float64(time.Second))
}

func (r *RateLimitsReverse) Request(ctx context.Context, session *ResettableSession, token string) (map[string]any, error) {
	payload, err := json.Marshal(map[string]any{
		"requestKind": "DEFAULT",
		"modelName":   "grok-4-1-thinking-1129",
	})
	if err != nil {
		return nil, fmt.Errorf("marshal rate-limits payload: %w", err)
	}

	headers := BuildHeaders(token, "application/json", "https://grok.com", "https://grok.com/")
	timeout := r.requestTimeout()
	var activeProxyKey string

	requestFn := func(callCtx context.Context) (*http.Response, error) {
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

		req, err := http.NewRequestWithContext(requestCtx, http.MethodPost, r.API, bytes.NewReader(payload))
		if err != nil {
			return nil, err
		}
		for key, value := range headers {
			req.Header.Set(key, value)
		}

		resp, err := session.DoWithProxy(req, activeProxyURL)
		if err != nil {
			return nil, err
		}
		if resp.StatusCode != http.StatusOK {
			content := readErrorBody(resp)
			upstreamErr := classifyRateLimitsError(resp, content)
			_ = resp.Body.Close()
			slog.Error("rate-limits upstream failure",
				"status", resp.StatusCode,
				"token_expired", upstreamErr.IsTokenExpired,
				"cloudflare", upstreamErr.IsCloudflare,
				"body", truncate(content, 300),
			)
			return nil, upstreamErr
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
		return nil, err
	}
	defer resp.Body.Close()

	var data map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&data); err != nil {
		return nil, fmt.Errorf("decode rate-limits response: %w", err)
	}
	return data, nil
}

func classifyRateLimitsError(resp *http.Response, body string) *UpstreamError {
	headers := http.Header{}
	statusCode := 0
	if resp != nil {
		headers = resp.Header.Clone()
		statusCode = resp.StatusCode
	}
	contentType := strings.ToLower(headers.Get("Content-Type"))
	serverHeader := strings.ToLower(headers.Get("Server"))
	bodyLower := strings.ToLower(body)
	isJSON := strings.Contains(contentType, "application/json")
	isCloudflare := strings.Contains(bodyLower, "challenge-platform")
	if strings.Contains(serverHeader, "cloudflare") && !isJSON {
		isCloudflare = true
	}
	isTokenExpired := statusCode == http.StatusUnauthorized && isJSON && containsAny(bodyLower, rateLimitAuthKeywords)

	return &UpstreamError{
		Message:        fmt.Sprintf("rate-limits request failed: %d", statusCode),
		StatusCode:     statusCode,
		Body:           body,
		Headers:        headers,
		IsTokenExpired: isTokenExpired,
		IsCloudflare:   isCloudflare,
	}
}

func containsAny(value string, keywords []string) bool {
	for _, keyword := range keywords {
		if keyword != "" && strings.Contains(value, keyword) {
			return true
		}
	}
	return false
}

func readJSONBody(body io.Reader) (map[string]any, error) {
	var data map[string]any
	if err := json.NewDecoder(body).Decode(&data); err != nil {
		return nil, err
	}
	return data, nil
}
