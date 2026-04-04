package config

import (
	"fmt"
	"math"
	"reflect"
	"strconv"
	"strings"
	"sync"

	"github.com/BurntSushi/toml"
)

type Config struct {
	mu     sync.RWMutex `toml:"-"`
	loadMu sync.Mutex   `toml:"-"`
	loaded bool         `toml:"-"`
	paths  Paths        `toml:"-"`

	App         AppConfig         `toml:"app"`
	Proxy       ProxyConfig       `toml:"proxy"`
	Retry       RetryConfig       `toml:"retry"`
	Token       TokenConfig       `toml:"token"`
	Log         LogConfig         `toml:"log"`
	Cache       CacheConfig       `toml:"cache"`
	Chat        ChatConfig        `toml:"chat"`
	Image       ImageConfig       `toml:"image"`
	ImagineFast ImagineFastConfig `toml:"imagine_fast"`
	Video       VideoConfig       `toml:"video"`
	Voice       VoiceConfig       `toml:"voice"`
	Asset       AssetConfig       `toml:"asset"`
	NSFW        NSFWConfig        `toml:"nsfw"`
	Usage       UsageConfig       `toml:"usage"`
}

func (c *Config) Load() error {
	c.loadMu.Lock()
	defer c.loadMu.Unlock()

	next := &Config{paths: c.paths}

	if !fileExists(c.paths.Defaults) {
		return fmt.Errorf("default config file not found: %s", c.paths.Defaults)
	}
	if _, err := toml.DecodeFile(c.paths.Defaults, next); err != nil {
		return fmt.Errorf("decode defaults: %w", err)
	}
	if fileExists(c.paths.Override) {
		if err := decodeOverrideFile(c.paths.Override, next); err != nil {
			return fmt.Errorf("decode override: %w", err)
		}
	}

	c.mu.Lock()
	defer c.mu.Unlock()
	c.copyFrom(next)
	c.loaded = true
	return nil
}

func decodeOverrideFile(path string, target *Config) error {
	raw := map[string]any{}
	if _, err := toml.DecodeFile(path, &raw); err != nil {
		return err
	}
	return applyOverrideMap(reflect.ValueOf(target).Elem(), raw, "")
}

func applyOverrideMap(dst reflect.Value, raw map[string]any, prefix string) error {
	for key, value := range raw {
		field, ok := structFieldByTOMLTag(dst, key)
		if !ok {
			continue
		}
		path := key
		if prefix != "" {
			path = prefix + "." + key
		}
		if err := setOverrideValue(field, value, path); err != nil {
			return err
		}
	}
	return nil
}

func setOverrideValue(dst reflect.Value, raw any, path string) error {
	if !dst.CanSet() {
		return nil
	}

	dst = indirectValue(dst)
	if !dst.IsValid() {
		return nil
	}

	switch dst.Kind() {
	case reflect.Struct:
		nested, ok := raw.(map[string]any)
		if !ok {
			return fmt.Errorf("%s: expected table, got %T", path, raw)
		}
		return applyOverrideMap(dst, nested, path)
	case reflect.String:
		text, ok := raw.(string)
		if !ok {
			return fmt.Errorf("%s: expected string, got %T", path, raw)
		}
		dst.SetString(text)
		return nil
	case reflect.Bool:
		value, err := coerceBool(raw)
		if err != nil {
			return fmt.Errorf("%s: %w", path, err)
		}
		dst.SetBool(value)
		return nil
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		value, err := coerceInt64(raw)
		if err != nil {
			return fmt.Errorf("%s: %w", path, err)
		}
		if dst.OverflowInt(value) {
			return fmt.Errorf("%s: value %d overflows %s", path, value, dst.Type())
		}
		dst.SetInt(value)
		return nil
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
		value, err := coerceUint64(raw)
		if err != nil {
			return fmt.Errorf("%s: %w", path, err)
		}
		if dst.OverflowUint(value) {
			return fmt.Errorf("%s: value %d overflows %s", path, value, dst.Type())
		}
		dst.SetUint(value)
		return nil
	case reflect.Float32, reflect.Float64:
		value, err := coerceFloat64(raw)
		if err != nil {
			return fmt.Errorf("%s: %w", path, err)
		}
		if dst.OverflowFloat(value) {
			return fmt.Errorf("%s: value %v overflows %s", path, value, dst.Type())
		}
		dst.SetFloat(value)
		return nil
	case reflect.Slice:
		rv := reflect.ValueOf(raw)
		if !rv.IsValid() || rv.Kind() != reflect.Slice {
			return fmt.Errorf("%s: expected array, got %T", path, raw)
		}
		slice := reflect.MakeSlice(dst.Type(), rv.Len(), rv.Len())
		for i := 0; i < rv.Len(); i++ {
			if err := setOverrideValue(slice.Index(i), rv.Index(i).Interface(), fmt.Sprintf("%s[%d]", path, i)); err != nil {
				return err
			}
		}
		dst.Set(slice)
		return nil
	default:
		value := reflect.ValueOf(raw)
		if value.IsValid() && value.Type().AssignableTo(dst.Type()) {
			dst.Set(value)
			return nil
		}
		return fmt.Errorf("%s: unsupported destination type %s", path, dst.Type())
	}
}

func coerceBool(raw any) (bool, error) {
	switch value := raw.(type) {
	case bool:
		return value, nil
	case string:
		parsed, err := strconv.ParseBool(strings.TrimSpace(value))
		if err != nil {
			return false, fmt.Errorf("expected bool, got %q", value)
		}
		return parsed, nil
	default:
		return false, fmt.Errorf("expected bool, got %T", raw)
	}
}

func coerceInt64(raw any) (int64, error) {
	switch value := raw.(type) {
	case int:
		return int64(value), nil
	case int8:
		return int64(value), nil
	case int16:
		return int64(value), nil
	case int32:
		return int64(value), nil
	case int64:
		return value, nil
	case uint:
		return int64(value), nil
	case uint8:
		return int64(value), nil
	case uint16:
		return int64(value), nil
	case uint32:
		return int64(value), nil
	case uint64:
		if value > math.MaxInt64 {
			return 0, fmt.Errorf("expected int, got %d", value)
		}
		return int64(value), nil
	case float32:
		return coerceInt64(float64(value))
	case float64:
		if value != math.Trunc(value) {
			return 0, fmt.Errorf("expected whole number, got %v", value)
		}
		if value < math.MinInt64 || value > math.MaxInt64 {
			return 0, fmt.Errorf("expected int, got %v", value)
		}
		return int64(value), nil
	case string:
		parsed, err := strconv.ParseInt(strings.TrimSpace(value), 10, 64)
		if err != nil {
			return 0, fmt.Errorf("expected int, got %q", value)
		}
		return parsed, nil
	default:
		return 0, fmt.Errorf("expected int, got %T", raw)
	}
}

func coerceUint64(raw any) (uint64, error) {
	switch value := raw.(type) {
	case int:
		if value < 0 {
			return 0, fmt.Errorf("expected unsigned int, got %d", value)
		}
		return uint64(value), nil
	case int8:
		if value < 0 {
			return 0, fmt.Errorf("expected unsigned int, got %d", value)
		}
		return uint64(value), nil
	case int16:
		if value < 0 {
			return 0, fmt.Errorf("expected unsigned int, got %d", value)
		}
		return uint64(value), nil
	case int32:
		if value < 0 {
			return 0, fmt.Errorf("expected unsigned int, got %d", value)
		}
		return uint64(value), nil
	case int64:
		if value < 0 {
			return 0, fmt.Errorf("expected unsigned int, got %d", value)
		}
		return uint64(value), nil
	case uint:
		return uint64(value), nil
	case uint8:
		return uint64(value), nil
	case uint16:
		return uint64(value), nil
	case uint32:
		return uint64(value), nil
	case uint64:
		return value, nil
	case float32:
		return coerceUint64(float64(value))
	case float64:
		if value != math.Trunc(value) || value < 0 {
			return 0, fmt.Errorf("expected unsigned whole number, got %v", value)
		}
		if value > math.MaxUint64 {
			return 0, fmt.Errorf("expected unsigned int, got %v", value)
		}
		return uint64(value), nil
	case string:
		parsed, err := strconv.ParseUint(strings.TrimSpace(value), 10, 64)
		if err != nil {
			return 0, fmt.Errorf("expected unsigned int, got %q", value)
		}
		return parsed, nil
	default:
		return 0, fmt.Errorf("expected unsigned int, got %T", raw)
	}
}

func coerceFloat64(raw any) (float64, error) {
	switch value := raw.(type) {
	case float32:
		return float64(value), nil
	case float64:
		return value, nil
	case int:
		return float64(value), nil
	case int8:
		return float64(value), nil
	case int16:
		return float64(value), nil
	case int32:
		return float64(value), nil
	case int64:
		return float64(value), nil
	case uint:
		return float64(value), nil
	case uint8:
		return float64(value), nil
	case uint16:
		return float64(value), nil
	case uint32:
		return float64(value), nil
	case uint64:
		return float64(value), nil
	case string:
		parsed, err := strconv.ParseFloat(strings.TrimSpace(value), 64)
		if err != nil {
			return 0, fmt.Errorf("expected float, got %q", value)
		}
		return parsed, nil
	default:
		return 0, fmt.Errorf("expected float, got %T", raw)
	}
}

func (c *Config) EnsureLoaded() error {
	c.mu.RLock()
	loaded := c.loaded
	c.mu.RUnlock()
	if loaded {
		return nil
	}
	return c.Load()
}

func (c *Config) GetString(key string, fallback ...string) string {
	value, ok := c.lookup(key)
	if !ok {
		return stringFallback(fallback)
	}
	switch typed := value.(type) {
	case string:
		return typed
	case fmt.Stringer:
		return typed.String()
	default:
		return stringFallback(fallback)
	}
}

func (c *Config) GetInt(key string, fallback ...int) int {
	value, ok := c.lookup(key)
	if !ok {
		return intFallback(fallback)
	}
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
	default:
		return intFallback(fallback)
	}
}

func (c *Config) GetIntSlice(key string, fallback ...[]int) []int {
	value, ok := c.lookup(key)
	if !ok {
		return intSliceFallback(fallback)
	}
	slice, ok := value.([]int)
	if !ok {
		return intSliceFallback(fallback)
	}
	return append([]int(nil), slice...)
}

func (c *Config) GetFloat(key string, fallback ...float64) float64 {
	value, ok := c.lookup(key)
	if !ok {
		return floatFallback(fallback)
	}
	switch typed := value.(type) {
	case float32:
		return float64(typed)
	case float64:
		return typed
	case int:
		return float64(typed)
	case int64:
		return float64(typed)
	default:
		return floatFallback(fallback)
	}
}

func (c *Config) GetBool(key string, fallback ...bool) bool {
	value, ok := c.lookup(key)
	if !ok {
		return boolFallback(fallback)
	}
	typed, ok := value.(bool)
	if !ok {
		return boolFallback(fallback)
	}
	return typed
}

func (c *Config) Paths() Paths {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.paths
}

func (c *Config) lookup(key string) (any, bool) {
	if err := c.EnsureLoaded(); err != nil {
		return nil, false
	}
	parts := strings.Split(key, ".")
	if len(parts) == 0 {
		return nil, false
	}

	c.mu.RLock()
	defer c.mu.RUnlock()

	current := reflect.ValueOf(c).Elem()
	for _, part := range parts {
		current = indirectValue(current)
		if !current.IsValid() {
			return nil, false
		}
		if current.Kind() != reflect.Struct {
			return nil, false
		}
		next, ok := structFieldByTOMLTag(current, part)
		if !ok {
			return nil, false
		}
		current = next
	}

	current = indirectValue(current)
	if !current.IsValid() {
		return nil, false
	}
	return current.Interface(), true
}

func structFieldByTOMLTag(value reflect.Value, part string) (reflect.Value, bool) {
	valueType := value.Type()
	for i := 0; i < value.NumField(); i++ {
		fieldType := valueType.Field(i)
		if !fieldType.IsExported() {
			continue
		}
		tag := strings.Split(fieldType.Tag.Get("toml"), ",")[0]
		if tag == "-" {
			continue
		}
		if tag == "" {
			tag = strings.ToLower(fieldType.Name)
		}
		if tag == part {
			return value.Field(i), true
		}
	}
	return reflect.Value{}, false
}

func indirectValue(value reflect.Value) reflect.Value {
	for value.IsValid() && (value.Kind() == reflect.Pointer || value.Kind() == reflect.Interface) {
		if value.IsNil() {
			return reflect.Value{}
		}
		value = value.Elem()
	}
	return value
}

func (c *Config) copyFrom(other *Config) {
	c.App = other.App
	c.Proxy = other.Proxy
	c.Retry = other.Retry
	c.Token = other.Token
	c.Log = other.Log
	c.Cache = other.Cache
	c.Chat = other.Chat
	c.Image = other.Image
	c.ImagineFast = other.ImagineFast
	c.Video = other.Video
	c.Voice = other.Voice
	c.Asset = other.Asset
	c.NSFW = other.NSFW
	c.Usage = other.Usage
	c.paths = other.paths
}

func stringFallback(values []string) string {
	if len(values) > 0 {
		return values[0]
	}
	return ""
}

func intFallback(values []int) int {
	if len(values) > 0 {
		return values[0]
	}
	return 0
}

func floatFallback(values []float64) float64 {
	if len(values) > 0 {
		return values[0]
	}
	return 0
}

func boolFallback(values []bool) bool {
	if len(values) > 0 {
		return values[0]
	}
	return false
}

func intSliceFallback(values [][]int) []int {
	if len(values) > 0 {
		return append([]int(nil), values[0]...)
	}
	return nil
}
