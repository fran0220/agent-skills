package reverse

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
	"github.com/fran0220/grok2api-go/internal/token"
)

const AssetsListAPI = "https://grok.com/rest/assets"

type AssetsListReverse struct {
	API string
}

func NewAssetsListReverse() *AssetsListReverse {
	return &AssetsListReverse{API: AssetsListAPI}
}

func (r *AssetsListReverse) Request(ctx context.Context, session *ResettableSession, tokenValue string, params map[string]any) (map[string]any, error) {
	headers := BuildHeaders(tokenValue, "application/json", "https://grok.com", "https://grok.com/files")
	timeout := time.Duration(getConfigInt("asset.list_timeout", 60)) * time.Second
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
		req, err := http.NewRequestWithContext(requestCtx, http.MethodGet, r.API, nil)
		if err != nil {
			return nil, err
		}
		for key, value := range headers {
			req.Header.Set(key, value)
		}
		query := req.URL.Query()
		for key, value := range params {
			switch typed := value.(type) {
			case []string:
				for _, item := range typed {
					query.Add(key, item)
				}
			default:
				query.Set(key, fmt.Sprint(value))
			}
		}
		req.URL.RawQuery = query.Encode()
		response, err := session.DoWithProxy(req, activeProxyURL)
		if err != nil {
			return nil, err
		}
		if response.StatusCode != http.StatusOK {
			content := readResponseBody(response)
			return nil, &UpstreamError{
				Message:    fmt.Sprintf("assets list request failed: %d", response.StatusCode),
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
			_ = token.GetInstance().RecordFail(tokenValue, upstreamErr.StatusCode, "assets_list_auth_failed")
		}
		return nil, err
	}
	defer resp.Body.Close()

	var result map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}
	if result == nil {
		result = map[string]any{}
	}
	return result, nil
}

func asUpstreamError(err error, target **UpstreamError) bool {
	if err == nil {
		return false
	}
	upstreamErr, ok := err.(*UpstreamError)
	if ok {
		*target = upstreamErr
	}
	return ok
}

func cloneValues(values url.Values) url.Values {
	if values == nil {
		return url.Values{}
	}
	cloned := make(url.Values, len(values))
	for key, items := range values {
		cloned[key] = append([]string(nil), items...)
	}
	return cloned
}
