package reverse

import (
	"bufio"
	"context"
	"crypto/tls"
	"encoding/base64"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net"
	"net/http"
	"net/url"
	"strings"
	"time"

	utls "github.com/refraction-networking/utls"
	"golang.org/x/net/http2"
	xproxy "golang.org/x/net/proxy"
)

// ClientOptions controls uTLS transport construction.
type ClientOptions struct {
	ProxyURL           string
	Browser            string
	SkipProxySSLVerify bool
	Timeout            time.Duration
}

// FingerprintTransport uses uTLS for HTTPS requests and a standard transport
// for plain HTTP requests.
type FingerprintTransport struct {
	proxyURL           *url.URL
	browser            string
	skipProxySSLVerify bool
	h2Transport        *http2.Transport
	httpTransport      *http.Transport
	dialer             *net.Dialer
	initErr            error
}

type bufferedConn struct {
	net.Conn
	reader *bufio.Reader
}

func (c *bufferedConn) Read(p []byte) (int, error) {
	if c.reader != nil && c.reader.Buffered() > 0 {
		return c.reader.Read(p)
	}
	return c.Conn.Read(p)
}

// NewFingerprintClient creates a client using the latest Chrome uTLS preset.
func NewFingerprintClient(proxyURL, browser string) *http.Client {
	return NewFingerprintClientWithOptions(ClientOptions{
		ProxyURL:           proxyURL,
		Browser:            browser,
		SkipProxySSLVerify: getConfigBool("proxy.skip_proxy_ssl_verify", false),
	})
}

// NewFingerprintClientWithOptions creates a client with explicit options.
func NewFingerprintClientWithOptions(opts ClientOptions) *http.Client {
	transport := NewFingerprintTransport(opts)
	client := &http.Client{Transport: transport}
	if opts.Timeout > 0 {
		client.Timeout = opts.Timeout
	}
	return client
}

// NewFingerprintTransport creates a new uTLS-capable round tripper.
func NewFingerprintTransport(opts ClientOptions) *FingerprintTransport {
	transport := &FingerprintTransport{
		browser:            opts.Browser,
		skipProxySSLVerify: opts.SkipProxySSLVerify,
		dialer:             &net.Dialer{Timeout: 30 * time.Second, KeepAlive: 30 * time.Second},
	}
	if transport.browser == "" {
		transport.browser = defaultBrowser()
	}
	if opts.ProxyURL != "" {
		proxyURL, err := parseProxyURL(opts.ProxyURL)
		if err != nil {
			transport.initErr = err
		} else {
			transport.proxyURL = proxyURL
		}
	}

	transport.h2Transport = &http2.Transport{
		DialTLSContext: func(ctx context.Context, network, addr string, _ *tls.Config) (net.Conn, error) {
			return transport.dialUTLSContext(ctx, network, addr)
		},
	}

	httpTransport := &http.Transport{ForceAttemptHTTP2: false}
	if transport.proxyURL == nil {
		httpTransport.DialContext = transport.dialer.DialContext
	} else if isSOCKSProxy(transport.proxyURL) {
		httpTransport.DialContext = transport.socksDialContext
	} else {
		httpTransport.Proxy = http.ProxyURL(transport.proxyURL)
		httpTransport.DialContext = transport.dialer.DialContext
	}
	transport.httpTransport = httpTransport

	return transport
}

func (t *FingerprintTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	if t.initErr != nil {
		return nil, t.initErr
	}
	if req.URL.Scheme == "https" {
		return t.h2Transport.RoundTrip(req)
	}
	return t.httpTransport.RoundTrip(req)
}

// CloseIdleConnections closes pooled idle connections.
func (t *FingerprintTransport) CloseIdleConnections() {
	if t.h2Transport != nil {
		t.h2Transport.CloseIdleConnections()
	}
	if t.httpTransport != nil {
		t.httpTransport.CloseIdleConnections()
	}
}

func parseProxyURL(raw string) (*url.URL, error) {
	proxyURL, err := url.Parse(raw)
	if err != nil {
		return nil, fmt.Errorf("parse proxy url: %w", err)
	}
	if proxyURL.Scheme == "" || proxyURL.Host == "" {
		return nil, fmt.Errorf("invalid proxy url: %q", raw)
	}
	return proxyURL, nil
}

func isSOCKSProxy(proxyURL *url.URL) bool {
	if proxyURL == nil {
		return false
	}
	switch strings.ToLower(proxyURL.Scheme) {
	case "socks5", "socks5h":
		return true
	default:
		return false
	}
}

func (t *FingerprintTransport) dialUTLSContext(ctx context.Context, network, addr string) (net.Conn, error) {
	rawConn, err := t.dialTargetContext(ctx, network, addr)
	if err != nil {
		return nil, err
	}

	host, _, err := net.SplitHostPort(addr)
	if err != nil {
		rawConn.Close()
		return nil, err
	}

	tlsConfig := &utls.Config{
		ServerName: host,
		NextProtos: []string{"h2", "http/1.1"},
	}
	uConn := utls.UClient(rawConn, tlsConfig, t.selectHelloID())
	if err := uConn.HandshakeContext(ctx); err != nil {
		rawConn.Close()
		return nil, err
	}
	return uConn, nil
}

func (t *FingerprintTransport) selectHelloID() utls.ClientHelloID {
	browser := strings.ToLower(strings.TrimSpace(t.browser))
	if strings.Contains(browser, "chrome") || browser == "" {
		if strings.Contains(browser, "136") {
			slog.Debug("uTLS does not ship HelloChrome_136 yet, using HelloChrome_Auto")
		}
		return utls.HelloChrome_Auto
	}
	return utls.HelloChrome_Auto
}

func (t *FingerprintTransport) dialTargetContext(ctx context.Context, network, addr string) (net.Conn, error) {
	if t.proxyURL == nil {
		return t.dialer.DialContext(ctx, network, addr)
	}
	if isSOCKSProxy(t.proxyURL) {
		return t.socksDialContext(ctx, network, addr)
	}
	return t.dialHTTPProxyTunnel(ctx, network, addr)
}

func (t *FingerprintTransport) socksDialContext(ctx context.Context, network, addr string) (net.Conn, error) {
	if t.proxyURL == nil {
		return t.dialer.DialContext(ctx, network, addr)
	}
	proxyURL := *t.proxyURL
	if proxyURL.Scheme == "socks5h" {
		proxyURL.Scheme = "socks5"
	}
	auth := &xproxy.Auth{}
	if proxyURL.User != nil {
		auth.User = proxyURL.User.Username()
		auth.Password, _ = proxyURL.User.Password()
		if auth.User == "" && auth.Password == "" {
			auth = nil
		}
	} else {
		auth = nil
	}
	dialer, err := xproxy.SOCKS5(network, proxyURL.Host, auth, t.dialer)
	if err != nil {
		return nil, err
	}
	type contextDialer interface {
		DialContext(context.Context, string, string) (net.Conn, error)
	}
	if ctxDialer, ok := dialer.(contextDialer); ok {
		return ctxDialer.DialContext(ctx, network, addr)
	}

	type dialResult struct {
		conn net.Conn
		err  error
	}
	resultCh := make(chan dialResult, 1)
	go func() {
		conn, dialErr := dialer.Dial(network, addr)
		resultCh <- dialResult{conn: conn, err: dialErr}
	}()

	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	case result := <-resultCh:
		return result.conn, result.err
	}
}

func (t *FingerprintTransport) dialHTTPProxyTunnel(ctx context.Context, network, addr string) (net.Conn, error) {
	if t.proxyURL == nil {
		return nil, errors.New("proxy url is required for HTTP proxy tunnel")
	}

	proxyConn, err := t.dialer.DialContext(ctx, network, t.proxyURL.Host)
	if err != nil {
		return nil, err
	}

	if strings.EqualFold(t.proxyURL.Scheme, "https") {
		proxyHost := t.proxyURL.Hostname()
		proxyTLS := tls.Client(proxyConn, &tls.Config{
			ServerName:         proxyHost,
			InsecureSkipVerify: t.skipProxySSLVerify,
		})
		if err := proxyTLS.HandshakeContext(ctx); err != nil {
			proxyConn.Close()
			return nil, err
		}
		proxyConn = proxyTLS
	}

	req := &http.Request{
		Method: http.MethodConnect,
		URL:    &url.URL{Opaque: addr},
		Host:   addr,
		Header: make(http.Header),
	}
	if t.proxyURL.User != nil {
		password, _ := t.proxyURL.User.Password()
		basicAuth := base64.StdEncoding.EncodeToString([]byte(t.proxyURL.User.Username() + ":" + password))
		req.Header.Set("Proxy-Authorization", "Basic "+basicAuth)
	}
	if err := req.Write(proxyConn); err != nil {
		proxyConn.Close()
		return nil, err
	}

	reader := bufio.NewReader(proxyConn)
	resp, err := http.ReadResponse(reader, req)
	if err != nil {
		proxyConn.Close()
		return nil, err
	}
	if resp.Body != nil {
		defer resp.Body.Close()
		_, _ = io.Copy(io.Discard, resp.Body)
	}
	if resp.StatusCode != http.StatusOK {
		proxyConn.Close()
		return nil, fmt.Errorf("proxy CONNECT failed: %s", resp.Status)
	}
	return &bufferedConn{Conn: proxyConn, reader: reader}, nil
}
