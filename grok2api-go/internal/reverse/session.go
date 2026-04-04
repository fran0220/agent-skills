package reverse

import (
	"net/http"
	"sync"
)

var defaultResetSessionStatusCodes = []int{403}

// SessionOptions controls ResettableSession construction.
type SessionOptions struct {
	Browser            string
	ResetOnStatus      []int
	SkipProxySSLVerify bool
}

// ResettableSession rebuilds its underlying client when requested or when a
// configured response status code is observed.
type ResettableSession struct {
	mu                 sync.Mutex
	browser            string
	skipProxySSLVerify bool
	resetOnStatus      map[int]struct{}
	resetRequested     bool
	proxyURL           string
	client             *http.Client
}

// NewResettableSession returns a session wrapper that can rebuild the client.
func NewResettableSession(opts SessionOptions) *ResettableSession {
	browser := opts.Browser
	if browser == "" {
		browser = defaultBrowser()
	}
	resetCodes := opts.ResetOnStatus
	if len(resetCodes) == 0 {
		resetCodes = getConfigIntSlice("retry.reset_session_status_codes", defaultResetSessionStatusCodes)
	}
	session := &ResettableSession{
		browser:            browser,
		skipProxySSLVerify: opts.SkipProxySSLVerify || getConfigBool("proxy.skip_proxy_ssl_verify", false),
		resetOnStatus:      makeRetryCodeSet(resetCodes),
	}
	session.client = session.newClient("")
	return session
}

func (s *ResettableSession) newClient(proxyURL string) *http.Client {
	return NewFingerprintClientWithOptions(ClientOptions{
		ProxyURL:           proxyURL,
		Browser:            s.browser,
		SkipProxySSLVerify: s.skipProxySSLVerify,
	})
}

func (s *ResettableSession) maybeResetLocked(proxyURL string) {
	if !s.resetRequested && proxyURL == s.proxyURL && s.client != nil {
		return
	}
	oldClient := s.client
	s.client = s.newClient(proxyURL)
	s.proxyURL = proxyURL
	s.resetRequested = false
	closeIdleConnections(oldClient)
}

// Do runs the request with the current client state.
func (s *ResettableSession) Do(req *http.Request) (*http.Response, error) {
	return s.DoWithProxy(req, "")
}

// DoWithProxy runs the request using a client bound to the provided proxy.
func (s *ResettableSession) DoWithProxy(req *http.Request, proxyURL string) (*http.Response, error) {
	s.mu.Lock()
	s.maybeResetLocked(proxyURL)
	client := s.client
	s.mu.Unlock()

	resp, err := client.Do(req)
	if err != nil {
		return nil, err
	}
	if _, ok := s.resetOnStatus[resp.StatusCode]; ok {
		s.mu.Lock()
		s.resetRequested = true
		s.mu.Unlock()
	}
	return resp, nil
}

// Reset forces the next request to rebuild the underlying client.
func (s *ResettableSession) Reset() {
	s.mu.Lock()
	s.resetRequested = true
	s.maybeResetLocked(s.proxyURL)
	s.mu.Unlock()
}

// Close tears down the current transport's idle connections.
func (s *ResettableSession) Close() {
	s.mu.Lock()
	client := s.client
	s.client = nil
	s.proxyURL = ""
	s.resetRequested = false
	s.mu.Unlock()
	closeIdleConnections(client)
}

func closeIdleConnections(client *http.Client) {
	if client == nil || client.Transport == nil {
		return
	}
	type idleCloser interface {
		CloseIdleConnections()
	}
	if closer, ok := client.Transport.(idleCloser); ok {
		closer.CloseIdleConnections()
	}
}
