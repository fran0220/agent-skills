package token

import (
	"math/rand/v2"
	"sync"
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

// Select picks a random available token, excluding specified ones.
func (p *TokenPool) Select(exclude map[string]bool) *TokenInfo {
	if p == nil {
		return nil
	}

	p.mu.RLock()
	available := make([]*TokenInfo, 0, len(p.tokens))
	for _, token := range p.tokens {
		if exclude != nil && exclude[NormalizeToken(token.Token)] {
			continue
		}
		if token.IsAvailable() {
			available = append(available, token)
		}
	}
	p.mu.RUnlock()

	if len(available) == 0 {
		return nil
	}

	return available[rand.IntN(len(available))]
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
	return stats
}
