package admin

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"

	configpkg "github.com/fran0220/grok2api-go/internal/config"
)

type fileInfo = os.FileInfo

func stringValue(value any) string {
	switch typed := value.(type) {
	case string:
		return typed
	case fmt.Stringer:
		return typed.String()
	case nil:
		return ""
	default:
		return fmt.Sprint(value)
	}
}

func mapValue(value any) map[string]any {
	if mapped, ok := value.(map[string]any); ok {
		return mapped
	}
	return nil
}

func anySliceToMapSlice(value any) []map[string]any {
	items, ok := value.([]any)
	if !ok {
		return nil
	}
	result := make([]map[string]any, 0, len(items))
	for _, item := range items {
		if mapped := mapValue(item); mapped != nil {
			result = append(result, mapped)
		}
	}
	return result
}

func readDir(path string) []os.DirEntry {
	entries, err := os.ReadDir(path)
	if err != nil {
		return nil
	}
	return entries
}

func ensureDir(path string) error {
	return os.MkdirAll(path, 0o755)
}

func removeFile(path string) error {
	err := os.Remove(path)
	if os.IsNotExist(err) {
		return nil
	}
	return err
}

func reverseBody(resp *http.Response) string {
	if resp == nil || resp.Body == nil {
		return ""
	}
	defer resp.Body.Close()
	data, _ := io.ReadAll(io.LimitReader(resp.Body, 4096))
	return string(data)
}

func hardTimeout(key string) int {
	if value := configpkg.GetInt(key); value > 0 {
		return value
	}
	return 60
}

func toStringSlice(value any) []string {
	items, ok := value.([]any)
	if !ok {
		return nil
	}
	result := make([]string, 0, len(items))
	for _, item := range items {
		if text := strings.TrimSpace(stringValue(item)); text != "" {
			result = append(result, text)
		}
	}
	return result
}
