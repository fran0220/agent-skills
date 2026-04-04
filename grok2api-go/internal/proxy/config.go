package proxy

import "sync"

// ConfigProvider lets the proxy pool read config without depending on the
// still-in-progress config package.
type ConfigProvider interface {
	String(key string) (string, bool)
}

var poolConfig struct {
	sync.RWMutex
	provider ConfigProvider
}

// SetConfigProvider wires runtime config into the proxy pool.
func SetConfigProvider(provider ConfigProvider) {
	poolConfig.Lock()
	poolConfig.provider = provider
	poolConfig.Unlock()
}

func getConfigString(key, fallback string) string {
	poolConfig.RLock()
	provider := poolConfig.provider
	poolConfig.RUnlock()
	if provider == nil {
		return fallback
	}
	if value, ok := provider.String(key); ok {
		return value
	}
	return fallback
}
