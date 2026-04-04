package reverse

import (
	"fmt"
	"net/http"
)

// UpstreamError captures a non-success upstream response or transport-mapped
// error so callers can reuse retry/status extraction logic.
type UpstreamError struct {
	Message        string
	StatusCode     int
	Body           string
	Headers        http.Header
	Cause          error
	RetryAfter     *float64
	IsTokenExpired bool
	IsCloudflare   bool
}

func (e *UpstreamError) Error() string {
	if e == nil {
		return "<nil>"
	}
	if e.Message != "" {
		return e.Message
	}
	if e.Cause != nil {
		return e.Cause.Error()
	}
	return fmt.Sprintf("upstream error: status=%d", e.StatusCode)
}

func (e *UpstreamError) Unwrap() error {
	if e == nil {
		return nil
	}
	return e.Cause
}
