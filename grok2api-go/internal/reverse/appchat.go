package reverse

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"maps"
	"net/http"
	"strings"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
)

const ChatAPI = "https://grok.com/rest/app-chat/conversations/new"

// TokenFailureRecorder lets the chat reverse layer notify token management when
// a 401 happens, without hard-coupling to the token package during the rewrite.
type TokenFailureRecorder func(ctx context.Context, token string, status int, reason string) error

type AppChatReverse struct {
	API                string
	RecordTokenFailure TokenFailureRecorder
}

func NewAppChatReverse() *AppChatReverse {
	return &AppChatReverse{API: ChatAPI}
}

func (a *AppChatReverse) resolveCustomPersonality() string {
	value := getConfigString("app.custom_instruction", "")
	if strings.TrimSpace(value) == "" {
		return ""
	}
	return value
}

// BuildPayload constructs the Grok app-chat request body.
func (a *AppChatReverse) BuildPayload(message, model, mode string, fileAttachments []string, toolOverrides, modelConfigOverride, requestOverrides map[string]any) map[string]any {
	attachments := append([]string(nil), fileAttachments...)
	payload := map[string]any{
		"deviceEnvInfo": map[string]any{
			"darkModeEnabled":  false,
			"devicePixelRatio": 2,
			"screenHeight":     1329,
			"screenWidth":      2056,
			"viewportHeight":   1083,
			"viewportWidth":    2056,
		},
		"disableMemory":               getConfigBool("app.disable_memory", true),
		"disableSearch":               false,
		"disableSelfHarmShortCircuit": false,
		"disableTextFollowUps":        false,
		"enableImageGeneration":       true,
		"enableImageStreaming":        true,
		"enableSideBySide":            true,
		"fileAttachments":             attachments,
		"forceConcise":                false,
		"forceSideBySide":             false,
		"imageAttachments":            []string{},
		"imageGenerationCount":        2,
		"isAsyncChat":                 false,
		"isReasoning":                 false,
		"message":                     message,
		"returnImageBytes":            false,
		"returnRawGrokInXaiRequest":   false,
		"sendFinalMetadata":           true,
		"temporary":                   getConfigBool("app.temporary", true),
		"toolOverrides":               maps.Clone(toolOverrides),
	}
	if payload["toolOverrides"] == nil {
		payload["toolOverrides"] = map[string]any{}
	}

	responseMetadata := map[string]any{}
	if model != "" {
		payload["modelName"] = model
		payload["modelMode"] = mode
		responseMetadata["requestModelDetails"] = map[string]any{"modelId": model}
	}
	payload["responseMetadata"] = responseMetadata

	if model == "grok-420" {
		payload["enable420"] = true
	}
	if custom := a.resolveCustomPersonality(); custom != "" {
		payload["customPersonality"] = custom
	}
	if len(modelConfigOverride) > 0 {
		responseMetadata["modelConfigOverride"] = maps.Clone(modelConfigOverride)
	}
	for key, value := range requestOverridesCopy(requestOverrides) {
		if value != nil {
			payload[key] = value
		}
	}

	slog.Debug("app chat payload built", "model", payload["modelName"], "mode", payload["modelMode"], "message_len", len(message), "file_attachments", len(attachments))
	return payload
}

func requestOverridesCopy(overrides map[string]any) map[string]any {
	if len(overrides) == 0 {
		return map[string]any{}
	}
	return maps.Clone(overrides)
}

func (a *AppChatReverse) requestTimeout() time.Duration {
	timeout := getConfigFloat64("chat.timeout", 60)
	if timeout <= 0 {
		imageTimeout := getConfigFloat64("image.timeout", 0)
		videoTimeout := getConfigFloat64("video.timeout", 0)
		if imageTimeout > timeout {
			timeout = imageTimeout
		}
		if videoTimeout > timeout {
			timeout = videoTimeout
		}
	}
	if timeout <= 0 {
		return 0
	}
	return time.Duration(timeout * float64(time.Second))
}

func readErrorBody(response *http.Response) string {
	if response == nil || response.Body == nil {
		return ""
	}
	data, err := io.ReadAll(response.Body)
	if err != nil {
		return ""
	}
	return string(data)
}

// Request sends a streaming app-chat request and returns the upstream body.
func (a *AppChatReverse) Request(ctx context.Context, session *ResettableSession, token, message, model, mode string, fileAttachments []string, toolOverrides, modelConfigOverride, requestOverrides map[string]any) (io.ReadCloser, error) {
	payload := a.BuildPayload(message, model, mode, fileAttachments, toolOverrides, modelConfigOverride, requestOverrides)
	body, err := json.Marshal(payload)
	if err != nil {
		return nil, fmt.Errorf("marshal app-chat payload: %w", err)
	}
	headers := BuildHeaders(token, "application/json", "https://grok.com", "https://grok.com/")
	timeout := a.requestTimeout()

	var activeProxyKey string
	var streamCancel context.CancelFunc // held alive until the caller closes the body
	requestFn := func(callCtx context.Context) (*http.Response, error) {
		requestCtx := callCtx
		if timeout > 0 {
			// Do NOT defer cancel here — the streaming body needs the context
			// to stay alive. The caller is responsible for closing the body,
			// which will unblock any in-progress reads.
			requestCtx, streamCancel = context.WithTimeout(callCtx, timeout)
		}

		activeProxyKey, _ = proxycfg.GetCurrentProxyFrom("proxy.base_proxy_url")
		activeProxyURL := ""
		if activeProxyKey != "" {
			activeProxyURL = proxycfg.GetCurrentProxy(activeProxyKey)
		}

		req, err := http.NewRequestWithContext(requestCtx, http.MethodPost, a.API, bytes.NewReader(body))
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
			_ = resp.Body.Close()
			slog.Error("app-chat upstream failure", "status", resp.StatusCode, "content_type", resp.Header.Get("content-type"), "body", truncate(content, 500))
			return nil, &UpstreamError{
				Message:    fmt.Sprintf("app-chat request failed: %d", resp.StatusCode),
				StatusCode: resp.StatusCode,
				Body:       content,
				Headers:    resp.Header.Clone(),
			}
		}
		return resp, nil
	}

	resp, err := RetryOnStatus(ctx, requestFn, RetryOptions{
		ExtractStatus: func(err error) *int {
			status := ExtractStatusForRetry(err)
			if status != nil && *status == http.StatusTooManyRequests {
				return nil
			}
			return status
		},
		OnRetry: func(ctx context.Context, attempt, status int, err error, delay time.Duration) {
			if activeProxyKey != "" && proxycfg.ShouldRotateProxy(status) {
				proxycfg.RotateProxy(activeProxyKey)
			}
		},
	})
	if err != nil {
		var upstreamErr *UpstreamError
		if errors.As(err, &upstreamErr) && upstreamErr.StatusCode == http.StatusUnauthorized && a.RecordTokenFailure != nil {
			if recordErr := a.RecordTokenFailure(ctx, token, upstreamErr.StatusCode, "app_chat_auth_failed"); recordErr != nil {
				slog.Warn("record token failure failed", "error", recordErr)
			}
		}
		return nil, err
	}

	// Wrap the body so that canceling the timeout context happens when the
	// caller closes the body (after the stream has been fully consumed).
	wrappedBody := resp.Body
	if streamCancel != nil {
		wrappedBody = &cancelOnCloseReader{ReadCloser: resp.Body, cancel: streamCancel}
	}
	return wrappedBody, nil
}

// cancelOnCloseReader wraps an io.ReadCloser and calls a cancel function on Close.
type cancelOnCloseReader struct {
	io.ReadCloser
	cancel context.CancelFunc
}

func (r *cancelOnCloseReader) Close() error {
	err := r.ReadCloser.Close()
	if r.cancel != nil {
		r.cancel()
	}
	return err
}

func truncate(value string, limit int) string {
	if len(value) <= limit {
		return value
	}
	return value[:limit]
}
