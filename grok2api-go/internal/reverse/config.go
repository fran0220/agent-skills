package reverse

import "sync"

// ConfigProvider lets the reverse layer stay decoupled while the config package
// is being implemented in a parallel thread.
type ConfigProvider interface {
	String(key string) (string, bool)
	Bool(key string) (bool, bool)
	Int(key string) (int, bool)
	Float64(key string) (float64, bool)
	IntSlice(key string) ([]int, bool)
}

var reverseConfig struct {
	sync.RWMutex
	provider ConfigProvider
}

// SetConfigProvider wires runtime configuration into the reverse package.
func SetConfigProvider(provider ConfigProvider) {
	reverseConfig.Lock()
	reverseConfig.provider = provider
	reverseConfig.Unlock()
}

func getConfigString(key, fallback string) string {
	reverseConfig.RLock()
	provider := reverseConfig.provider
	reverseConfig.RUnlock()
	if provider == nil {
		return fallback
	}
	if value, ok := provider.String(key); ok {
		return value
	}
	return fallback
}

func getConfigBool(key string, fallback bool) bool {
	reverseConfig.RLock()
	provider := reverseConfig.provider
	reverseConfig.RUnlock()
	if provider == nil {
		return fallback
	}
	if value, ok := provider.Bool(key); ok {
		return value
	}
	return fallback
}

func getConfigInt(key string, fallback int) int {
	reverseConfig.RLock()
	provider := reverseConfig.provider
	reverseConfig.RUnlock()
	if provider == nil {
		return fallback
	}
	if value, ok := provider.Int(key); ok {
		return value
	}
	return fallback
}

func getConfigFloat64(key string, fallback float64) float64 {
	reverseConfig.RLock()
	provider := reverseConfig.provider
	reverseConfig.RUnlock()
	if provider == nil {
		return fallback
	}
	if value, ok := provider.Float64(key); ok {
		return value
	}
	return fallback
}

func getConfigIntSlice(key string, fallback []int) []int {
	reverseConfig.RLock()
	provider := reverseConfig.provider
	reverseConfig.RUnlock()
	if provider == nil {
		return append([]int(nil), fallback...)
	}
	if value, ok := provider.IntSlice(key); ok {
		return append([]int(nil), value...)
	}
	return append([]int(nil), fallback...)
}
