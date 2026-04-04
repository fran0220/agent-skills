package reverse

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
	"github.com/fran0220/grok2api-go/internal/token"
	"github.com/gorilla/websocket"
)

const (
	LivekitTokenAPI = "https://grok.com/rest/livekit/tokens"
	LivekitWSURL    = "wss://livekit.grok.com"
)

type LivekitTokenReverse struct {
	API string
}

func NewLivekitTokenReverse() *LivekitTokenReverse {
	return &LivekitTokenReverse{API: LivekitTokenAPI}
}

func (r *LivekitTokenReverse) Request(ctx context.Context, session *ResettableSession, tokenValue, voice, personality string, speed float64) (map[string]any, error) {
	headers := BuildHeaders(tokenValue, "application/json", "https://grok.com", "https://grok.com/")
	payload := map[string]any{
		"sessionPayload": string(mustJSON(map[string]any{
			"voice":          defaultString(voice, "ara"),
			"personality":    defaultString(personality, "assistant"),
			"playback_speed": speed,
			"enable_vision":  false,
			"turn_detection": map[string]any{"type": "server_vad"},
		})),
		"requestAgentDispatch": false,
		"livekitUrl":           LivekitWSURL,
		"params":               map[string]string{"enable_markdown_transcript": "true"},
	}
	body := mustJSON(payload)
	timeout := time.Duration(getConfigInt("voice.timeout", 60)) * time.Second
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
		if response.StatusCode != http.StatusOK {
			content := readResponseBody(response)
			return nil, &UpstreamError{
				Message:    fmt.Sprintf("livekit token request failed: %d", response.StatusCode),
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
			_ = token.GetInstance().RecordFail(tokenValue, upstreamErr.StatusCode, "livekit_token_auth_failed")
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

type LivekitWebSocketReverse struct {
	client *WebSocketClient
}

func NewLivekitWebSocketReverse() *LivekitWebSocketReverse {
	return &LivekitWebSocketReverse{client: NewWebSocketClient("")}
}

func (r *LivekitWebSocketReverse) Connect(ctx context.Context, accessToken string) (*websocket.Conn, error) {
	base := strings.TrimRight(LivekitWSURL, "/")
	if !strings.HasSuffix(base, "/rtc") {
		base += "/rtc"
	}
	params := url.Values{}
	params.Set("access_token", accessToken)
	params.Set("auto_subscribe", "1")
	params.Set("sdk", "js")
	params.Set("version", "2.11.4")
	params.Set("protocol", "15")
	wsURL := base + "?" + params.Encode()
	headers := BuildWSHeaders("", "https://grok.com")
	return r.client.Connect(ctx, wsURL, headers, time.Duration(getConfigInt("voice.timeout", 60))*time.Second)
}

func mustJSON(value any) []byte {
	data, _ := json.Marshal(value)
	return data
}

func defaultString(value, fallback string) string {
	if strings.TrimSpace(value) == "" {
		return fallback
	}
	return value
}
