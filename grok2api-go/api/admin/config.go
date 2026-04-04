package admin

import (
	"encoding/json"
	"fmt"
	"net/http"
	"reflect"
	"regexp"
	"strings"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/storage"
)

var cfgSanitizer = strings.NewReplacer(
	"\u2010", "-",
	"\u2011", "-",
	"\u2012", "-",
	"\u2013", "-",
	"\u2014", "-",
	"\u2212", "-",
	"\u2018", "'",
	"\u2019", "'",
	"\u201c", "\"",
	"\u201d", "\"",
	"\u00a0", " ",
	"\u2007", " ",
	"\u202f", " ",
	"\u200b", "",
	"\u200c", "",
	"\u200d", "",
	"\ufeff", "",
)

var whitespaceRE = regexp.MustCompile(`\s+`)

type Handler struct {
	cfg     *config.Config
	storage storage.Storage
}

func NewHandler(cfg *config.Config) *Handler {
	return &Handler{cfg: cfg, storage: storage.NewLocalStorage()}
}

func (h *Handler) Verify() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w, http.MethodGet)
			return
		}
		writeJSON(w, http.StatusOK, map[string]string{"status": "success"})
	})
}

func (h *Handler) Storage() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w, http.MethodGet)
			return
		}
		writeJSON(w, http.StatusOK, map[string]string{"type": "local"})
	})
}

func (h *Handler) Config() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			snapshot := configToMap(h.cfg)
			maskSecrets(snapshot)
			writeJSON(w, http.StatusOK, snapshot)
		case http.MethodPost, http.MethodPut:
			var payload map[string]any
			decoder := json.NewDecoder(r.Body)
			decoder.UseNumber()
			if err := decoder.Decode(&payload); err != nil {
				badRequest(w, "invalid JSON body")
				return
			}
			payload = sanitizeProxyConfigPayload(payload)

			effective := configToMap(h.cfg)
			masked := deepCopyMap(effective)
			maskSecrets(masked)
			restoreMaskedSecrets(payload, masked, effective)

			existingOverrides, err := h.storage.LoadConfig()
			if err != nil {
				internalServerError(w, err)
				return
			}
			merged := deepMergeMaps(existingOverrides, payload)
			if err := h.storage.SaveConfig(merged); err != nil {
				internalServerError(w, err)
				return
			}
			if err := h.cfg.Load(); err != nil {
				internalServerError(w, err)
				return
			}
			writeJSON(w, http.StatusOK, map[string]string{"status": "success", "message": "配置已更新"})
		default:
			methodNotAllowed(w, http.MethodGet, http.MethodPost, http.MethodPut)
		}
	})
}

func sanitizeProxyConfigPayload(data map[string]any) map[string]any {
	if data == nil {
		return nil
	}
	payload := deepCopyMap(data)
	proxyValue, ok := payload["proxy"]
	if !ok {
		return payload
	}
	proxy, ok := asStringMap(proxyValue)
	if !ok {
		return payload
	}
	if raw, ok := proxy["user_agent"]; ok {
		proxy["user_agent"] = sanitizeProxyText(raw, false)
	}
	if raw, ok := proxy["cf_cookies"]; ok {
		proxy["cf_cookies"] = sanitizeProxyText(raw, false)
	}
	if raw, ok := proxy["cf_clearance"]; ok {
		proxy["cf_clearance"] = sanitizeProxyText(raw, true)
	}
	payload["proxy"] = proxy
	return payload
}

func sanitizeProxyText(value any, removeAllSpaces bool) string {
	text := cfgSanitizer.Replace(toString(value))
	if removeAllSpaces {
		text = whitespaceRE.ReplaceAllString(text, "")
	} else {
		text = strings.TrimSpace(text)
	}
	return strings.Map(func(r rune) rune {
		if r > 255 {
			return -1
		}
		return r
	}, text)
}

func configToMap(cfg *config.Config) map[string]any {
	if cfg == nil {
		return map[string]any{}
	}
	result, ok := structToMap(reflect.ValueOf(cfg))
	if !ok {
		return map[string]any{}
	}
	return result
}

func structToMap(value reflect.Value) (map[string]any, bool) {
	value = indirect(value)
	if !value.IsValid() || value.Kind() != reflect.Struct {
		return nil, false
	}
	result := make(map[string]any)
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
		fieldValue := indirect(value.Field(i))
		if !fieldValue.IsValid() {
			continue
		}
		result[tag] = valueToAny(fieldValue)
	}
	return result, true
}

func valueToAny(value reflect.Value) any {
	value = indirect(value)
	if !value.IsValid() {
		return nil
	}
	switch value.Kind() {
	case reflect.Struct:
		mapped, _ := structToMap(value)
		return mapped
	case reflect.Slice, reflect.Array:
		length := value.Len()
		items := make([]any, 0, length)
		for i := 0; i < length; i++ {
			items = append(items, valueToAny(value.Index(i)))
		}
		return items
	default:
		return value.Interface()
	}
}

func indirect(value reflect.Value) reflect.Value {
	for value.IsValid() && (value.Kind() == reflect.Pointer || value.Kind() == reflect.Interface) {
		if value.IsNil() {
			return reflect.Value{}
		}
		value = value.Elem()
	}
	return value
}

func maskSecrets(data map[string]any) {
	maskSecretPath(data, []string{"app", "api_key"})
	maskSecretPath(data, []string{"app", "app_key"})
	maskSecretPath(data, []string{"app", "function_key"})
	maskSecretPath(data, []string{"proxy", "cf_clearance"})
	maskSecretPath(data, []string{"proxy", "cf_cookies"})
}

func maskSecretPath(data map[string]any, path []string) {
	if len(path) == 0 || data == nil {
		return
	}
	current := data
	for i := 0; i < len(path)-1; i++ {
		next, ok := asStringMap(current[path[i]])
		if !ok {
			return
		}
		current = next
	}
	key := path[len(path)-1]
	value, ok := current[key]
	if !ok {
		return
	}
	text := toString(value)
	if text == "" {
		return
	}
	current[key] = maskValue(text)
}

func restoreMaskedSecrets(payload, masked, actual map[string]any) {
	for key, value := range payload {
		actualValue, actualOK := actual[key]
		maskedValue, maskedOK := masked[key]

		payloadMap, payloadIsMap := asStringMap(value)
		actualMap, actualIsMap := asStringMap(actualValue)
		maskedMap, maskedIsMap := asStringMap(maskedValue)
		if payloadIsMap && actualIsMap && maskedIsMap {
			restoreMaskedSecrets(payloadMap, maskedMap, actualMap)
			payload[key] = payloadMap
			continue
		}

		if actualOK && maskedOK && toString(value) != "" && toString(value) == toString(maskedValue) {
			payload[key] = actualValue
		}
	}
}

func maskValue(value string) string {
	if value == "" {
		return ""
	}
	visible := value
	if len(visible) > 10 {
		visible = visible[:10]
	}
	if len(value) <= len(visible) {
		return visible
	}
	return visible + strings.Repeat("*", len(value)-len(visible))
}

func deepCopyMap(input map[string]any) map[string]any {
	if input == nil {
		return map[string]any{}
	}
	result := make(map[string]any, len(input))
	for key, value := range input {
		if nested, ok := asStringMap(value); ok {
			result[key] = deepCopyMap(nested)
			continue
		}
		result[key] = value
	}
	return result
}

func deepMergeMaps(base, override map[string]any) map[string]any {
	if base == nil {
		base = map[string]any{}
	}
	result := deepCopyMap(base)
	for key, value := range override {
		if nestedOverride, ok := asStringMap(value); ok {
			if nestedBase, ok := asStringMap(result[key]); ok {
				result[key] = deepMergeMaps(nestedBase, nestedOverride)
			} else {
				result[key] = deepCopyMap(nestedOverride)
			}
			continue
		}
		result[key] = value
	}
	return result
}

func asStringMap(value any) (map[string]any, bool) {
	mapped, ok := value.(map[string]any)
	if ok {
		return mapped, true
	}
	mappedInterface, ok := value.(map[string]interface{})
	if ok {
		converted := make(map[string]any, len(mappedInterface))
		for key, item := range mappedInterface {
			converted[key] = item
		}
		return converted, true
	}
	return nil, false
}

func toString(value any) string {
	if value == nil {
		return ""
	}
	switch typed := value.(type) {
	case string:
		return typed
	case json.Number:
		return typed.String()
	case fmt.Stringer:
		return typed.String()
	default:
		return strings.TrimSpace(fmt.Sprint(value))
	}
}
