package config

func (c *Config) String(key string) (string, bool) {
	value, ok := c.lookup(key)
	if !ok {
		return "", false
	}
	text, ok := value.(string)
	return text, ok
}

func (c *Config) Bool(key string) (bool, bool) {
	value, ok := c.lookup(key)
	if !ok {
		return false, false
	}
	flag, ok := value.(bool)
	return flag, ok
}

func (c *Config) Int(key string) (int, bool) {
	value, ok := c.lookup(key)
	if !ok {
		return 0, false
	}
	switch typed := value.(type) {
	case int:
		return typed, true
	case int8:
		return int(typed), true
	case int16:
		return int(typed), true
	case int32:
		return int(typed), true
	case int64:
		return int(typed), true
	case uint:
		return int(typed), true
	case uint8:
		return int(typed), true
	case uint16:
		return int(typed), true
	case uint32:
		return int(typed), true
	case uint64:
		return int(typed), true
	default:
		return 0, false
	}
}

func (c *Config) Float64(key string) (float64, bool) {
	value, ok := c.lookup(key)
	if !ok {
		return 0, false
	}
	switch typed := value.(type) {
	case float32:
		return float64(typed), true
	case float64:
		return typed, true
	case int:
		return float64(typed), true
	case int64:
		return float64(typed), true
	default:
		return 0, false
	}
}

func (c *Config) IntSlice(key string) ([]int, bool) {
	value, ok := c.lookup(key)
	if !ok {
		return nil, false
	}
	items, ok := value.([]int)
	if !ok {
		return nil, false
	}
	return append([]int(nil), items...), true
}
