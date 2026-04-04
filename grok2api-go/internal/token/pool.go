package token

import (
	"math/rand/v2"
	"sync"

	"github.com/fran0220/grok2api-go/internal/config"
)

type TokenPool struct {
	name   string
	tokens map[string]*TokenInfo
	mu     sync.RWMutex
}

func NewTokenPool(name string) *TokenPool {
	return &TokenPool{
		name:   name,
		tokens: make(map[string]*TokenInfo),
	}
}

func (p *TokenPool) Name() string {
	if p == nil {
		return ""
	}
	return p.name
}

func (p *TokenPool) Add(token *TokenInfo) {
	if p == nil || token == nil {
		return
	}
	p.mu.Lock()
	defer p.mu.Unlock()
	p.tokens[NormalizeToken(token.Token)] = token
}

func (p *TokenPool) Remove(tokenStr string) bool {
	if p == nil {
		return false
	}
	tokenKey := NormalizeToken(tokenStr)
	p.mu.Lock()
	defer p.mu.Unlock()
	if _, ok := p.tokens[tokenKey]; !ok {
		return false
	}
	delete(p.tokens, tokenKey)
	return true
}

func (p *TokenPool) Get(tokenStr string) *TokenInfo {
	if p == nil {
		return nil
	}
	tokenKey := NormalizeToken(tokenStr)
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.tokens[tokenKey]
}

func (p *TokenPool) Select(exclude map[string]bool, preferTags map[string]bool) *TokenInfo {
	if p == nil {
		return nil
	}
	consumedMode := config.GetBool("token.consumed_mode_enabled")

	p.mu.RLock()
	available := make([]*TokenInfo, 0, len(p.tokens))
	for _, token := range p.tokens {
		if exclude != nil && exclude[NormalizeToken(token.Token)] {
			continue
		}
		if token.IsAvailable(consumedMode) {
			available = append(available, token)
		}
	}
	p.mu.RUnlock()

	if len(available) == 0 {
		return nil
	}

	if len(preferTags) > 0 {
		preferred := make([]*TokenInfo, 0, len(available))
		for _, token := range available {
			if hasAllTags(token.Tags, preferTags) {
				preferred = append(preferred, token)
			}
		}
		if len(preferred) > 0 {
			available = preferred
		}
	}

	best := make([]*TokenInfo, 0, len(available))
	for _, token := range available {
		if len(best) == 0 {
			best = append(best, token)
			continue
		}
		current := best[0]
		if consumedMode {
			switch {
			case token.Consumed < current.Consumed:
				best = []*TokenInfo{token}
			case token.Consumed == current.Consumed:
				best = append(best, token)
			}
			continue
		}
		switch {
		case token.Quota > current.Quota:
			best = []*TokenInfo{token}
		case token.Quota == current.Quota:
			best = append(best, token)
		}
	}

	return best[rand.IntN(len(best))]
}

func (p *TokenPool) Count() int {
	if p == nil {
		return 0
	}
	p.mu.RLock()
	defer p.mu.RUnlock()
	return len(p.tokens)
}

func (p *TokenPool) List() []*TokenInfo {
	if p == nil {
		return nil
	}
	p.mu.RLock()
	defer p.mu.RUnlock()
	items := make([]*TokenInfo, 0, len(p.tokens))
	for _, token := range p.tokens {
		items = append(items, token)
	}
	return items
}

func (p *TokenPool) GetStats() TokenPoolStats {
	stats := TokenPoolStats{}
	if p == nil {
		return stats
	}
	p.mu.RLock()
	defer p.mu.RUnlock()
	stats.Total = len(p.tokens)
	for _, token := range p.tokens {
		stats.TotalQuota += token.Quota
		stats.TotalConsumed += token.Consumed
		switch token.Status {
		case StatusActive:
			stats.Active++
		case StatusDisabled:
			stats.Disabled++
		case StatusExpired:
			stats.Expired++
		case StatusCooling:
			stats.Cooling++
		}
	}
	if stats.Total > 0 {
		stats.AvgQuota = float64(stats.TotalQuota) / float64(stats.Total)
		stats.AvgConsumed = float64(stats.TotalConsumed) / float64(stats.Total)
	}
	return stats
}

func (p *TokenPool) RebuildIndex() {
	// Reserved for future indexing. The Python version keeps this hook for
	// parity after bulk loads.
}

func hasAllTags(tags []string, required map[string]bool) bool {
	if len(required) == 0 {
		return true
	}
	available := make(map[string]bool, len(tags))
	for _, tag := range tags {
		available[tag] = true
	}
	for tag := range required {
		if !available[tag] {
			return false
		}
	}
	return true
}
