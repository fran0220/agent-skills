package proxy

import (
	"log/slog"
	"strings"
	"sync"
)

var (
	lock                sync.Mutex
	pools               = map[string][]string{}
	indexes             = map[string]int{}
	rawCache            = map[string]string{}
	failoverStatusCodes = map[int]struct{}{403: {}, 429: {}, 502: {}}
)

func parseProxies(raw string) []string {
	if raw == "" {
		return nil
	}
	parts := strings.Split(raw, ",")
	proxies := make([]string, 0, len(parts))
	for _, part := range parts {
		part = strings.TrimSpace(part)
		if part != "" {
			proxies = append(proxies, part)
		}
	}
	return proxies
}

func ensurePool(configKey string) []string {
	raw := getConfigString(configKey, "")
	if raw != rawCache[configKey] {
		proxies := parseProxies(raw)
		pools[configKey] = proxies
		indexes[configKey] = 0
		rawCache[configKey] = raw
		if len(proxies) > 1 {
			slog.Info("proxy pool loaded", "config_key", configKey, "count", len(proxies))
		}
	}
	return pools[configKey]
}

// GetCurrentProxy returns the sticky proxy for the given config key.
func GetCurrentProxy(configKey string) string {
	lock.Lock()
	defer lock.Unlock()
	pool := ensurePool(configKey)
	if len(pool) == 0 {
		return ""
	}
	idx := indexes[configKey] % len(pool)
	indexes[configKey] = idx
	return pool[idx]
}

// GetCurrentProxyFrom returns the first configured sticky proxy from the keys.
func GetCurrentProxyFrom(configKeys ...string) (string, string) {
	for _, configKey := range configKeys {
		proxy := GetCurrentProxy(configKey)
		if proxy != "" {
			return configKey, proxy
		}
	}
	return "", ""
}

// RotateProxy advances to the next proxy for the config key.
func RotateProxy(configKey string) string {
	lock.Lock()
	defer lock.Unlock()
	pool := ensurePool(configKey)
	if len(pool) == 0 {
		return ""
	}
	if len(pool) == 1 {
		return pool[0]
	}
	next := (indexes[configKey] + 1) % len(pool)
	indexes[configKey] = next
	proxy := pool[next]
	slog.Warn("proxy rotated", "config_key", configKey, "index", next+1, "count", len(pool))
	return proxy
}

// ShouldRotateProxy reports whether the status should trigger failover.
func ShouldRotateProxy(statusCode int) bool {
	_, ok := failoverStatusCodes[statusCode]
	return ok
}

// BuildHTTPProxies returns curl-style proxy settings for compatibility helpers.
func BuildHTTPProxies(proxyURL string) map[string]string {
	if proxyURL == "" {
		return nil
	}
	return map[string]string{"http": proxyURL, "https": proxyURL}
}
