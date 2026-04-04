package token

import (
	"strings"
	"time"
)

const (
	BasicDefaultQuota = 80
	SuperDefaultQuota = 140
	FailThreshold     = 5
)

type TokenStatus string

const (
	StatusActive   TokenStatus = "active"
	StatusDisabled TokenStatus = "disabled"
	StatusExpired  TokenStatus = "expired"
	StatusCooling  TokenStatus = "cooling"
)

type EffortType string

const (
	EffortLow  EffortType = "low"
	EffortHigh EffortType = "high"
)

var EffortCost = map[EffortType]int{
	EffortLow:  1,
	EffortHigh: 4,
}

type TokenInfo struct {
	Token            string      `json:"token"`
	Status           TokenStatus `json:"status"`
	Quota            int         `json:"quota"`
	Consumed         int         `json:"consumed"`
	CreatedAt        int64       `json:"created_at"`
	LastUsedAt       int64       `json:"last_used_at,omitempty"`
	UseCount         int64       `json:"use_count"`
	FailCount        int         `json:"fail_count"`
	LastFailAt       int64       `json:"last_fail_at,omitempty"`
	LastFailReason   string      `json:"last_fail_reason,omitempty"`
	LastSyncAt       int64       `json:"last_sync_at,omitempty"`
	Tags             []string    `json:"tags,omitempty"`
	Note             string      `json:"note,omitempty"`
	LastAssetClearAt int64       `json:"last_asset_clear_at,omitempty"`
}

type TokenPoolStats struct {
	Total         int     `json:"total"`
	Active        int     `json:"active"`
	Disabled      int     `json:"disabled"`
	Expired       int     `json:"expired"`
	Cooling       int     `json:"cooling"`
	TotalQuota    int     `json:"total_quota"`
	AvgQuota      float64 `json:"avg_quota"`
	TotalConsumed int     `json:"total_consumed"`
	AvgConsumed   float64 `json:"avg_consumed"`
}

func NewTokenInfo(token string, quota int) *TokenInfo {
	if quota <= 0 {
		quota = BasicDefaultQuota
	}
	return &TokenInfo{
		Token:     NormalizeToken(token),
		Status:    StatusActive,
		Quota:     quota,
		CreatedAt: nowMillis(),
		Tags:      []string{},
	}
}

func NormalizeToken(value string) string {
	token := strings.TrimSpace(value)
	token = strings.TrimPrefix(token, "sso=")
	token = strings.ReplaceAll(token, " ", "")
	return token
}

func (t *TokenInfo) IsAvailable(consumedMode bool) bool {
	if t == nil || t.Status != StatusActive {
		return false
	}
	if consumedMode {
		return true
	}
	return t.Quota > 0
}

func (t *TokenInfo) EnterCooling(resetConsumed bool) {
	if t == nil {
		return
	}
	t.Status = StatusCooling
	if resetConsumed {
		t.Consumed = 0
	}
}

func (t *TokenInfo) RecoverActive(allowFromExpired bool) {
	if t == nil {
		return
	}
	if t.Status == StatusCooling || (allowFromExpired && t.Status == StatusExpired) {
		t.Status = StatusActive
		t.FailCount = 0
		t.LastFailAt = 0
		t.LastFailReason = ""
	}
}

func (t *TokenInfo) Consume(effort EffortType) int {
	if t == nil {
		return 0
	}
	cost := effortCost(effort)
	actualCost := min(cost, max(t.Quota, 0))
	t.LastUsedAt = nowMillis()
	t.Consumed += cost
	t.UseCount += int64(actualCost)
	t.Quota = max(t.Quota-actualCost, 0)
	if t.Quota <= 0 {
		t.EnterCooling(true)
	} else {
		t.RecoverActive(false)
	}
	return t.Quota
}

func (t *TokenInfo) ConsumeWithConsumed(effort EffortType) int {
	if t == nil {
		return 0
	}
	cost := effortCost(effort)
	t.Consumed += cost
	t.LastUsedAt = nowMillis()
	t.UseCount++
	t.RecoverActive(false)
	return cost
}

func (t *TokenInfo) UpdateQuota(newQuota int) {
	if t == nil {
		return
	}
	t.Quota = max(newQuota, 0)
	if t.Quota == 0 {
		t.EnterCooling(true)
		return
	}
	t.RecoverActive(true)
}

func (t *TokenInfo) UpdateQuotaWithConsumed(newQuota int) {
	if t == nil {
		return
	}
	t.Quota = max(newQuota, 0)
	if t.Quota == 0 {
		t.EnterCooling(true)
		return
	}
	t.Consumed = 0
	t.RecoverActive(true)
}

func (t *TokenInfo) Reset(defaultQuota *int) {
	if t == nil {
		return
	}
	quota := BasicDefaultQuota
	if defaultQuota != nil {
		quota = *defaultQuota
	}
	t.Quota = max(quota, 0)
	t.Status = StatusActive
	t.FailCount = 0
	t.LastFailAt = 0
	t.LastFailReason = ""
	t.Consumed = 0
}

func (t *TokenInfo) RecordFail(statusCode int, reason string, threshold *int) {
	if t == nil || statusCode != 401 {
		return
	}
	limit := FailThreshold
	if threshold != nil && *threshold > 0 {
		limit = *threshold
	}
	t.FailCount++
	t.LastFailAt = nowMillis()
	t.LastFailReason = reason
	if t.FailCount >= limit {
		t.EnterCooling(false)
	}
}

func (t *TokenInfo) RecordSuccess(isUsage bool) {
	if t == nil {
		return
	}
	t.FailCount = 0
	t.LastFailAt = 0
	t.LastFailReason = ""
	if isUsage {
		t.UseCount++
		t.LastUsedAt = nowMillis()
	}
}

func (t *TokenInfo) NeedRefresh(intervalHours int) bool {
	if t == nil || t.Status != StatusCooling {
		return false
	}
	if t.LastSyncAt == 0 {
		return true
	}
	if intervalHours <= 0 {
		intervalHours = 8
	}
	interval := int64(intervalHours) * int64(time.Hour/time.Millisecond)
	return nowMillis()-t.LastSyncAt >= interval
}

func (t *TokenInfo) MarkSynced() {
	if t == nil {
		return
	}
	t.LastSyncAt = nowMillis()
}

func (t *TokenInfo) ShouldCoolDown(remainingTokens, threshold int) bool {
	if t == nil {
		return false
	}
	if threshold <= 0 {
		threshold = 10
	}
	if remainingTokens <= threshold {
		t.Status = StatusCooling
		return true
	}
	return false
}

func (t *TokenInfo) Clone() *TokenInfo {
	if t == nil {
		return nil
	}
	clone := *t
	if t.Tags != nil {
		clone.Tags = append([]string(nil), t.Tags...)
	}
	return &clone
}

func effortCost(effort EffortType) int {
	if cost, ok := EffortCost[effort]; ok {
		return cost
	}
	return EffortCost[EffortLow]
}

func nowMillis() int64 {
	return time.Now().UnixMilli()
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
