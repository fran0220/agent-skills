package admin

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/fran0220/grok2api-go/internal/service"
	"github.com/fran0220/grok2api-go/internal/storage"
	"github.com/fran0220/grok2api-go/internal/token"
)

var tokenSanitizer = strings.NewReplacer(
	"\u2010", "-",
	"\u2011", "-",
	"\u2012", "-",
	"\u2013", "-",
	"\u2014", "-",
	"\u2212", "-",
	"\u00a0", " ",
	"\u2007", " ",
	"\u202f", " ",
	"\u200b", "",
	"\u200c", "",
	"\u200d", "",
	"\ufeff", "",
)

type addTokensRequest struct {
	Pool   string   `json:"pool"`
	Tokens []string `json:"tokens"`
}

type deleteTokensRequest struct {
	Tokens []string `json:"tokens"`
}

type refreshTokensRequest struct {
	Token  string   `json:"token"`
	Tokens []string `json:"tokens"`
}

type moveTokenRequest struct {
	Token    string `json:"token"`
	FromPool string `json:"from_pool"`
	ToPool   string `json:"to_pool"`
}

type tokenRefreshState struct {
	Status     token.TokenStatus
	Quota      int
	Consumed   int
	LastSyncAt int64
	Exists     bool
}

func (h *Handler) Tokens() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case http.MethodGet:
			manager := token.GetInstance()
			result := manager.ListAll()
			ensurePool(result, token.BasicPoolName)
			ensurePool(result, token.SuperPoolName)
			writeJSON(w, http.StatusOK, map[string]any{
				"tokens":                result,
				"consumed_mode_enabled": h.cfg.GetBool("token.consumed_mode_enabled"),
			})
		case http.MethodPost:
			h.handlePostTokens(w, r)
		case http.MethodDelete:
			h.handleDeleteTokens(w, r)
		default:
			methodNotAllowed(w, http.MethodGet, http.MethodPost, http.MethodDelete)
		}
	})
}

func (h *Handler) RefreshTokens() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}

		var req refreshTokensRequest
		if err := decodeOptionalJSON(r, &req); err != nil {
			badRequest(w, "invalid JSON body")
			return
		}

		requested := collectUniqueTokens(req.Token, req.Tokens)
		manager := token.GetInstance()
		before := snapshotTokenStates(manager.ListAll())
		summary := manager.RefreshCoolingTokens("admin", 0)
		after := snapshotTokenStates(manager.ListAll())

		results := make(map[string]bool, len(requested))
		for _, item := range requested {
			results[item] = tokenStateChanged(before[item], after[item])
		}

		response := map[string]any{
			"status":  "success",
			"summary": summary,
			"results": results,
		}
		if summary["checked"] == 0 {
			response["warning"] = "no cooling tokens were refreshed"
		}
		writeJSON(w, http.StatusOK, response)
	})
}

func (h *Handler) MoveToken() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}

		var req moveTokenRequest
		decoder := json.NewDecoder(r.Body)
		decoder.UseNumber()
		if err := decoder.Decode(&req); err != nil {
			badRequest(w, "invalid JSON body")
			return
		}

		tokenValue := sanitizeTokenText(req.Token)
		toPool := normalizePoolName(req.ToPool)
		fromPool := normalizePoolName(req.FromPool)
		if tokenValue == "" {
			badRequest(w, "token is required")
			return
		}
		if toPool == "" {
			badRequest(w, "to_pool is required")
			return
		}

		current, err := h.storage.LoadTokens()
		if err != nil {
			internalServerError(w, err)
			return
		}

		current, moved, err := moveStoredToken(current, tokenValue, fromPool, toPool)
		if err != nil {
			badRequest(w, err.Error())
			return
		}
		if !moved {
			badRequest(w, "token not found")
			return
		}
		if err := h.storage.SaveTokens(current); err != nil {
			internalServerError(w, err)
			return
		}
		if err := token.GetInstance().Reload(); err != nil {
			internalServerError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, map[string]string{"status": "success", "message": "Token 已移动"})
	})
}

func (h *Handler) handlePostTokens(w http.ResponseWriter, r *http.Request) {
	var raw map[string]json.RawMessage
	decoder := json.NewDecoder(r.Body)
	decoder.UseNumber()
	if err := decoder.Decode(&raw); err != nil {
		badRequest(w, "invalid JSON body")
		return
	}

	if _, ok := raw["pool"]; ok {
		h.handleAddTokens(w, raw)
		return
	}
	h.handleReplaceTokens(w, raw)
}

func (h *Handler) handleAddTokens(w http.ResponseWriter, raw map[string]json.RawMessage) {
	var req addTokensRequest
	payload, err := json.Marshal(raw)
	if err != nil {
		internalServerError(w, err)
		return
	}
	if err := json.Unmarshal(payload, &req); err != nil {
		badRequest(w, "invalid token payload")
		return
	}
	poolName := normalizePoolName(req.Pool)
	if poolName == "" {
		poolName = token.BasicPoolName
	}

	current, err := h.storage.LoadTokens()
	if err != nil {
		internalServerError(w, err)
		return
	}
	ensureStoredPool(current, poolName)
	index := existingTokenIndex(current)

	added := 0
	for _, item := range req.Tokens {
		normalized := sanitizeTokenText(item)
		if normalized == "" {
			continue
		}
		if _, exists := findExistingToken(index, normalized); exists {
			continue
		}
		info := token.NewTokenInfo(normalized, defaultQuotaForPool(poolName))
		data, err := tokenInfoToData(info)
		if err != nil {
			internalServerError(w, err)
			return
		}
		current[poolName] = append(current[poolName], data)
		added++
	}

	if err := h.storage.SaveTokens(current); err != nil {
		internalServerError(w, err)
		return
	}
	if err := token.GetInstance().Reload(); err != nil {
		internalServerError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"status": "success", "added": added})
}

func (h *Handler) handleReplaceTokens(w http.ResponseWriter, raw map[string]json.RawMessage) {
	current, err := h.storage.LoadTokens()
	if err != nil {
		internalServerError(w, err)
		return
	}
	index := existingTokenIndex(current)

	normalized := make(map[string][]storage.TokenData, len(raw))
	for poolName, payload := range raw {
		normalizedPool := normalizePoolName(poolName)
		if normalizedPool == "" {
			normalizedPool = poolName
		}
		var items []any
		decoder := json.NewDecoder(strings.NewReader(string(payload)))
		decoder.UseNumber()
		if err := decoder.Decode(&items); err != nil {
			badRequest(w, "invalid token list payload")
			return
		}

		poolItems := make([]storage.TokenData, 0, len(items))
		for _, item := range items {
			candidate, err := buildTokenData(item, normalizedPool, index)
			if err != nil {
				continue
			}
			poolItems = append(poolItems, candidate)
		}
		normalized[normalizedPool] = poolItems
	}
	ensureStoredPool(normalized, token.BasicPoolName)
	ensureStoredPool(normalized, token.SuperPoolName)

	if err := h.storage.SaveTokens(normalized); err != nil {
		internalServerError(w, err)
		return
	}
	if err := token.GetInstance().Reload(); err != nil {
		internalServerError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]string{"status": "success", "message": "Token 已更新"})
}

func (h *Handler) handleDeleteTokens(w http.ResponseWriter, r *http.Request) {
	var req deleteTokensRequest
	if err := decodeOptionalJSON(r, &req); err != nil {
		badRequest(w, "invalid JSON body")
		return
	}

	removeSet := make(map[string]struct{}, len(req.Tokens))
	for _, item := range req.Tokens {
		normalized := sanitizeTokenText(item)
		if normalized != "" {
			removeSet[normalized] = struct{}{}
		}
	}
	if len(removeSet) == 0 {
		badRequest(w, "tokens are required")
		return
	}

	current, err := h.storage.LoadTokens()
	if err != nil {
		internalServerError(w, err)
		return
	}

	removed := 0
	for poolName, items := range current {
		filtered := items[:0]
		for _, item := range items {
			normalized := sanitizeTokenText(item["token"])
			if _, ok := removeSet[normalized]; ok {
				removed++
				continue
			}
			filtered = append(filtered, item)
		}
		current[poolName] = append([]storage.TokenData(nil), filtered...)
	}

	if err := h.storage.SaveTokens(current); err != nil {
		internalServerError(w, err)
		return
	}
	if err := token.GetInstance().Reload(); err != nil {
		internalServerError(w, err)
		return
	}
	writeJSON(w, http.StatusOK, map[string]any{"status": "success", "removed": removed})
}

func decodeOptionalJSON(r *http.Request, target any) error {
	if r.Body == nil {
		return nil
	}
	decoder := json.NewDecoder(r.Body)
	decoder.UseNumber()
	if err := decoder.Decode(target); err != nil {
		if errors.Is(err, io.EOF) {
			return nil
		}
		return err
	}
	return nil
}

func ensurePool(result map[string][]*token.TokenInfo, name string) {
	if _, ok := result[name]; !ok {
		result[name] = []*token.TokenInfo{}
	}
}

func ensureStoredPool(result map[string][]storage.TokenData, name string) {
	if _, ok := result[name]; !ok {
		result[name] = []storage.TokenData{}
	}
}

func collectUniqueTokens(primary string, values []string) []string {
	seen := map[string]struct{}{}
	result := make([]string, 0, len(values)+1)
	appendToken := func(value string) {
		normalized := sanitizeTokenText(value)
		if normalized == "" {
			return
		}
		if _, ok := seen[normalized]; ok {
			return
		}
		seen[normalized] = struct{}{}
		result = append(result, normalized)
	}
	appendToken(primary)
	for _, value := range values {
		appendToken(value)
	}
	return result
}

func snapshotTokenStates(values map[string][]*token.TokenInfo) map[string]tokenRefreshState {
	result := make(map[string]tokenRefreshState)
	for _, items := range values {
		for _, item := range items {
			if item == nil {
				continue
			}
			result[item.Token] = tokenRefreshState{
				Status:     item.Status,
				Quota:      item.Quota,
				Consumed:   item.Consumed,
				LastSyncAt: item.LastSyncAt,
				Exists:     true,
			}
		}
	}
	return result
}

func tokenStateChanged(before, after tokenRefreshState) bool {
	if before.Exists != after.Exists {
		return true
	}
	if !before.Exists && !after.Exists {
		return false
	}
	return before.Status != after.Status || before.Quota != after.Quota || before.Consumed != after.Consumed || before.LastSyncAt != after.LastSyncAt
}

func moveStoredToken(current map[string][]storage.TokenData, tokenValue, fromPool, toPool string) (map[string][]storage.TokenData, bool, error) {
	ensureStoredPool(current, toPool)
	if fromPool == toPool && fromPool != "" {
		return current, true, nil
	}

	item, sourcePool, found := extractStoredToken(current, tokenValue, fromPool)
	if !found {
		return current, false, nil
	}
	if sourcePool == toPool {
		return current, true, nil
	}
	if poolContainsToken(current[toPool], tokenValue) {
		return nil, false, errors.New("token already exists in destination pool")
	}
	item["token"] = tokenValue
	current[toPool] = append(current[toPool], item)
	return current, true, nil
}

func extractStoredToken(current map[string][]storage.TokenData, tokenValue, fromPool string) (storage.TokenData, string, bool) {
	pools := []string{}
	if fromPool != "" {
		pools = append(pools, fromPool)
	}
	for poolName := range current {
		if poolName != fromPool {
			pools = append(pools, poolName)
		}
	}
	for _, poolName := range pools {
		items := current[poolName]
		filtered := items[:0]
		for _, item := range items {
			if sanitizeTokenText(item["token"]) == tokenValue {
				current[poolName] = append([]storage.TokenData(nil), filtered...)
				return copyTokenData(item), poolName, true
			}
			filtered = append(filtered, item)
		}
		current[poolName] = append([]storage.TokenData(nil), filtered...)
	}
	return nil, "", false
}

func poolContainsToken(items []storage.TokenData, tokenValue string) bool {
	for _, item := range items {
		if sanitizeTokenText(item["token"]) == tokenValue {
			return true
		}
	}
	return false
}

func existingTokenIndex(data map[string][]storage.TokenData) map[string]map[string]storage.TokenData {
	index := make(map[string]map[string]storage.TokenData, len(data))
	for poolName, items := range data {
		poolIndex := make(map[string]storage.TokenData, len(items))
		for _, item := range items {
			normalized := sanitizeTokenText(item["token"])
			if normalized == "" {
				continue
			}
			poolIndex[normalized] = copyTokenData(item)
		}
		index[poolName] = poolIndex
	}
	return index
}

func findExistingToken(index map[string]map[string]storage.TokenData, tokenValue string) (storage.TokenData, bool) {
	for _, poolIndex := range index {
		if item, ok := poolIndex[tokenValue]; ok {
			return copyTokenData(item), true
		}
	}
	return nil, false
}

func buildTokenData(item any, poolName string, index map[string]map[string]storage.TokenData) (storage.TokenData, error) {
	var tokenData storage.TokenData
	switch typed := item.(type) {
	case string:
		tokenData = storage.TokenData{"token": typed}
	case map[string]any:
		tokenData = copyTokenData(storage.TokenData(typed))
	default:
		return nil, errors.New("unsupported token payload")
	}

	normalized := sanitizeTokenText(tokenData["token"])
	if normalized == "" {
		return nil, errors.New("token cannot be empty")
	}
	base := copyTokenData(index[poolName][normalized])
	if base == nil {
		if existing, ok := findExistingToken(index, normalized); ok {
			base = existing
		}
	}
	merged := copyTokenData(base)
	if merged == nil {
		merged = storage.TokenData{}
	}
	for key, value := range tokenData {
		merged[key] = value
	}
	merged["token"] = normalized
	return normalizeTokenData(merged, poolName)
}

func normalizeTokenData(data storage.TokenData, poolName string) (storage.TokenData, error) {
	payload, err := json.Marshal(data)
	if err != nil {
		return nil, err
	}
	var info token.TokenInfo
	if err := json.Unmarshal(payload, &info); err != nil {
		return nil, err
	}
	info.Token = sanitizeTokenText(info.Token)
	if info.Token == "" {
		return nil, errors.New("token cannot be empty")
	}
	if info.Status == "" {
		info.Status = token.StatusActive
	}
	if info.Quota == 0 {
		if _, ok := data["quota"]; !ok {
			info.Quota = defaultQuotaForPool(poolName)
		}
	}
	if info.CreatedAt == 0 {
		info.CreatedAt = time.Now().UnixMilli()
	}
	if info.Tags == nil {
		info.Tags = []string{}
	}
	return tokenInfoToData(&info)
}

func tokenInfoToData(info *token.TokenInfo) (storage.TokenData, error) {
	payload, err := json.Marshal(info)
	if err != nil {
		return nil, err
	}
	var data storage.TokenData
	if err := json.Unmarshal(payload, &data); err != nil {
		return nil, err
	}
	data["token"] = sanitizeTokenText(info.Token)
	return data, nil
}

func copyTokenData(data storage.TokenData) storage.TokenData {
	if data == nil {
		return nil
	}
	cloned := make(storage.TokenData, len(data))
	for key, value := range data {
		cloned[key] = value
	}
	return cloned
}

func sanitizeTokenText(value any) string {
	text := tokenSanitizer.Replace(toString(value))
	text = whitespaceRE.ReplaceAllString(text, "")
	text = strings.TrimPrefix(text, "sso=")
	return strings.Map(func(r rune) rune {
		if r > 127 {
			return -1
		}
		return r
	}, text)
}

func normalizePoolName(poolName string) string {
	return strings.TrimSpace(poolName)
}

func defaultQuotaForPool(poolName string) int {
	if poolName == token.SuperPoolName {
		return token.SuperDefaultQuota
	}
	return token.BasicDefaultQuota
}

func (h *Handler) NsfwEnable() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		var body struct {
			Token  string   `json:"token"`
			Tokens []string `json:"tokens"`
		}
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			badRequest(w, "invalid request body")
			return
		}

		mgr := token.GetInstance()
		tokens := collectTokens(body.Token, body.Tokens, mgr)
		if len(tokens) == 0 {
			badRequest(w, "no tokens available")
			return
		}

		nsfwSvc := service.NewNSFWService(h.cfg)
		results := nsfwSvc.Batch(r.Context(), tokens, mgr, nil, nil, nil)
		writeJSON(w, http.StatusOK, map[string]any{
			"status":  "success",
			"results": results,
		})
	})
}

func (h *Handler) NsfwEnableAsync() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		var body struct {
			Token  string   `json:"token"`
			Tokens []string `json:"tokens"`
		}
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			badRequest(w, "invalid request body")
			return
		}

		mgr := token.GetInstance()
		tokens := collectTokens(body.Token, body.Tokens, mgr)
		if len(tokens) == 0 {
			badRequest(w, "no tokens available")
			return
		}

		task := newCacheBatchTask(len(tokens))
		go func() {
			nsfwSvc := service.NewNSFWService(h.cfg)
			results := nsfwSvc.Batch(context.Background(), tokens, mgr, nil, nil, func() bool {
				return task.isCancelled()
			})
			task.finish(map[string]any{"status": "success", "results": results})
		}()
		writeJSON(w, http.StatusOK, map[string]any{
			"status":  "success",
			"task_id": task.id,
		})
	})
}

func collectTokens(single string, list []string, mgr *token.TokenManager) []string {
	var tokens []string
	if s := strings.TrimSpace(single); s != "" {
		tokens = append(tokens, s)
	}
	for _, t := range list {
		if s := strings.TrimSpace(t); s != "" {
			tokens = append(tokens, s)
		}
	}
	if len(tokens) == 0 {
		for _, pool := range mgr.ListAll() {
			for _, info := range pool {
				raw := info.Token
				if strings.HasPrefix(raw, "sso=") {
					raw = raw[4:]
				}
				tokens = append(tokens, raw)
			}
		}
	}
	seen := make(map[string]bool, len(tokens))
	result := tokens[:0]
	for _, t := range tokens {
		if !seen[t] {
			seen[t] = true
			result = append(result, t)
		}
	}
	return result
}
