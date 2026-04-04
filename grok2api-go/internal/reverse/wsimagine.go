package reverse

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"net"
	"regexp"
	"strings"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/websocket"
)

const WSImagineURL = "wss://grok.com/ws/imagine/listen"

type blockedError struct{}

func (blockedError) Error() string { return "blocked_no_final_image" }

type ImageEvent struct {
	Type      string `json:"type"`
	ImageID   string `json:"image_id,omitempty"`
	Ext       string `json:"ext,omitempty"`
	Stage     string `json:"stage,omitempty"`
	Blob      string `json:"blob,omitempty"`
	BlobSize  int    `json:"blob_size,omitempty"`
	URL       string `json:"url,omitempty"`
	IsFinal   bool   `json:"is_final,omitempty"`
	ErrorCode string `json:"error_code,omitempty"`
	ErrorMsg  string `json:"error,omitempty"`
	Status    int    `json:"status,omitempty"`
}

type ImagineWebSocketReverse struct {
	urlPattern *regexp.Regexp
	client     *WebSocketClient
}

func NewImagineWebSocketReverse() *ImagineWebSocketReverse {
	return &ImagineWebSocketReverse{
		urlPattern: regexp.MustCompile(`/images/([a-f0-9-]+)\.(png|jpg|jpeg)`),
		client:     NewWebSocketClient(""),
	}
}

func (r *ImagineWebSocketReverse) Stream(ctx context.Context, token, prompt, aspectRatio string, n int, enableNSFW bool, maxRetries int) <-chan ImageEvent {
	out := make(chan ImageEvent, 32)
	go func() {
		defer close(out)
		retries := max(1, maxRetries)
		parallelEnabled := getConfigBool("image.blocked_parallel_enabled", true)
		for attempt := 0; attempt < retries; attempt++ {
			items, err := r.collectOnce(ctx, token, prompt, aspectRatio, n, enableNSFW)
			if err == nil {
				emitEvents(ctx, out, items)
				return
			}
			if _, ok := err.(blockedError); !ok {
				emitEvents(ctx, out, []ImageEvent{{Type: "error", ErrorCode: "ws_stream_failed", ErrorMsg: err.Error()}})
				return
			}
			retriesLeft := retries - (attempt + 1)
			if retriesLeft > 0 && parallelEnabled {
				slog.Warn("websocket imagine blocked, launching parallel retries", "retries_left", retriesLeft)
				results := r.parallelCollect(ctx, token, prompt, aspectRatio, n, enableNSFW, retriesLeft)
				for _, result := range results {
					if hasFinalImage(result.items) {
						emitEvents(ctx, out, result.items)
						return
					}
				}
				emitEvents(ctx, out, []ImageEvent{{Type: "error", ErrorCode: "blocked", ErrorMsg: "blocked_no_final_image"}})
				return
			}
			if attempt+1 < retries {
				slog.Warn("websocket imagine blocked, retrying", "attempt", attempt+1, "retries", retries)
				continue
			}
			emitEvents(ctx, out, []ImageEvent{{Type: "error", ErrorCode: "blocked", ErrorMsg: "blocked_no_final_image"}})
			return
		}
	}()
	return out
}

type collectResult struct {
	items []ImageEvent
	err   error
}

func (r *ImagineWebSocketReverse) parallelCollect(ctx context.Context, token, prompt, aspectRatio string, n int, enableNSFW bool, attempts int) []collectResult {
	results := make([]collectResult, attempts)
	var wg sync.WaitGroup
	for i := 0; i < attempts; i++ {
		wg.Add(1)
		go func(index int) {
			defer wg.Done()
			items, err := r.collectOnce(ctx, token, prompt, aspectRatio, n, enableNSFW)
			results[index] = collectResult{items: items, err: err}
		}(i)
	}
	wg.Wait()
	return results
}

func emitEvents(ctx context.Context, out chan<- ImageEvent, items []ImageEvent) {
	for _, item := range items {
		select {
		case <-ctx.Done():
			return
		case out <- item:
		}
	}
}

func hasFinalImage(items []ImageEvent) bool {
	for _, item := range items {
		if item.Type == "image" && item.IsFinal {
			return true
		}
	}
	return false
}

func (r *ImagineWebSocketReverse) collectOnce(ctx context.Context, token, prompt, aspectRatio string, n int, enableNSFW bool) ([]ImageEvent, error) {
	timeout := time.Duration(getConfigInt("image.timeout", 60)) * time.Second
	streamTimeout := time.Duration(getConfigInt("image.stream_timeout", 60)) * time.Second
	finalTimeout := time.Duration(getConfigInt("image.final_timeout", 15)) * time.Second
	blockedGrace := time.Duration(getConfigInt("image.blocked_grace_seconds", 10)) * time.Second
	if blockedGrace < time.Second {
		blockedGrace = time.Second
	}
	if blockedGrace > finalTimeout {
		blockedGrace = finalTimeout
	}
	finalMinBytes := getConfigInt("image.final_min_bytes", 100000)
	mediumMinBytes := getConfigInt("image.medium_min_bytes", 30000)
	requestID := uuid.NewString()
	headers := BuildWSHeaders(token, "https://grok.com")

	conn, err := r.client.Connect(ctx, WSImagineURL, headers, timeout)
	if err != nil {
		if upstream, ok := err.(*UpstreamError); ok {
			code := "connection_failed"
			if upstream.StatusCode == 429 {
				code = "rate_limit_exceeded"
			}
			return []ImageEvent{{Type: "error", ErrorCode: code, ErrorMsg: upstream.Error(), Status: upstream.StatusCode}}, nil
		}
		return []ImageEvent{{Type: "error", ErrorCode: "connection_failed", ErrorMsg: err.Error()}}, nil
	}
	defer conn.Close()
	conn.SetReadLimit(64 << 20)

	message := r.buildRequestMessage(requestID, prompt, aspectRatio, enableNSFW)
	if err := conn.WriteJSON(message); err != nil {
		return []ImageEvent{{Type: "error", ErrorCode: "connection_failed", ErrorMsg: err.Error()}}, nil
	}

	items := make([]ImageEvent, 0, 16)
	finalIDs := map[string]bool{}
	completed := 0
	startTime := time.Now()
	lastActivity := startTime
	var mediumReceivedTime time.Time
	readWait := 5 * time.Second
	if streamTimeout > 0 && streamTimeout < readWait {
		readWait = streamTimeout
	}

	for time.Since(startTime) < timeout {
		select {
		case <-ctx.Done():
			return items, ctx.Err()
		default:
		}

		_ = conn.SetReadDeadline(time.Now().Add(readWait))
		messageType, payload, err := conn.ReadMessage()
		if err != nil {
			if isReadTimeout(err) {
				now := time.Now()
				if !mediumReceivedTime.IsZero() && completed == 0 && now.Sub(mediumReceivedTime) > blockedGrace {
					slog.Warn("imagine stream blocked suspected", "request_id", requestID, "blocked_grace", blockedGrace.String())
					return nil, blockedError{}
				}
				if completed > 0 && now.Sub(lastActivity) > 10*time.Second {
					break
				}
				continue
			}
			if websocket.IsCloseError(err, websocket.CloseNormalClosure, websocket.CloseGoingAway) {
				items = append(items, ImageEvent{Type: "error", ErrorCode: "ws_closed", ErrorMsg: err.Error()})
				return items, nil
			}
			return []ImageEvent{{Type: "error", ErrorCode: "connection_failed", ErrorMsg: err.Error()}}, nil
		}
		if messageType != websocket.TextMessage {
			continue
		}
		lastActivity = time.Now()

		var msg map[string]any
		if err := json.Unmarshal(payload, &msg); err != nil {
			continue
		}
		msgType := stringValue(msg["type"])
		switch msgType {
		case "image":
			info := r.classifyImage(stringValue(msg["url"]), stringValue(msg["blob"]), finalMinBytes, mediumMinBytes)
			if info == nil {
				continue
			}
			if info.Stage == "medium" && mediumReceivedTime.IsZero() {
				mediumReceivedTime = time.Now()
			}
			if info.IsFinal && !finalIDs[info.ImageID] {
				finalIDs[info.ImageID] = true
				completed++
			}
			items = append(items, *info)
		case "error":
			items = append(items, ImageEvent{Type: "error", ErrorCode: stringValue(msg["err_code"]), ErrorMsg: stringValue(msg["err_msg"])})
			return items, nil
		}

		if completed >= n {
			break
		}
		if !mediumReceivedTime.IsZero() && completed == 0 && time.Since(mediumReceivedTime) > finalTimeout {
			slog.Warn("imagine stream final timeout suspected block", "request_id", requestID, "final_timeout", finalTimeout.String())
			return nil, blockedError{}
		}
	}

	return items, nil
}

func (r *ImagineWebSocketReverse) buildRequestMessage(requestID, prompt, aspectRatio string, enableNSFW bool) map[string]any {
	return map[string]any{
		"type":      "conversation.item.create",
		"timestamp": time.Now().UnixMilli(),
		"item": map[string]any{
			"type": "message",
			"content": []map[string]any{{
				"requestId": requestID,
				"text":      prompt,
				"type":      "input_text",
				"properties": map[string]any{
					"section_count":  0,
					"is_kids_mode":   false,
					"enable_nsfw":    enableNSFW,
					"skip_upsampler": false,
					"is_initial":     false,
					"aspect_ratio":   aspectRatio,
				},
			}},
		},
	}
}

func (r *ImagineWebSocketReverse) parseImageURL(rawURL string) (string, string) {
	match := r.urlPattern.FindStringSubmatch(rawURL)
	if len(match) != 3 {
		return "", ""
	}
	return match[1], strings.ToLower(match[2])
}

func (r *ImagineWebSocketReverse) classifyImage(rawURL, blob string, finalMinBytes, mediumMinBytes int) *ImageEvent {
	if rawURL == "" || blob == "" {
		return nil
	}
	imageID, ext := r.parseImageURL(rawURL)
	if imageID == "" {
		imageID = uuid.NewString()
	}
	blobSize := len(blob)
	isFinal := blobSize >= finalMinBytes
	stage := "preview"
	if isFinal {
		stage = "final"
	} else if blobSize >= mediumMinBytes {
		stage = "medium"
	}
	return &ImageEvent{
		Type:     "image",
		ImageID:  imageID,
		Ext:      ext,
		Stage:    stage,
		Blob:     blob,
		BlobSize: blobSize,
		URL:      rawURL,
		IsFinal:  isFinal,
	}
}

func stringValue(value any) string {
	switch typed := value.(type) {
	case string:
		return typed
	case fmt.Stringer:
		return typed.String()
	default:
		return ""
	}
}

func isReadTimeout(err error) bool {
	var netErr net.Error
	return errors.As(err, &netErr) && netErr.Timeout()
}
