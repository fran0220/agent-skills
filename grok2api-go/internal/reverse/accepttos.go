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

const AcceptTOSAPI = "https://accounts.x.ai/auth_mgmt.AuthManagement/SetTosAcceptedVersion"

type AcceptTOSReverse struct {
	API  string
	grpc GrpcClient
}

func NewAcceptTOSReverse() *AcceptTOSReverse {
	return &AcceptTOSReverse{API: AcceptTOSAPI}
}

func (r *AcceptTOSReverse) Request(ctx context.Context, session *ResettableSession, token string) (GrpcStatus, error) {
	headers := BuildHeaders(token, "application/json", "https://accounts.x.ai", "https://accounts.x.ai/accept-tos")
	headers["Content-Type"] = "application/grpc-web+proto"
	headers["Accept"] = "*/*"
	headers["Sec-Fetch-Dest"] = "empty"
	headers["x-grpc-web"] = "1"
	headers["x-user-agent"] = "connect-es/2.1.1"
	headers["Cache-Control"] = "no-cache"
	headers["Pragma"] = "no-cache"

	payload := r.grpc.EncodePayload([]byte{0x10, 0x01})
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
				Message:    fmt.Sprintf("accept tos request failed: %d", response.StatusCode),
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
			Message:    fmt.Sprintf("accept tos grpc failed: %d", status.Code),
			StatusCode: status.HTTPEquiv(),
			Body:       status.Message,
		}
	}
	return status, nil
}
