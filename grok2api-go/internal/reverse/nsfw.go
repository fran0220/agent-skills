package reverse

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"net/http"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
)

const NSFWMgmtAPI = "https://grok.com/auth_mgmt.AuthManagement/UpdateUserFeatureControls"

type NsfwMgmtReverse struct {
	API  string
	grpc GrpcClient
}

func NewNsfwMgmtReverse() *NsfwMgmtReverse {
	return &NsfwMgmtReverse{API: NSFWMgmtAPI}
}

func (r *NsfwMgmtReverse) Request(ctx context.Context, session *ResettableSession, token string) (GrpcStatus, error) {
	headers := BuildHeaders(token, "application/json", "https://grok.com", "https://grok.com/?_s=data")
	headers["Content-Type"] = "application/grpc-web+proto"
	headers["Accept"] = "*/*"
	headers["Sec-Fetch-Dest"] = "empty"
	headers["x-grpc-web"] = "1"
	headers["x-user-agent"] = "connect-es/2.1.1"
	headers["Cache-Control"] = "no-cache"
	headers["Pragma"] = "no-cache"

	name := []byte("always_show_nsfw_content")
	inner := append([]byte{0x0a, byte(len(name))}, name...)
	protobuf := append([]byte{0x0a, 0x02, 0x10, 0x01, 0x12, byte(len(inner))}, inner...)
	payload := r.grpc.EncodePayload(protobuf)
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
		req, err := http.NewRequestWithContext(requestCtx, http.MethodPost, r.API, bytes.NewReader(payload))
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
				Message:    fmt.Sprintf("nsfw request failed: %d", response.StatusCode),
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
		return GrpcStatus{}, err
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return GrpcStatus{}, err
	}
	_, trailers, err := r.grpc.ParseResponse(body, resp.Header.Get("content-type"), resp.Header)
	if err != nil {
		return GrpcStatus{}, err
	}
	status := r.grpc.GetStatus(trailers)
	if status.Code != -1 && status.Code != 0 {
		return GrpcStatus{}, &UpstreamError{
			Message:    fmt.Sprintf("nsfw grpc failed: %d", status.Code),
			StatusCode: status.HTTPEquiv(),
			Body:       status.Message,
		}
	}
	return status, nil
}
