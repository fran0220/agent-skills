package admin

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/fran0220/grok2api-go/internal/config"
	proxycfg "github.com/fran0220/grok2api-go/internal/proxy"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/storage"
	"github.com/fran0220/grok2api-go/internal/token"
)

var (
	cacheImageExts = map[string]bool{".jpg": true, ".jpeg": true, ".png": true, ".gif": true, ".webp": true, ".bmp": true}
	cacheVideoExts = map[string]bool{".mp4": true, ".mov": true, ".m4v": true, ".webm": true, ".avi": true, ".mkv": true}
)

type localCacheService struct {
	imageDir string
	videoDir string
}

type cacheListItem struct {
	Name      string `json:"name"`
	SizeBytes int64  `json:"size_bytes"`
	MTimeMS   int64  `json:"mtime_ms"`
	ViewURL   string `json:"view_url"`
}

type assetAdminService struct {
	cfg     *config.Config
	session *reverse.ResettableSession
}

type batchTask struct {
	id        string
	total     int
	processed int
	result    any
	cancelled bool
	err       string
	done      bool
	mu        sync.RWMutex
}

var cacheBatchTasks sync.Map

func (h *Handler) Cache() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w, http.MethodGet)
			return
		}
		cacheService := newLocalCacheService()
		accounts := h.listAccounts()
		accountLookup := accountsByToken(accounts)
		scope := strings.TrimSpace(r.URL.Query().Get("scope"))
		selectedToken := strings.TrimSpace(r.URL.Query().Get("token"))
		selectedTokens := parseCSV(r.URL.Query().Get("tokens"))
		onlineStats := map[string]any{"count": 0, "status": "unknown", "token": nil, "last_asset_clear_at": nil}
		onlineDetails := make([]map[string]any, 0)
		switch {
		case len(selectedTokens) > 0:
			details, total := h.fetchAssetsDetailsBatch(r.Context(), selectedTokens, accountLookup, nil)
			onlineDetails = details
			onlineStats = map[string]any{"count": total, "status": boolString(len(selectedTokens) > 0, "ok", "no_token"), "token": nil, "last_asset_clear_at": nil}
			scope = "selected"
		case scope == "all":
			tokens := make([]string, 0, len(accounts))
			for _, account := range accounts {
				tokens = append(tokens, stringValue(account["token"]))
			}
			details, total := h.fetchAssetsDetailsBatch(r.Context(), tokens, accountLookup, nil)
			onlineDetails = details
			onlineStats = map[string]any{"count": total, "status": boolString(len(tokens) > 0, "ok", "no_token"), "token": nil, "last_asset_clear_at": nil}
		default:
			if selectedToken == "" {
				onlineStats = map[string]any{"count": 0, "status": "not_loaded", "token": nil, "last_asset_clear_at": nil}
				break
			}
			detail, count, err := h.fetchAssetDetail(r.Context(), selectedToken, accountLookup[selectedToken])
			if err != nil {
				onlineStats = map[string]any{"count": 0, "status": fmt.Sprintf("error: %v", err), "token": selectedToken, "token_masked": maskedToken(selectedToken), "last_asset_clear_at": lastClear(accountLookup[selectedToken])}
				break
			}
			onlineStats = map[string]any{"count": count, "status": detail["status"], "token": detail["token"], "token_masked": detail["token_masked"], "last_asset_clear_at": detail["last_asset_clear_at"]}
		}
		writeJSON(w, http.StatusOK, map[string]any{
			"local_image":     cacheService.getStats("image"),
			"local_video":     cacheService.getStats("video"),
			"online":          onlineStats,
			"online_accounts": accounts,
			"online_scope":    emptyScope(scope),
			"online_details":  onlineDetails,
		})
	})
}

func (h *Handler) CacheList() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w, http.MethodGet)
			return
		}
		cacheType := strings.TrimSpace(r.URL.Query().Get("type"))
		if cacheType == "" {
			cacheType = "image"
		}
		page := parsePositiveInt(r.URL.Query().Get("page"), 1)
		pageSize := parsePositiveInt(r.URL.Query().Get("page_size"), 1000)
		result := newLocalCacheService().listFiles(cacheType, page, pageSize)
		payload := map[string]any{"status": "success"}
		for key, value := range result {
			payload[key] = value
		}
		writeJSON(w, http.StatusOK, payload)
	})
}

func (h *Handler) CacheClear() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		var payload map[string]any
		if err := decodeOptionalJSON(r, &payload); err != nil {
			badRequest(w, "invalid JSON body")
			return
		}
		cacheType := strings.TrimSpace(stringValue(payload["type"]))
		if cacheType == "" {
			cacheType = "image"
		}
		writeJSON(w, http.StatusOK, map[string]any{"status": "success", "result": newLocalCacheService().clear(cacheType)})
	})
}

func (h *Handler) CacheDeleteItem() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		var payload map[string]any
		if err := decodeOptionalJSON(r, &payload); err != nil {
			badRequest(w, "invalid JSON body")
			return
		}
		cacheType := strings.TrimSpace(stringValue(payload["type"]))
		if cacheType == "" {
			cacheType = "image"
		}
		name := strings.TrimSpace(stringValue(payload["name"]))
		if name == "" {
			badRequest(w, "Missing file name")
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{"status": "success", "result": newLocalCacheService().deleteFile(cacheType, name)})
	})
}

func (h *Handler) CacheOnlineClear() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		var payload map[string]any
		if err := decodeOptionalJSON(r, &payload); err != nil {
			badRequest(w, "invalid JSON body")
			return
		}
		mgr := token.GetInstance()
		tokens := dedupeStrings(anyToStringSlice(payload["tokens"]))
		if len(tokens) > 0 {
			results := make(map[string]any, len(tokens))
			for _, tokenValue := range tokens {
				result, err := h.clearAssetsForToken(r.Context(), mgr, tokenValue)
				if err != nil {
					results[tokenValue] = map[string]any{"status": "error", "error": err.Error()}
					continue
				}
				results[tokenValue] = result
			}
			writeJSON(w, http.StatusOK, map[string]any{"status": "success", "results": results})
			return
		}
		tokenValue := strings.TrimSpace(stringValue(payload["token"]))
		if tokenValue == "" {
			accounts := h.listAccounts()
			if len(accounts) == 0 {
				badRequest(w, "No available token to perform cleanup")
				return
			}
			tokenValue = stringValue(accounts[0]["token"])
		}
		result, err := h.clearAssetsForToken(r.Context(), mgr, tokenValue)
		if err != nil {
			writeJSON(w, http.StatusOK, map[string]any{"status": "error", "error": err.Error()})
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{"status": "success", "result": result["result"]})
	})
}

func (h *Handler) CacheOnlineLoadAsync() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		var payload map[string]any
		if err := decodeOptionalJSON(r, &payload); err != nil {
			badRequest(w, "invalid JSON body")
			return
		}
		accounts := h.listAccounts()
		accountLookup := accountsByToken(accounts)
		tokens := dedupeStrings(anyToStringSlice(payload["tokens"]))
		scope := strings.TrimSpace(stringValue(payload["scope"]))
		if len(tokens) == 0 && scope == "all" {
			for _, account := range accounts {
				tokens = append(tokens, stringValue(account["token"]))
			}
			scope = "all"
		}
		if len(tokens) == 0 {
			badRequest(w, "No tokens provided")
			return
		}
		if scope == "" {
			scope = "selected"
		}
		task := newCacheBatchTask(len(tokens))
		go func() {
			cacheService := newLocalCacheService()
			details, total := h.fetchAssetsDetailsBatch(context.Background(), tokens, accountLookup, task)
			if task.isCancelled() {
				task.finishCancelled()
				return
			}
			task.finish(map[string]any{
				"local_image":     cacheService.getStats("image"),
				"local_video":     cacheService.getStats("video"),
				"online":          map[string]any{"count": total, "status": boolString(len(tokens) > 0, "ok", "no_token"), "token": nil, "last_asset_clear_at": nil},
				"online_accounts": accounts,
				"online_scope":    scope,
				"online_details":  details,
			})
		}()
		writeJSON(w, http.StatusOK, map[string]any{"status": "success", "task_id": task.id, "total": len(tokens)})
	})
}

func (h *Handler) CacheOnlineClearAsync() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		var payload map[string]any
		if err := decodeOptionalJSON(r, &payload); err != nil {
			badRequest(w, "invalid JSON body")
			return
		}
		tokens := dedupeStrings(anyToStringSlice(payload["tokens"]))
		if len(tokens) == 0 {
			badRequest(w, "No tokens provided")
			return
		}
		task := newCacheBatchTask(len(tokens))
		go func() {
			mgr := token.GetInstance()
			results := make(map[string]any, len(tokens))
			okCount := 0
			failCount := 0
			for _, tokenValue := range tokens {
				if task.isCancelled() {
					task.finishCancelled()
					return
				}
				result, err := h.clearAssetsForToken(context.Background(), mgr, tokenValue)
				if err != nil {
					results[tokenValue] = map[string]any{"status": "error", "error": err.Error()}
					failCount++
				} else {
					results[tokenValue] = result
					okCount++
				}
				task.record()
			}
			task.finish(map[string]any{"status": "success", "summary": map[string]any{"total": len(tokens), "ok": okCount, "fail": failCount}, "results": results})
		}()
		writeJSON(w, http.StatusOK, map[string]any{"status": "success", "task_id": task.id, "total": len(tokens)})
	})
}

func (h *Handler) Batch() http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !validateCacheAppKey(h.cfg.GetString("app.app_key", "grok2api"), r) {
			writeJSON(w, http.StatusUnauthorized, map[string]string{"error": "Invalid authentication token"})
			return
		}
		parts := strings.Split(strings.Trim(strings.TrimPrefix(r.URL.Path, "/v1/admin/batch/"), "/"), "/")
		if len(parts) != 2 {
			writeJSON(w, http.StatusNotFound, map[string]string{"error": "not found"})
			return
		}
		task := loadCacheBatchTask(parts[0])
		if task == nil {
			writeJSON(w, http.StatusNotFound, map[string]string{"error": "task not found"})
			return
		}
		switch parts[1] {
		case "stream":
			if r.Method != http.MethodGet {
				methodNotAllowed(w, http.MethodGet)
				return
			}
			streamCacheBatchTask(w, r, task)
		case "cancel":
			if r.Method != http.MethodPost {
				methodNotAllowed(w, http.MethodPost)
				return
			}
			task.cancel()
			writeJSON(w, http.StatusOK, map[string]string{"status": "success"})
		default:
			writeJSON(w, http.StatusNotFound, map[string]string{"error": "not found"})
		}
	})
}

func (h *Handler) listAccounts() []map[string]any {
	all := token.GetInstance().ListAll()
	poolNames := make([]string, 0, len(all))
	for poolName := range all {
		poolNames = append(poolNames, poolName)
	}
	sort.Strings(poolNames)
	accounts := make([]map[string]any, 0)
	for _, poolName := range poolNames {
		for _, info := range all[poolName] {
			accounts = append(accounts, map[string]any{
				"token":               info.Token,
				"token_masked":        maskedToken(info.Token),
				"pool":                poolName,
				"status":              info.Status,
				"last_asset_clear_at": info.LastAssetClearAt,
			})
		}
	}
	return accounts
}

func (h *Handler) fetchAssetDetail(ctx context.Context, tokenValue string, account map[string]any) (map[string]any, int, error) {
	service := newAssetAdminService(h.cfg)
	defer service.close()
	assetIDs, err := service.listAssets(ctx, tokenValue)
	if err != nil {
		return map[string]any{"token": tokenValue, "token_masked": maskedToken(tokenValue), "count": 0, "status": fmt.Sprintf("error: %v", err), "last_asset_clear_at": lastClear(account)}, 0, err
	}
	detail := map[string]any{"token": tokenValue, "token_masked": maskedToken(tokenValue), "count": len(assetIDs), "status": "ok", "last_asset_clear_at": lastClear(account)}
	return detail, len(assetIDs), nil
}

func (h *Handler) fetchAssetsDetailsBatch(ctx context.Context, tokens []string, accountLookup map[string]map[string]any, task *batchTask) ([]map[string]any, int) {
	tokens = dedupeStrings(tokens)
	limit := h.cfg.GetInt("asset.list_batch_size", 50)
	if limit <= 0 {
		limit = 50
	}
	sem := make(chan struct{}, limit)
	var mu sync.Mutex
	var wg sync.WaitGroup
	details := make([]map[string]any, 0, len(tokens))
	total := 0
	for _, tokenValue := range tokens {
		if task != nil && task.isCancelled() {
			break
		}
		wg.Add(1)
		sem <- struct{}{}
		go func(tokenValue string) {
			defer wg.Done()
			defer func() { <-sem }()
			detail, count, _ := h.fetchAssetDetail(ctx, tokenValue, accountLookup[tokenValue])
			mu.Lock()
			details = append(details, detail)
			total += count
			mu.Unlock()
			if task != nil {
				task.record()
			}
		}(tokenValue)
	}
	wg.Wait()
	sort.Slice(details, func(i, j int) bool {
		return stringValue(details[i]["token_masked"]) < stringValue(details[j]["token_masked"])
	})
	return details, total
}

func (h *Handler) clearAssetsForToken(ctx context.Context, mgr *token.TokenManager, tokenValue string) (map[string]any, error) {
	service := newAssetAdminService(h.cfg)
	defer service.close()
	assetIDs, err := service.listAssets(ctx, tokenValue)
	if err != nil {
		return nil, err
	}
	result, err := service.deleteAssets(ctx, tokenValue, assetIDs)
	if err != nil {
		return nil, err
	}
	if err := mgr.MarkAssetClear(tokenValue); err != nil {
		return nil, err
	}
	return map[string]any{"status": "success", "result": result}, nil
}

func newAssetAdminService(cfg *config.Config) *assetAdminService {
	return &assetAdminService{cfg: cfg, session: reverse.NewResettableSession(reverse.SessionOptions{Browser: cfg.GetString("proxy.browser", "")})}
}

func (s *assetAdminService) close() {
	if s != nil && s.session != nil {
		s.session.Close()
	}
}

func (s *assetAdminService) listAssets(ctx context.Context, tokenValue string) ([]string, error) {
	params := map[string]string{"pageSize": "50", "orderBy": "ORDER_BY_LAST_USE_TIME", "source": "SOURCE_ANY", "isLatest": "true"}
	pageToken := ""
	seen := make(map[string]bool)
	assetIDs := make([]string, 0)
	for {
		if pageToken != "" {
			if seen[pageToken] {
				break
			}
			seen[pageToken] = true
			params["pageToken"] = pageToken
		} else {
			delete(params, "pageToken")
		}
		payload, err := s.assetsRequest(ctx, http.MethodGet, "https://grok.com/rest/assets", tokenValue, params)
		if err != nil {
			return nil, err
		}
		for _, asset := range anyMapSlice(payload["assets"]) {
			if assetID := strings.TrimSpace(stringValue(asset["assetId"])); assetID != "" {
				assetIDs = append(assetIDs, assetID)
			}
		}
		pageToken = strings.TrimSpace(stringValue(payload["nextPageToken"]))
		if pageToken == "" {
			break
		}
	}
	return assetIDs, nil
}

func (s *assetAdminService) deleteAssets(ctx context.Context, tokenValue string, assetIDs []string) (map[string]any, error) {
	if len(assetIDs) == 0 {
		return map[string]any{"total": 0, "success": 0, "failed": 0, "skipped": true}, nil
	}
	concurrency := s.cfg.GetInt("asset.delete_concurrent", 100)
	if concurrency <= 0 {
		concurrency = 100
	}
	sem := make(chan struct{}, concurrency)
	var mu sync.Mutex
	var wg sync.WaitGroup
	success := 0
	failed := 0
	for _, assetID := range assetIDs {
		if assetID == "" {
			continue
		}
		wg.Add(1)
		sem <- struct{}{}
		go func(assetID string) {
			defer wg.Done()
			defer func() { <-sem }()
			_, err := s.assetsRequest(ctx, http.MethodDelete, "https://grok.com/rest/assets-metadata/"+assetID, tokenValue, nil)
			mu.Lock()
			defer mu.Unlock()
			if err != nil {
				failed++
				return
			}
			success++
		}(assetID)
	}
	wg.Wait()
	return map[string]any{"total": len(assetIDs), "success": success, "failed": failed}, nil
}

func (s *assetAdminService) assetsRequest(ctx context.Context, method, url, tokenValue string, query map[string]string) (map[string]any, error) {
	headers := reverse.BuildHeaders(tokenValue, "application/json", "https://grok.com", "https://grok.com/files")
	timeoutSeconds := s.cfg.GetInt("asset.list_timeout", 60)
	if method == http.MethodDelete {
		timeoutSeconds = s.cfg.GetInt("asset.delete_timeout", 60)
	}
	requestTimeout := time.Duration(timeoutSeconds) * time.Second
	var activeProxyKey string
	resp, err := reverse.RetryOnStatus(ctx, func(callCtx context.Context) (*http.Response, error) {
		requestCtx := callCtx
		var cancel context.CancelFunc
		if requestTimeout > 0 {
			requestCtx, cancel = context.WithTimeout(callCtx, requestTimeout)
			defer cancel()
		}
		activeProxyKey, _ = proxycfg.GetCurrentProxyFrom("proxy.asset_proxy_url", "proxy.base_proxy_url")
		activeProxyURL := ""
		if activeProxyKey != "" {
			activeProxyURL = proxycfg.GetCurrentProxy(activeProxyKey)
		}
		req, err := http.NewRequestWithContext(requestCtx, method, url, nil)
		if err != nil {
			return nil, err
		}
		for key, value := range headers {
			req.Header.Set(key, value)
		}
		if len(query) > 0 {
			values := req.URL.Query()
			for key, value := range query {
				values.Set(key, value)
			}
			req.URL.RawQuery = values.Encode()
		}
		response, err := s.session.DoWithProxy(req, activeProxyURL)
		if err != nil {
			return nil, err
		}
		if response.StatusCode != http.StatusOK {
			body := readResponseBody(response.Body)
			return nil, &reverse.UpstreamError{Message: fmt.Sprintf("assets request failed: %d", response.StatusCode), StatusCode: response.StatusCode, Body: body, Headers: response.Header.Clone()}
		}
		return response, nil
	}, reverse.RetryOptions{OnRetry: func(ctx context.Context, attempt, status int, err error, delay time.Duration) {
		if activeProxyKey != "" && proxycfg.ShouldRotateProxy(status) {
			proxycfg.RotateProxy(activeProxyKey)
		}
	}})
	if err != nil {
		var upstreamErr *reverse.UpstreamError
		if errors.As(err, &upstreamErr) && upstreamErr.StatusCode == http.StatusUnauthorized {
			_ = token.GetInstance().RecordFail(tokenValue, upstreamErr.StatusCode, "assets_auth_failed")
		}
		return nil, err
	}
	defer resp.Body.Close()
	var payload map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&payload); err != nil {
		return nil, err
	}
	return payload, nil
}

func newLocalCacheService() *localCacheService {
	baseDir := filepath.Join(storage.DataDir(), "tmp")
	imageDir := filepath.Join(baseDir, "image")
	videoDir := filepath.Join(baseDir, "video")
	_ = os.MkdirAll(imageDir, 0o755)
	_ = os.MkdirAll(videoDir, 0o755)
	return &localCacheService{imageDir: imageDir, videoDir: videoDir}
}

func (s *localCacheService) getStats(mediaType string) map[string]any {
	files := s.files(mediaType)
	var total int64
	for _, file := range files {
		total += file.Size()
	}
	return map[string]any{"count": len(files), "size_mb": roundMB(total)}
}

func (s *localCacheService) listFiles(mediaType string, page, pageSize int) map[string]any {
	files := s.files(mediaType)
	items := make([]cacheListItem, 0, len(files))
	for _, file := range files {
		items = append(items, cacheListItem{Name: file.Name(), SizeBytes: file.Size(), MTimeMS: file.ModTime().UnixMilli(), ViewURL: "/v1/files/" + mediaType + "/" + file.Name()})
	}
	sort.Slice(items, func(i, j int) bool { return items[i].MTimeMS > items[j].MTimeMS })
	if page <= 0 {
		page = 1
	}
	if pageSize <= 0 {
		pageSize = 1000
	}
	start := (page - 1) * pageSize
	if start > len(items) {
		start = len(items)
	}
	end := start + pageSize
	if end > len(items) {
		end = len(items)
	}
	return map[string]any{"total": len(items), "page": page, "page_size": pageSize, "items": items[start:end]}
}

func (s *localCacheService) deleteFile(mediaType, name string) map[string]any {
	path := filepath.Join(s.dir(mediaType), strings.ReplaceAll(name, "/", "-"))
	if err := os.Remove(path); err != nil {
		if os.IsNotExist(err) {
			return map[string]any{"deleted": false}
		}
		return map[string]any{"deleted": false}
	}
	return map[string]any{"deleted": true}
}

func (s *localCacheService) clear(mediaType string) map[string]any {
	files := s.files(mediaType)
	var total int64
	count := 0
	for _, file := range files {
		total += file.Size()
		if err := os.Remove(filepath.Join(s.dir(mediaType), file.Name())); err == nil {
			count++
		}
	}
	return map[string]any{"count": count, "size_mb": roundMB(total)}
}

func (s *localCacheService) files(mediaType string) []os.FileInfo {
	entries, err := os.ReadDir(s.dir(mediaType))
	if err != nil {
		return nil
	}
	allowed := cacheImageExts
	if mediaType == "video" {
		allowed = cacheVideoExts
	}
	files := make([]os.FileInfo, 0, len(entries))
	for _, entry := range entries {
		if entry.IsDir() || !allowed[strings.ToLower(filepath.Ext(entry.Name()))] {
			continue
		}
		info, err := entry.Info()
		if err != nil {
			continue
		}
		files = append(files, info)
	}
	return files
}

func (s *localCacheService) dir(mediaType string) string {
	if mediaType == "video" {
		return s.videoDir
	}
	return s.imageDir
}

func newCacheBatchTask(total int) *batchTask {
	task := &batchTask{id: fmt.Sprintf("task_%d", time.Now().UnixNano()), total: total}
	cacheBatchTasks.Store(task.id, task)
	go func() {
		time.Sleep(5 * time.Minute)
		cacheBatchTasks.Delete(task.id)
	}()
	return task
}

func loadCacheBatchTask(id string) *batchTask {
	if value, ok := cacheBatchTasks.Load(id); ok {
		if task, ok := value.(*batchTask); ok {
			return task
		}
	}
	return nil
}

func (t *batchTask) record() {
	t.mu.Lock()
	defer t.mu.Unlock()
	if !t.done && !t.cancelled {
		t.processed++
	}
}

func (t *batchTask) finish(result any) {
	t.mu.Lock()
	defer t.mu.Unlock()
	if t.cancelled {
		return
	}
	t.done = true
	t.result = result
	t.processed = t.total
}

func (t *batchTask) finishCancelled() {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.cancelled = true
	t.done = true
}

func (t *batchTask) cancel() {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.cancelled = true
}

func (t *batchTask) isCancelled() bool {
	t.mu.RLock()
	defer t.mu.RUnlock()
	return t.cancelled
}

func (t *batchTask) snapshot() map[string]any {
	t.mu.RLock()
	defer t.mu.RUnlock()
	payload := map[string]any{"total": t.total, "processed": t.processed}
	switch {
	case t.cancelled:
		payload["type"] = "cancelled"
	case t.err != "":
		payload["type"] = "error"
		payload["error"] = t.err
	case t.done:
		payload["type"] = "done"
		payload["result"] = t.result
	default:
		payload["type"] = "progress"
	}
	return payload
}

func streamCacheBatchTask(w http.ResponseWriter, r *http.Request, task *batchTask) {
	flusher, ok := w.(http.Flusher)
	if !ok {
		writeJSON(w, http.StatusInternalServerError, map[string]string{"error": "stream not supported"})
		return
	}
	w.Header().Set("Content-Type", "text/event-stream")
	w.Header().Set("Cache-Control", "no-cache")
	w.Header().Set("Connection", "keep-alive")
	w.WriteHeader(http.StatusOK)
	writeCacheBatchMessage(w, flusher, map[string]any{"type": "snapshot", "total": task.total, "processed": task.processed})
	ticker := time.NewTicker(300 * time.Millisecond)
	defer ticker.Stop()
	for {
		select {
		case <-r.Context().Done():
			return
		case <-ticker.C:
			payload := task.snapshot()
			writeCacheBatchMessage(w, flusher, payload)
			typeValue := stringValue(payload["type"])
			if typeValue == "done" || typeValue == "cancelled" || typeValue == "error" {
				return
			}
		}
	}
}

func writeCacheBatchMessage(w http.ResponseWriter, flusher http.Flusher, payload map[string]any) {
	data, _ := json.Marshal(payload)
	_, _ = w.Write([]byte("data: "))
	_, _ = w.Write(data)
	_, _ = w.Write([]byte("\n\n"))
	flusher.Flush()
}

func validateCacheAppKey(appKey string, r *http.Request) bool {
	authorization := strings.TrimSpace(r.Header.Get("Authorization"))
	if strings.HasPrefix(strings.ToLower(authorization), "bearer ") {
		return strings.TrimSpace(authorization[7:]) == appKey
	}
	return strings.TrimSpace(r.URL.Query().Get("app_key")) == appKey
}

func accountsByToken(accounts []map[string]any) map[string]map[string]any {
	lookup := make(map[string]map[string]any, len(accounts))
	for _, account := range accounts {
		lookup[stringValue(account["token"])] = account
	}
	return lookup
}

func lastClear(account map[string]any) any {
	if account == nil {
		return nil
	}
	if value, ok := account["last_asset_clear_at"]; ok {
		return value
	}
	return nil
}

func maskedToken(tokenValue string) string {
	if len(tokenValue) <= 24 {
		return tokenValue
	}
	return tokenValue[:8] + "..." + tokenValue[len(tokenValue)-16:]
}

func parseCSV(value string) []string {
	parts := strings.Split(value, ",")
	result := make([]string, 0, len(parts))
	for _, part := range parts {
		if trimmed := strings.TrimSpace(part); trimmed != "" {
			result = append(result, trimmed)
		}
	}
	return result
}

func anyToStringSlice(value any) []string {
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

func dedupeStrings(values []string) []string {
	seen := make(map[string]bool)
	result := make([]string, 0, len(values))
	for _, value := range values {
		trimmed := strings.TrimSpace(value)
		if trimmed == "" || seen[trimmed] {
			continue
		}
		seen[trimmed] = true
		result = append(result, trimmed)
	}
	return result
}

func anyMapSlice(value any) []map[string]any {
	switch typed := value.(type) {
	case []map[string]any:
		return typed
	case []any:
		result := make([]map[string]any, 0, len(typed))
		for _, item := range typed {
			if mapped := mapValue(item); mapped != nil {
				result = append(result, mapped)
			}
		}
		return result
	default:
		return nil
	}
}

func parsePositiveInt(value string, fallback int) int {
	parsed, err := strconv.Atoi(strings.TrimSpace(value))
	if err != nil || parsed <= 0 {
		return fallback
	}
	return parsed
}

func roundMB(size int64) float64 {
	return float64(size) / 1024.0 / 1024.0
}

func emptyScope(scope string) string {
	if strings.TrimSpace(scope) == "" {
		return "none"
	}
	return scope
}

func boolString(ok bool, truthy, falsy string) string {
	if ok {
		return truthy
	}
	return falsy
}

func readResponseBody(body io.ReadCloser) string {
	if body == nil {
		return ""
	}
	defer body.Close()
	data, _ := io.ReadAll(io.LimitReader(body, 4096))
	return string(data)
}
