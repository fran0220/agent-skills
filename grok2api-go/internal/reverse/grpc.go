package reverse

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"net/url"
	"regexp"
	"strings"
)

var grpcBase64RE = regexp.MustCompile(`^[A-Za-z0-9+/=\r\n]+$`)

type GrpcStatus struct {
	Code    int
	Message string
}

func (s GrpcStatus) OK() bool {
	return s.Code == 0
}

func (s GrpcStatus) HTTPEquiv() int {
	switch s.Code {
	case 0:
		return http.StatusOK
	case 4:
		return http.StatusGatewayTimeout
	case 7:
		return http.StatusForbidden
	case 8:
		return http.StatusTooManyRequests
	case 14:
		return http.StatusServiceUnavailable
	case 16:
		return http.StatusUnauthorized
	default:
		return http.StatusBadGateway
	}
}

type GrpcClient struct{}

func (GrpcClient) EncodePayload(data []byte) []byte {
	frame := make([]byte, 5+len(data))
	frame[0] = 0x00
	length := len(data)
	frame[1] = byte(length >> 24)
	frame[2] = byte(length >> 16)
	frame[3] = byte(length >> 8)
	frame[4] = byte(length)
	copy(frame[5:], data)
	return frame
}

func (GrpcClient) maybeDecodeGRPCWebText(body []byte, contentType string) []byte {
	ct := strings.ToLower(strings.TrimSpace(contentType))
	if strings.Contains(ct, "grpc-web-text") {
		compact := strings.Join(strings.Fields(string(body)), "")
		decoded, err := base64.StdEncoding.DecodeString(compact)
		if err == nil {
			return decoded
		}
		return body
	}
	head := body
	if len(head) > 2048 {
		head = head[:2048]
	}
	if len(head) > 0 && grpcBase64RE.Match(head) {
		compact := strings.Join(strings.Fields(string(body)), "")
		decoded, err := base64.StdEncoding.DecodeString(compact)
		if err == nil {
			return decoded
		}
	}
	return body
}

func (GrpcClient) parseTrailerBlock(payload []byte) map[string]string {
	text := strings.ReplaceAll(string(payload), "\r\n", "\n")
	trailers := make(map[string]string)
	for _, line := range strings.Split(text, "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		parts := strings.SplitN(line, ":", 2)
		if len(parts) != 2 {
			continue
		}
		key := strings.ToLower(strings.TrimSpace(parts[0]))
		value := strings.TrimSpace(parts[1])
		if key == "grpc-message" {
			if decoded, err := url.PathUnescape(value); err == nil {
				value = decoded
			}
		}
		trailers[key] = value
	}
	return trailers
}

func (g GrpcClient) ParseResponse(body []byte, contentType string, headers http.Header) ([][]byte, map[string]string, error) {
	decoded := g.maybeDecodeGRPCWebText(body, contentType)
	messages := make([][]byte, 0)
	trailers := make(map[string]string)

	for index := 0; index < len(decoded); {
		if len(decoded)-index < 5 {
			break
		}
		flag := decoded[index]
		length := int(decoded[index+1])<<24 | int(decoded[index+2])<<16 | int(decoded[index+3])<<8 | int(decoded[index+4])
		index += 5
		if len(decoded)-index < length {
			break
		}
		payload := decoded[index : index+length]
		index += length

		switch {
		case flag&0x80 != 0:
			for key, value := range g.parseTrailerBlock(payload) {
				trailers[key] = value
			}
		case flag&0x01 != 0:
			return nil, nil, fmt.Errorf("grpc-web compressed flag not supported")
		default:
			messages = append(messages, append([]byte(nil), payload...))
		}
	}

	if headers != nil {
		if value := strings.TrimSpace(headers.Get("grpc-status")); value != "" && trailers["grpc-status"] == "" {
			trailers["grpc-status"] = value
		}
		if value := strings.TrimSpace(headers.Get("grpc-message")); value != "" && trailers["grpc-message"] == "" {
			if decoded, err := url.PathUnescape(value); err == nil {
				value = decoded
			}
			trailers["grpc-message"] = value
		}
	}

	status := g.GetStatus(trailers)
	if status.Code != 0 && status.Code != -1 {
		g.logErrorPayload(status, body, contentType, headers, trailers, messages)
	}

	return messages, trailers, nil
}

func (GrpcClient) GetStatus(trailers map[string]string) GrpcStatus {
	raw := strings.TrimSpace(trailers["grpc-status"])
	message := strings.TrimSpace(trailers["grpc-message"])
	code := -1
	if raw != "" {
		_, _ = fmt.Sscanf(raw, "%d", &code)
	}
	return GrpcStatus{Code: code, Message: message}
}

func (GrpcClient) safeHeaders(headers http.Header) map[string]string {
	if headers == nil {
		return map[string]string{}
	}
	safe := make(map[string]string, len(headers))
	for key, values := range headers {
		value := strings.Join(values, ", ")
		switch strings.ToLower(key) {
		case "set-cookie", "cookie", "authorization":
			safe[key] = "<redacted>"
		default:
			safe[key] = value
		}
	}
	return safe
}

func (g GrpcClient) logErrorPayload(status GrpcStatus, body []byte, contentType string, headers http.Header, trailers map[string]string, messages [][]byte) {
	payload := map[string]any{
		"grpc_status":  status.Code,
		"grpc_message": status.Message,
		"content_type": contentType,
		"headers":      g.safeHeaders(headers),
		"trailers":     trailers,
		"body_b64":     base64.StdEncoding.EncodeToString(body),
	}
	messageFrames := make([]string, 0, len(messages))
	for _, message := range messages {
		messageFrames = append(messageFrames, base64.StdEncoding.EncodeToString(message))
	}
	payload["messages_b64"] = messageFrames
	serialized, err := json.Marshal(payload)
	if err != nil {
		slog.Error("grpc response error payload marshal failed", "error", err)
		return
	}
	slog.Error("grpc response error", "payload", string(serialized))
}
