package reverse

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
	"github.com/fran0220/grok2api-go/internal/token"
)

const AssetsDeleteAPI = "https://grok.com/rest/assets-metadata"

type AssetsDeleteReverse struct {
	API string
}

func NewAssetsDeleteReverse() *AssetsDeleteReverse {
	return &AssetsDeleteReverse{API: AssetsDeleteAPI}
}

func (r *AssetsDeleteReverse) Request(ctx context.Context, session *ResettableSession, tokenValue, assetID string) (map[string]any, error) {
	headers := BuildHeaders(tokenValue, "application/json", "https://grok.com", "https://grok.com/files")
	timeout := time.Duration(getConfigInt("asset.delete_timeout", 60)) * time.Second
	var activeProxyKey string

	resp, err := RetryOnStatus(ctx, func(callCtx context.Context) (*http.Response, error) {
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
		req, err := http.NewRequestWithContext(requestCtx, http.MethodDelete, r.API+"/"+assetID, nil)
		if err != nil {
			return nil, err
		}
		for key, value := range headers {
			req.Header.Set(key, value)
		}
		response, err := session.DoWithProxy(req, activeProxyURL)
		if err != nil {
			return nil, err
		}
		if response.StatusCode != http.StatusOK {
			content := readResponseBody(response)
			return nil, &UpstreamError{
				Message:    fmt.Sprintf("assets delete request failed: %d", response.StatusCode),
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
	if err != nil {
		var upstreamErr *UpstreamError
		if ok := asUpstreamError(err, &upstreamErr); ok && upstreamErr.StatusCode == http.StatusUnauthorized {
			_ = token.GetInstance().RecordFail(tokenValue, upstreamErr.StatusCode, "assets_delete_auth_failed")
		}
		return nil, err
	}
	defer resp.Body.Close()

	var result map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return map[string]any{}, nil
	}
	if result == nil {
		result = map[string]any{}
	}
	return result, nil
}
