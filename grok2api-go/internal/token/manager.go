package token

import (
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/storage"
)

const (
	DefaultRefreshIntervalHours          = 8
	DefaultSuperRefreshIntervalHours     = 2
	DefaultReloadIntervalSec             = 30
	DefaultSaveDelayMS                   = 500
	DefaultUsageFlushIntervalSec         = 5
	DefaultOnDemandRefreshMinIntervalSec = 300
	DefaultOnDemandRefreshMaxTokens      = 100
	SuperWindowThresholdSeconds          = 14400

	SuperPoolName = "ssoSuper"
	BasicPoolName = "ssoBasic"
)

type TokenRefreshResult struct {
	RemainingQuota    int
	HasRemainingQuota bool
	WindowSizeSeconds int
	HasWindowSize     bool
	Expired           bool
}

type CoolingTokenRefresher func(token string) (TokenRefreshResult, error)

type dirtyTokenChange struct {
	PoolName   string
	ChangeKind string
}

type TokenManager struct {
	pools                  map[string]*TokenPool
	storage                storage.Storage
	initialized            bool
	mu                     sync.RWMutex
	dirty                  bool
	hasStateChanges        bool
	lastUsageFlushAt       time.Time
	lastReloadAt           time.Time
	lastOnDemandRefreshAt  time.Time
	dirtyTokens            map[string]dirtyTokenChange
	dirtyDeletes           map[string]struct{}
	saveTimer              *time.Timer
	onDemandRefreshRunning bool
}

var (
	managerOnce sync.Once
	managerInst *TokenManager

	refreshHookMu sync.RWMutex
	refreshHook   CoolingTokenRefresher
)

func GetInstance() *TokenManager {
	managerOnce.Do(func() {
		managerInst = &TokenManager{
			pools:        defaultPools(),
			storage:      storage.NewLocalStorage(),
			dirtyTokens:  make(map[string]dirtyTokenChange),
			dirtyDeletes: make(map[string]struct{}),
		}
		if err := managerInst.Load(); err != nil {
			slog.Warn("token manager initial load failed", "error", err)
		}
	})
	return managerInst
}

func SetCoolingTokenRefresher(hook CoolingTokenRefresher) {
	refreshHookMu.Lock()
	defer refreshHookMu.Unlock()
	refreshHook = hook
}

func (m *TokenManager) Load() error {
	if m == nil {
		return errors.New("token manager is nil")
	}

	loaded, err := m.storage.LoadTokens()
	if err != nil {
		return err
	}

	pools := defaultPools()
	for poolName, items := range loaded {
		pool := pools[poolName]
		if pool == nil {
			pool = NewTokenPool(poolName)
			pools[poolName] = pool
		}
		for _, item := range items {
			info, err := tokenInfoFromData(item, poolName)
			if err != nil {
				slog.Warn("skip invalid token while loading", "pool", poolName, "error", err)
				continue
			}
			pool.Add(info)
		}
		pool.RebuildIndex()
	}

	m.mu.Lock()
	defer m.mu.Unlock()
	m.stopSaveTimerLocked()
	m.pools = pools
	m.initialized = true
	m.dirty = false
	m.hasStateChanges = false
	m.dirtyTokens = make(map[string]dirtyTokenChange)
	m.dirtyDeletes = make(map[string]struct{})
	m.lastReloadAt = time.Now()
	return nil
}

func (m *TokenManager) Save() error {
	return m.flushDirty(true)
}

func (m *TokenManager) Reload() error {
	return m.Load()
}

func (m *TokenManager) ReloadIfStale() {
	if m == nil {
		return
	}
	interval := config.GetInt("token.reload_interval_sec")
	if interval <= 0 {
		interval = DefaultReloadIntervalSec
	}
	m.mu.RLock()
	stale := m.lastReloadAt.IsZero() || time.Since(m.lastReloadAt) >= time.Duration(interval)*time.Second
	m.mu.RUnlock()
	if !stale {
		return
	}
	if err := m.Reload(); err != nil {
		slog.Warn("reload stale token pools failed", "error", err)
	}
}

func (m *TokenManager) GetToken(poolName string, exclude map[string]bool, preferTags map[string]bool) string {
	m.mu.RLock()
	pool := m.pools[poolName]
	m.mu.RUnlock()
	if pool == nil {
		return ""
	}
	selected := pool.Select(exclude, preferTags)
	if selected == nil {
		return ""
	}
	return NormalizeToken(selected.Token)
}

func (m *TokenManager) GetTokenForVideo(resolution string, videoLength int, poolCandidates []string) *TokenInfo {
	requiresSuper := resolution == "720p" || videoLength > 6
	primary := BasicPoolName
	fallback := SuperPoolName
	if requiresSuper {
		primary = SuperPoolName
		fallback = BasicPoolName
	}

	orderedPools := make([]string, 0, 2)
	if len(poolCandidates) > 0 {
		// Start with pool_candidates order, then insert primary at front if present.
		// This matches Python: pool_candidates take priority, primary only reorders.
		seen := map[string]bool{}
		for _, poolName := range poolCandidates {
			if !seen[poolName] {
				orderedPools = append(orderedPools, poolName)
				seen[poolName] = true
			}
		}
		// If primary is in candidates, move it to front
		if seen[primary] {
			reordered := make([]string, 0, len(orderedPools))
			reordered = append(reordered, primary)
			for _, p := range orderedPools {
				if p != primary {
					reordered = append(reordered, p)
				}
			}
			orderedPools = reordered
		}
	} else {
		orderedPools = []string{primary, fallback}
	}

	for _, poolName := range orderedPools {
		token := m.getTokenInfo(poolName, nil)
		if token != nil {
			return token
		}
	}
	return nil
}

func (m *TokenManager) Consume(token string, effort EffortType) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	poolName, info := m.findTokenLocked(token)
	if info == nil {
		return fmt.Errorf("token not found: %s", NormalizeToken(token))
	}
	oldStatus := info.Status
	if m.isConsumedModeLocked() {
		info.ConsumeWithConsumed(effort)
	} else {
		info.Consume(effort)
	}
	changeKind := "usage"
	if info.Status != oldStatus {
		changeKind = "state"
	}
	m.markTokenChangeLocked(info, poolName, changeKind)
	m.scheduleSaveLocked()
	return nil
}

func (m *TokenManager) MarkRateLimited(token string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	poolName, info := m.findTokenLocked(token)
	if info == nil {
		return fmt.Errorf("token not found: %s", NormalizeToken(token))
	}
	info.Quota = 0
	info.EnterCooling(true)
	m.markTokenChangeLocked(info, poolName, "state")
	m.scheduleSaveLocked()
	return nil
}

func (m *TokenManager) GetPoolNameForToken(token string) string {
	m.mu.RLock()
	defer m.mu.RUnlock()
	normalized := NormalizeToken(token)
	for poolName, pool := range m.pools {
		if pool.Get(normalized) != nil {
			return poolName
		}
	}
	return ""
}

func (m *TokenManager) RecordFail(token string, statusCode int, reason string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	poolName, info := m.findTokenLocked(token)
	if info == nil {
		return fmt.Errorf("token not found: %s", NormalizeToken(token))
	}
	threshold := config.GetInt("token.fail_threshold")
	if threshold <= 0 {
		threshold = FailThreshold
	}
	info.RecordFail(statusCode, reason, &threshold)
	if statusCode == 401 {
		m.markTokenChangeLocked(info, poolName, "state")
		m.scheduleSaveLocked()
	}
	return nil
}

func (m *TokenManager) RecordSuccess(token string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	poolName, info := m.findTokenLocked(token)
	if info == nil {
		return fmt.Errorf("token not found: %s", NormalizeToken(token))
	}
	info.RecordSuccess(false)
	m.markTokenChangeLocked(info, poolName, "state")
	m.scheduleSaveLocked()
	return nil
}

func (m *TokenManager) AddTag(tokenValue, tag string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	tag = strings.TrimSpace(tag)
	if tag == "" {
		return errors.New("tag cannot be empty")
	}
	poolName, info := m.findTokenLocked(tokenValue)
	if info == nil {
		return fmt.Errorf("token not found: %s", NormalizeToken(tokenValue))
	}
	for _, existing := range info.Tags {
		if existing == tag {
			return nil
		}
	}
	info.Tags = append(info.Tags, tag)
	m.markTokenChangeLocked(info, poolName, "state")
	m.scheduleSaveLocked()
	return nil
}

func (m *TokenManager) MarkAssetClear(token string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	poolName, info := m.findTokenLocked(token)
	if info == nil {
		return fmt.Errorf("token not found: %s", NormalizeToken(token))
	}
	info.LastAssetClearAt = time.Now().UnixMilli()
	m.markTokenChangeLocked(info, poolName, "state")
	m.scheduleSaveLocked()
	return nil
}

func (m *TokenManager) RefreshCoolingTokens(trigger string, maxTokens int) map[string]int {
	result := map[string]int{"checked": 0, "refreshed": 0, "recovered": 0, "expired": 0}
	hook := currentRefreshHook()
	if hook == nil {
		return result
	}

	candidates := m.coolingCandidates(maxTokens)
	result["checked"] = len(candidates)
	if len(candidates) == 0 {
		return result
	}

	stateChanged := false
	for _, candidate := range candidates {
		refreshResult, err := hook(candidate.Token)
		if err != nil {
			slog.Warn("refresh cooling token failed", "trigger", trigger, "token", candidate.Token, "error", err)
			continue
		}

		m.mu.Lock()
		poolName, info := m.findTokenLocked(candidate.Token)
		if info == nil {
			m.mu.Unlock()
			continue
		}
		oldStatus := info.Status
		oldQuota := info.Quota

		if refreshResult.Expired {
			info.Status = StatusExpired
			m.markTokenChangeLocked(info, poolName, "state")
			result["expired"]++
			stateChanged = true
			m.mu.Unlock()
			continue
		}
		if !refreshResult.HasRemainingQuota {
			m.mu.Unlock()
			continue
		}

		if m.isConsumedModeLocked() {
			info.UpdateQuotaWithConsumed(refreshResult.RemainingQuota)
		} else {
			info.UpdateQuota(refreshResult.RemainingQuota)
		}
		info.MarkSynced()
		if refreshResult.HasWindowSize {
			poolName = m.adjustPoolForWindowLocked(info, poolName, refreshResult.WindowSizeSeconds)
		}

		m.markTokenChangeLocked(info, poolName, "state")
		stateChanged = true
		result["refreshed"]++
		if oldStatus == StatusCooling && info.Status == StatusActive && oldQuota == 0 && info.Quota > 0 {
			result["recovered"]++
		}
		m.mu.Unlock()
	}

	if stateChanged {
		if err := m.Save(); err != nil {
			slog.Warn("save refreshed cooling tokens failed", "trigger", trigger, "error", err)
		}
	}
	return result
}

func (m *TokenManager) RefreshCoolingTokensOnDemand(trigger string) map[string]int {
	result := map[string]int{"checked": 0, "refreshed": 0, "recovered": 0, "expired": 0}
	if m == nil || !config.GetBool("token.on_demand_refresh_enabled") {
		return result
	}
	minInterval := config.GetInt("token.on_demand_refresh_min_interval_sec")
	if minInterval <= 0 {
		minInterval = DefaultOnDemandRefreshMinIntervalSec
	}
	maxTokens := config.GetInt("token.on_demand_refresh_max_tokens")
	if maxTokens <= 0 {
		maxTokens = DefaultOnDemandRefreshMaxTokens
	}

	m.mu.Lock()
	if m.onDemandRefreshRunning {
		m.mu.Unlock()
		return result
	}
	if !m.lastOnDemandRefreshAt.IsZero() && time.Since(m.lastOnDemandRefreshAt) < time.Duration(minInterval)*time.Second {
		m.mu.Unlock()
		return result
	}
	m.onDemandRefreshRunning = true
	m.lastOnDemandRefreshAt = time.Now()
	m.mu.Unlock()

	defer func() {
		m.mu.Lock()
		m.onDemandRefreshRunning = false
		m.mu.Unlock()
	}()

	return m.RefreshCoolingTokens(trigger, maxTokens)
}

func (m *TokenManager) AddToken(token, poolName string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if poolName == "" {
		poolName = BasicPoolName
	}
	pool := m.pools[poolName]
	if pool == nil {
		pool = NewTokenPool(poolName)
		m.pools[poolName] = pool
	}
	normalized := NormalizeToken(token)
	if normalized == "" {
		return errors.New("token cannot be empty")
	}
	if pool.Get(normalized) != nil {
		return fmt.Errorf("token already exists in pool %s", poolName)
	}
	info := NewTokenInfo(normalized, defaultQuotaForPool(poolName))
	pool.Add(info)
	m.markTokenChangeLocked(info, poolName, "state")
	return m.flushDirtyLocked(true)
}

func (m *TokenManager) RemoveToken(token string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	normalized := NormalizeToken(token)
	for poolName, pool := range m.pools {
		if pool.Remove(normalized) {
			m.markTokenDeleteLocked(normalized)
			if err := m.flushDirtyLocked(true); err != nil {
				return err
			}
			slog.Info("token removed", "pool", poolName, "token", normalized)
			return nil
		}
	}
	return fmt.Errorf("token not found: %s", normalized)
}

func (m *TokenManager) ListAll() map[string][]*TokenInfo {
	m.mu.RLock()
	defer m.mu.RUnlock()
	result := make(map[string][]*TokenInfo, len(m.pools))
	for poolName, pool := range m.pools {
		items := pool.List()
		copied := make([]*TokenInfo, 0, len(items))
		for _, item := range items {
			copied = append(copied, item.Clone())
		}
		result[poolName] = copied
	}
	return result
}

func (m *TokenManager) GetStats() map[string]TokenPoolStats {
	m.mu.RLock()
	defer m.mu.RUnlock()
	stats := make(map[string]TokenPoolStats, len(m.pools))
	for poolName, pool := range m.pools {
		stats[poolName] = pool.GetStats()
	}
	return stats
}

func (m *TokenManager) getTokenInfo(poolName string, preferTags map[string]bool) *TokenInfo {
	m.mu.RLock()
	pool := m.pools[poolName]
	m.mu.RUnlock()
	if pool == nil {
		return nil
	}
	return pool.Select(nil, preferTags)
}

func (m *TokenManager) coolingCandidates(maxTokens int) []*TokenInfo {
	m.mu.RLock()
	defer m.mu.RUnlock()
	candidates := make([]*TokenInfo, 0)
	for poolName, pool := range m.pools {
		interval := DefaultRefreshIntervalHours
		if poolName == SuperPoolName {
			interval = DefaultSuperRefreshIntervalHours
		}
		if configured := refreshIntervalForPool(poolName); configured > 0 {
			interval = configured
		}
		for _, info := range pool.List() {
			if info.NeedRefresh(interval) {
				candidates = append(candidates, info.Clone())
			}
		}
	}
	sort.Slice(candidates, func(i, j int) bool {
		left := candidates[i]
		right := candidates[j]
		if left.LastSyncAt != right.LastSyncAt {
			return left.LastSyncAt < right.LastSyncAt
		}
		if left.LastUsedAt != right.LastUsedAt {
			return left.LastUsedAt < right.LastUsedAt
		}
		return left.CreatedAt < right.CreatedAt
	})
	if maxTokens > 0 && len(candidates) > maxTokens {
		return candidates[:maxTokens]
	}
	return candidates
}

func (m *TokenManager) findTokenLocked(token string) (string, *TokenInfo) {
	normalized := NormalizeToken(token)
	for poolName, pool := range m.pools {
		if info := pool.Get(normalized); info != nil {
			return poolName, info
		}
	}
	return "", nil
}

func (m *TokenManager) isConsumedModeLocked() bool {
	return config.GetBool("token.consumed_mode_enabled")
}

func (m *TokenManager) markTokenChangeLocked(info *TokenInfo, poolName, changeKind string) {
	if info == nil {
		return
	}
	tokenKey := NormalizeToken(info.Token)
	if existing, ok := m.dirtyTokens[tokenKey]; ok && existing.ChangeKind == "state" {
		changeKind = "state"
	}
	delete(m.dirtyDeletes, tokenKey)
	m.dirtyTokens[tokenKey] = dirtyTokenChange{PoolName: poolName, ChangeKind: changeKind}
	m.dirty = true
	if changeKind == "state" {
		m.hasStateChanges = true
	}
}

func (m *TokenManager) markTokenDeleteLocked(token string) {
	normalized := NormalizeToken(token)
	delete(m.dirtyTokens, normalized)
	m.dirtyDeletes[normalized] = struct{}{}
	m.dirty = true
	m.hasStateChanges = true
}

func (m *TokenManager) scheduleSaveLocked() {
	delayMS := config.GetInt("token.save_delay_ms")
	if delayMS < 0 {
		delayMS = 0
	}
	if delayMS == 0 {
		delayMS = DefaultSaveDelayMS
	}

	delay := time.Duration(delayMS) * time.Millisecond
	if !m.hasStateChanges {
		flushIntervalSec := config.GetInt("token.usage_flush_interval_sec")
		if flushIntervalSec <= 0 {
			flushIntervalSec = DefaultUsageFlushIntervalSec
		}
		remaining := time.Duration(flushIntervalSec)*time.Second - time.Since(m.lastUsageFlushAt)
		if !m.lastUsageFlushAt.IsZero() && remaining > delay {
			delay = remaining
		}
	}

	m.stopSaveTimerLocked()
	m.saveTimer = time.AfterFunc(delay, func() {
		if err := m.flushDirty(false); err != nil {
			slog.Warn("flush dirty tokens failed", "error", err)
		}
	})
}

func (m *TokenManager) stopSaveTimerLocked() {
	if m.saveTimer == nil {
		return
	}
	m.saveTimer.Stop()
	m.saveTimer = nil
}

func (m *TokenManager) flushDirty(force bool) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.flushDirtyLocked(force)
}

func (m *TokenManager) flushDirtyLocked(force bool) error {
	if !force && !m.dirty {
		return nil
	}
	if !force && !m.hasStateChanges {
		flushIntervalSec := config.GetInt("token.usage_flush_interval_sec")
		if flushIntervalSec <= 0 {
			flushIntervalSec = DefaultUsageFlushIntervalSec
		}
		if !m.lastUsageFlushAt.IsZero() && time.Since(m.lastUsageFlushAt) < time.Duration(flushIntervalSec)*time.Second {
			m.scheduleSaveLocked()
			return nil
		}
	}

	serialized, err := m.serializeLocked()
	if err != nil {
		return err
	}
	if err := m.storage.SaveTokens(serialized); err != nil {
		m.dirty = true
		return err
	}

	m.dirty = false
	m.hasStateChanges = false
	m.dirtyTokens = make(map[string]dirtyTokenChange)
	m.dirtyDeletes = make(map[string]struct{})
	m.lastUsageFlushAt = time.Now()
	m.stopSaveTimerLocked()
	return nil
}

func (m *TokenManager) serializeLocked() (map[string][]storage.TokenData, error) {
	result := make(map[string][]storage.TokenData, len(m.pools))
	for poolName, pool := range m.pools {
		items := pool.List()
		serialized := make([]storage.TokenData, 0, len(items))
		for _, item := range items {
			payload, err := tokenDataFromInfo(item)
			if err != nil {
				return nil, err
			}
			serialized = append(serialized, payload)
		}
		result[poolName] = serialized
	}
	return result, nil
}

func (m *TokenManager) adjustPoolForWindowLocked(info *TokenInfo, currentPool string, windowSizeSeconds int) string {
	if info == nil {
		return currentPool
	}
	targetPool := currentPool
	if currentPool == SuperPoolName && windowSizeSeconds >= SuperWindowThresholdSeconds {
		targetPool = BasicPoolName
	} else if currentPool == BasicPoolName && windowSizeSeconds < SuperWindowThresholdSeconds {
		targetPool = SuperPoolName
	}
	if targetPool == currentPool {
		return currentPool
	}
	if m.pools[targetPool] == nil {
		m.pools[targetPool] = NewTokenPool(targetPool)
	}
	m.pools[currentPool].Remove(info.Token)
	m.pools[targetPool].Add(info)
	return targetPool
}

func defaultPools() map[string]*TokenPool {
	return map[string]*TokenPool{
		BasicPoolName: NewTokenPool(BasicPoolName),
		SuperPoolName: NewTokenPool(SuperPoolName),
	}
}

func defaultQuotaForPool(poolName string) int {
	if poolName == SuperPoolName {
		return SuperDefaultQuota
	}
	return BasicDefaultQuota
}

func refreshIntervalForPool(poolName string) int {
	if poolName == SuperPoolName {
		interval := config.GetInt("token.super_refresh_interval_hours")
		if interval > 0 {
			return interval
		}
		return DefaultSuperRefreshIntervalHours
	}
	interval := config.GetInt("token.refresh_interval_hours")
	if interval > 0 {
		return interval
	}
	return DefaultRefreshIntervalHours
}

func currentRefreshHook() CoolingTokenRefresher {
	refreshHookMu.RLock()
	defer refreshHookMu.RUnlock()
	return refreshHook
}

func tokenInfoFromData(data storage.TokenData, poolName string) (*TokenInfo, error) {
	payload, err := json.Marshal(data)
	if err != nil {
		return nil, err
	}
	var info TokenInfo
	if err := json.Unmarshal(payload, &info); err != nil {
		return nil, err
	}
	info.Token = NormalizeToken(info.Token)
	if info.Token == "" {
		return nil, errors.New("token cannot be empty")
	}
	if info.Status == "" {
		info.Status = StatusActive
	}
	if info.Quota == 0 && data["quota"] == nil {
		info.Quota = defaultQuotaForPool(poolName)
	}
	if info.CreatedAt == 0 {
		info.CreatedAt = nowMillis()
	}
	if info.Tags == nil {
		info.Tags = []string{}
	}
	return &info, nil
}

func tokenDataFromInfo(info *TokenInfo) (storage.TokenData, error) {
	payload, err := json.Marshal(info)
	if err != nil {
		return nil, err
	}
	var data storage.TokenData
	if err := json.Unmarshal(payload, &data); err != nil {
		return nil, err
	}
	data["token"] = NormalizeToken(info.Token)
	return data, nil
}
