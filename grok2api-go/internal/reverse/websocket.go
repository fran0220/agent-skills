package reverse

import (
	"context"
	"crypto/tls"
	"fmt"
	"io"
	"log/slog"
	"net"
	"net/http"
	"net/url"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
	"github.com/gorilla/websocket"
	xproxy "golang.org/x/net/proxy"
)

type WebSocketClient struct {
	ProxyURL string
}

func NewWebSocketClient(proxyURL string) *WebSocketClient {
	return &WebSocketClient{ProxyURL: proxyURL}
}

func (c *WebSocketClient) Connect(ctx context.Context, urlStr string, headers map[string]string, timeout time.Duration) (*websocket.Conn, error) {
	maxRetry := max(0, getConfigInt("retry.max_retry", 0))
	var lastErr error

	for attempt := 0; attempt <= maxRetry; attempt++ {
		activeProxyKey := ""
		proxyURL := c.ProxyURL
		if proxyURL == "" {
			activeProxyKey, proxyURL = proxycfg.GetCurrentProxyFrom("proxy.base_proxy_url")
		}

		dialer, err := c.newDialer(proxyURL, timeout)
		if err != nil {
			return nil, err
		}
		httpHeader := make(http.Header, len(headers))
		for key, value := range headers {
			httpHeader.Set(key, value)
		}

		conn, resp, err := dialer.DialContext(ctx, urlStr, httpHeader)
		if err == nil {
			return conn, nil
		}
		lastErr = wrapWSError(err, resp)
		if c.ProxyURL != "" || activeProxyKey == "" || attempt >= maxRetry {
			return nil, lastErr
		}
		proxycfg.RotateProxy(activeProxyKey)
		slog.Warn("websocket connect failed, rotating proxy", "attempt", attempt+1, "max_retry", maxRetry, "proxy_key", activeProxyKey, "error", err)
	}

	if lastErr != nil {
		return nil, lastErr
	}
	return nil, fmt.Errorf("websocket connect failed")
}

func (c *WebSocketClient) newDialer(proxyURL string, timeout time.Duration) (*websocket.Dialer, error) {
	dialer := &websocket.Dialer{
		HandshakeTimeout:  timeout,
		EnableCompression: true,
		TLSClientConfig: &tls.Config{
			MinVersion: tls.VersionTLS12,
		},
	}
	if proxyURL == "" {
		return dialer, nil
	}
	parsed, err := parseProxyURL(proxyURL)
	if err != nil {
		return nil, err
	}
	if isSOCKSProxy(parsed) {
		dialer.NetDialContext = newSOCKSDialContext(parsed, timeout)
		return dialer, nil
	}
	dialer.Proxy = http.ProxyURL(parsed)
	return dialer, nil
}

func newSOCKSDialContext(proxyURL *url.URL, timeout time.Duration) func(context.Context, string, string) (net.Conn, error) {
	baseDialer := &net.Dialer{Timeout: timeout, KeepAlive: 30 * time.Second}
	return func(ctx context.Context, network, addr string) (net.Conn, error) {
		if proxyURL == nil {
			return baseDialer.DialContext(ctx, network, addr)
		}
		proxyCopy := *proxyURL
		if proxyCopy.Scheme == "socks5h" {
			proxyCopy.Scheme = "socks5"
		}
		var auth *xproxy.Auth
		if proxyCopy.User != nil {
			auth = &xproxy.Auth{User: proxyCopy.User.Username()}
			auth.Password, _ = proxyCopy.User.Password()
			if auth.User == "" && auth.Password == "" {
				auth = nil
			}
		}
		d, err := xproxy.SOCKS5(network, proxyCopy.Host, auth, baseDialer)
		if err != nil {
			return nil, err
		}
		type contextDialer interface {
			DialContext(context.Context, string, string) (net.Conn, error)
		}
		if ctxDialer, ok := d.(contextDialer); ok {
			return ctxDialer.DialContext(ctx, network, addr)
		}
		type dialResult struct {
			conn net.Conn
			err  error
		}
		resultCh := make(chan dialResult, 1)
		go func() {
			conn, dialErr := d.Dial(network, addr)
			resultCh <- dialResult{conn: conn, err: dialErr}
		}()
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case result := <-resultCh:
			return result.conn, result.err
		}
	}
}

func wrapWSError(err error, resp *http.Response) error {
	if resp == nil {
		return err
	}
	defer resp.Body.Close()
	body, _ := io.ReadAll(io.LimitReader(resp.Body, 4096))
	return &UpstreamError{
		Message:    fmt.Sprintf("websocket connect failed: %d", resp.StatusCode),
		StatusCode: resp.StatusCode,
		Body:       string(body),
		Headers:    resp.Header.Clone(),
		Cause:      err,
	}
}
