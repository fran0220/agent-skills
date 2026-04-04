package service

import (
	"context"
	"encoding/json"
	"errors"
	"log/slog"
	"strconv"
	"strings"
	"sync"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/token"
)

type UsageService struct {
	cfg      *config.Config
	reverse  *reverse.RateLimitsReverse
	limiter  chan struct{}
	limitCap int
}

var (
	usageServiceMu   sync.Mutex
	usageServiceInst *UsageService
)

func NewUsageService(cfg *config.Config) *UsageService {
	limit := 1
	if cfg != nil {
		limit = cfg.GetInt("usage.concurrent", 1)
	}
	if limit <= 0 {
		limit = 1
	}
	return &UsageService{
		cfg:      cfg,
		reverse:  reverse.NewRateLimitsReverse(),
		limiter:  make(chan struct{}, limit),
		limitCap: limit,
	}
}

func GetUsageService(cfg *config.Config) *UsageService {
	usageServiceMu.Lock()
	defer usageServiceMu.Unlock()
	limit := 1
	if cfg != nil {
		limit = cfg.GetInt("usage.concurrent", 1)
	}
	if limit <= 0 {
		limit = 1
	}
	if usageServiceInst == nil || usageServiceInst.limitCap != limit {
		usageServiceInst = NewUsageService(cfg)
	}
	return usageServiceInst
}

func ConfigureTokenRefresh(cfg *config.Config) {
	usageSvc := GetUsageService(cfg)
	token.SetCoolingTokenRefresher(func(tokenValue string) (token.TokenRefreshResult, error) {
		data, err := usageSvc.Get(tokenValue)
		if err != nil {
			var upstreamErr *reverse.UpstreamError
			if errors.As(err, &upstreamErr) && upstreamErr.IsTokenExpired {
				return token.TokenRefreshResult{Expired: true}, nil
			}
			return token.TokenRefreshResult{}, err
		}

		result := token.TokenRefreshResult{}
		if remaining, ok := anyInt(data["remainingTokens"]); ok {
			result.RemainingQuota = remaining
			result.HasRemainingQuota = true
		} else if remaining, ok := anyInt(data["remainingQueries"]); ok {
			result.RemainingQuota = remaining
			result.HasRemainingQuota = true
		}
		if windowSize, ok := anyInt(data["windowSizeSeconds"]); ok {
			result.WindowSizeSeconds = windowSize
			result.HasWindowSize = true
		}
		return result, nil
	})
}

func (s *UsageService) Get(tokenValue string) (map[string]any, error) {
	if s == nil {
		return nil, errors.New("usage service is nil")
	}
	return s.GetContext(context.Background(), tokenValue)
}

func (s *UsageService) GetContext(ctx context.Context, tokenValue string) (map[string]any, error) {
	if s == nil {
		return nil, errors.New("usage service is nil")
	}
	select {
	case s.limiter <- struct{}{}:
		defer func() { <-s.limiter }()
	case <-ctx.Done():
		return nil, ctx.Err()
	}

	session := reverse.NewResettableSession(reverse.SessionOptions{})
	defer session.Close()

	data, err := s.reverse.Request(ctx, session, tokenValue)
	if err != nil {
		return nil, err
	}
	if _, ok := data["remainingTokens"]; !ok {
		if remaining, ok := data["remainingQueries"]; ok {
			data["remainingTokens"] = remaining
		}
	}
	slog.Debug("usage sync success", "token", token.NormalizeToken(tokenValue), "remainingTokens", data["remainingTokens"])
	return data, nil
}

func anyInt(value any) (int, bool) {
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
	case float32:
		return int(typed), true
	case float64:
		return int(typed), true
	case json.Number:
		parsed, err := typed.Int64()
		if err == nil {
			return int(parsed), true
		}
	case string:
		parsed, err := strconv.Atoi(strings.TrimSpace(typed))
		if err == nil {
			return parsed, true
		}
	default:
		return 0, false
	}
	return 0, false
}
