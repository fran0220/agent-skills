package reverse

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"math/rand/v2"
	"net/http"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
)

const SetBirthAPI = "https://grok.com/rest/auth/set-birth-date"

type SetBirthReverse struct {
	API string
}

func NewSetBirthReverse() *SetBirthReverse {
	return &SetBirthReverse{API: SetBirthAPI}
}

func (r *SetBirthReverse) Request(ctx context.Context, session *ResettableSession, token string) error {
	headers := BuildHeaders(token, "application/json", "https://grok.com", "https://grok.com/?_s=home")
	body, err := json.Marshal(map[string]string{"birthDate": randomBirthDate()})
	if err != nil {
		return err
	}
	timeout := time.Duration(getConfigInt("nsfw.timeout", 60)) * time.Second
	var activeProxyKey string

	resp, err := RetryOnStatus(ctx, func(callCtx context.Context) (*http.Response, error) {
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
		req, err := http.NewRequestWithContext(requestCtx, http.MethodPost, r.API, bytes.NewReader(body))
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
		if response.StatusCode != http.StatusOK && response.StatusCode != http.StatusNoContent {
			content := readResponseBody(response)
			return nil, &UpstreamError{
				Message:    fmt.Sprintf("set birth request failed: %d", response.StatusCode),
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
	if resp != nil && resp.Body != nil {
		defer resp.Body.Close()
	}
	return err
}

func randomBirthDate() string {
	year, _, _ := time.Now().UTC().Date()
	birthYear := year - (20 + rand.IntN(29))
	birthMonth := 1 + rand.IntN(12)
	birthDay := 1 + rand.IntN(28)
	hour := rand.IntN(24)
	minute := rand.IntN(60)
	second := rand.IntN(60)
	millisecond := rand.IntN(1000)
	return fmt.Sprintf("%04d-%02d-%02dT%02d:%02d:%02d.%03dZ", birthYear, birthMonth, birthDay, hour, minute, second, millisecond)
}
