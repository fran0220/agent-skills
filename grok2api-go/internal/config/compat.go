package config

import (
	"os"
	"strconv"
	"strings"
	"sync"
	"unicode"
)

var (
	currentMu sync.RWMutex
	current   *Config
)

func SetCurrent(cfg *Config) {
	currentMu.Lock()
	defer currentMu.Unlock()
	current = cfg
}

func GetBool(path string) bool {
	if cfg := currentConfig(); cfg != nil {
		if value, ok := cfg.lookup(path); ok {
			if typed, ok := value.(bool); ok {
				return typed
			}
		}
	}

	value, ok := os.LookupEnv(envKey(path))
	if !ok {
		return false
	}
	parsed, err := strconv.ParseBool(strings.TrimSpace(value))
	if err != nil {
		return false
	}
	return parsed
}

func GetInt(path string) int {
	if cfg := currentConfig(); cfg != nil {
		if value, ok := cfg.lookup(path); ok {
			switch typed := value.(type) {
			case int:
				return typed
			case int8:
				return int(typed)
			case int16:
				return int(typed)
			case int32:
				return int(typed)
			case int64:
				return int(typed)
			case uint:
				return int(typed)
			case uint8:
				return int(typed)
			case uint16:
				return int(typed)
			case uint32:
				return int(typed)
			case uint64:
				return int(typed)
			}
		}
	}

	value, ok := os.LookupEnv(envKey(path))
	if !ok {
		return 0
	}
	parsed, err := strconv.Atoi(strings.TrimSpace(value))
	if err != nil {
		return 0
	}
	return parsed
}

func currentConfig() *Config {
	currentMu.RLock()
	defer currentMu.RUnlock()
	return current
}

func envKey(path string) string {
	var builder strings.Builder
	builder.WriteString("GROK2API_")
	for _, r := range path {
		switch {
		case unicode.IsLetter(r), unicode.IsDigit(r):
			builder.WriteRune(unicode.ToUpper(r))
		default:
			builder.WriteByte('_')
		}
	}
	return builder.String()
}
