package reverse

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
)

const UploadAPI = "https://grok.com/rest/app-chat/upload-file"

type AssetsUploadReverse struct{}

func (AssetsUploadReverse) Request(ctx context.Context, session *ResettableSession, token, filename, mime, base64Data string) (string, string, error) {
	payload := map[string]string{
		"fileName":     filename,
		"fileMimeType": mime,
		"content":      base64Data,
	}
	body, err := json.Marshal(payload)
	if err != nil {
		return "", "", fmt.Errorf("marshal upload payload: %w", err)
	}
	headers := BuildHeaders(token, "application/json", "https://grok.com", "https://grok.com/")
	timeout := time.Duration(getConfigInt("asset.upload_timeout", 60)) * time.Second
	var activeProxyKey string

	resp, err := RetryOnStatus(ctx, func(callCtx context.Context) (*http.Response, error) {
		requestCtx := callCtx
		var cancel context.CancelFunc
		if timeout > 0 {
			requestCtx, cancel = context.WithTimeout(callCtx, timeout)
			defer cancel()
		}
		activeProxyKey, _ = proxycfg.GetCurrentProxyFrom("proxy.asset_proxy_url", "proxy.base_proxy_url")
		activeProxyURL := ""
		if activeProxyKey != "" {
			activeProxyURL = proxycfg.GetCurrentProxy(activeProxyKey)
		}
		request, err := http.NewRequestWithContext(requestCtx, http.MethodPost, UploadAPI, bytes.NewReader(body))
		if err != nil {
			return nil, err
		}
		for key, value := range headers {
			request.Header.Set(key, value)
		}
		response, err := session.DoWithProxy(request, activeProxyURL)
		if err != nil {
			return nil, err
		}
		if response.StatusCode != http.StatusOK {
			content := readResponseBody(response)
			return nil, &UpstreamError{
				Message:    fmt.Sprintf("upload request failed: %d", response.StatusCode),
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
		return "", "", err
	}
	defer resp.Body.Close()

	var result struct {
		FileMetadataID string `json:"fileMetadataId"`
		FileURI        string `json:"fileUri"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return "", "", fmt.Errorf("decode upload response: %w", err)
	}
	return result.FileMetadataID, result.FileURI, nil
}

func readResponseBody(resp *http.Response) string {
	if resp == nil || resp.Body == nil {
		return ""
	}
	defer resp.Body.Close()
	data, _ := io.ReadAll(io.LimitReader(resp.Body, 4096))
	return string(data)
}
