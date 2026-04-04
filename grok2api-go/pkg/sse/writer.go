package sse

import (
	"encoding/json"
	"net/http"
)

type errorPayloadProvider interface {
	SSEErrorPayload() map[string]any
}

// Writer wraps http.ResponseWriter for SSE responses.
type Writer struct {
	w       http.ResponseWriter
	flusher http.Flusher
}

func NewWriter(w http.ResponseWriter) *Writer {
	writer := &Writer{w: w}
	if flusher, ok := w.(http.Flusher); ok {
		writer.flusher = flusher
	}
	return writer
}

func (sw *Writer) SetHeaders() {
	if sw == nil || sw.w == nil {
		return
	}
	headers := sw.w.Header()
	headers.Set("Content-Type", "text/event-stream")
	headers.Set("Cache-Control", "no-cache")
	headers.Set("Connection", "keep-alive")
	headers.Set("X-Accel-Buffering", "no")
}

func (sw *Writer) WriteEvent(data string) error {
	if sw == nil || sw.w == nil {
		return nil
	}
	if _, err := sw.w.Write([]byte("data: " + data + "\n\n")); err != nil {
		return err
	}
	if sw.flusher != nil {
		sw.flusher.Flush()
	}
	return nil
}

func (sw *Writer) WriteDone() error {
	if sw == nil || sw.w == nil {
		return nil
	}
	if _, err := sw.w.Write([]byte("data: [DONE]\n\n")); err != nil {
		return err
	}
	if sw.flusher != nil {
		sw.flusher.Flush()
	}
	return nil
}

func (sw *Writer) WriteError(err error) error {
	if sw == nil || sw.w == nil {
		return nil
	}
	payload := map[string]any{
		"error": map[string]any{
			"message": "stream_error",
			"type":    "server_error",
			"code":    "stream_error",
		},
	}
	if err != nil {
		payload["error"].(map[string]any)["message"] = err.Error()
		if provider, ok := err.(errorPayloadProvider); ok {
			payload = provider.SSEErrorPayload()
		}
	}
	data, marshalErr := json.Marshal(payload)
	if marshalErr != nil {
		return marshalErr
	}
	if _, err := sw.w.Write([]byte("event: error\n")); err != nil {
		return err
	}
	if _, err := sw.w.Write([]byte("data: " + string(data) + "\n\n")); err != nil {
		return err
	}
	if sw.flusher != nil {
		sw.flusher.Flush()
	}
	return nil
}
