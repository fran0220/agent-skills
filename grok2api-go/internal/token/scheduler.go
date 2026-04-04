package token

import (
	"log/slog"
	"sync"
	"time"

	"github.com/fran0220/grok2api-go/internal/config"
)

type TokenRefreshScheduler struct {
	manager *TokenManager

	mu      sync.Mutex
	running bool
	stopCh  chan struct{}
	doneCh  chan struct{}
}

var (
	schedulerOnce sync.Once
	schedulerInst *TokenRefreshScheduler
)

func GetScheduler() *TokenRefreshScheduler {
	schedulerOnce.Do(func() {
		schedulerInst = &TokenRefreshScheduler{manager: GetInstance()}
	})
	return schedulerInst
}

func (s *TokenRefreshScheduler) Start() {
	if s == nil {
		return
	}
	if !config.GetBool("token.auto_refresh") {
		slog.Info("token refresh scheduler disabled")
		return
	}

	s.mu.Lock()
	defer s.mu.Unlock()
	if s.running {
		return
	}
	s.running = true
	s.stopCh = make(chan struct{})
	s.doneCh = make(chan struct{})
	interval := s.refreshInterval()
	slog.Info("token refresh scheduler started", "interval", interval.String())
	go s.loop(interval, s.stopCh, s.doneCh)
}

func (s *TokenRefreshScheduler) Stop() {
	if s == nil {
		return
	}
	s.mu.Lock()
	if !s.running {
		s.mu.Unlock()
		return
	}
	stopCh := s.stopCh
	doneCh := s.doneCh
	s.running = false
	s.stopCh = nil
	s.doneCh = nil
	s.mu.Unlock()

	close(stopCh)
	<-doneCh
	slog.Info("token refresh scheduler stopped")
}

func (s *TokenRefreshScheduler) loop(interval time.Duration, stopCh <-chan struct{}, doneCh chan<- struct{}) {
	defer close(doneCh)
	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	s.runRefresh("scheduler_start")
	for {
		select {
		case <-stopCh:
			return
		case <-ticker.C:
			s.runRefresh("scheduler")
		}
	}
}

func (s *TokenRefreshScheduler) runRefresh(trigger string) {
	if s == nil || s.manager == nil {
		return
	}
	summary := s.manager.RefreshCoolingTokens(trigger, 0)
	slog.Info("token refresh completed",
		"trigger", trigger,
		"checked", summary["checked"],
		"refreshed", summary["refreshed"],
		"recovered", summary["recovered"],
		"expired", summary["expired"],
	)
}

func (s *TokenRefreshScheduler) refreshInterval() time.Duration {
	basicHours := refreshIntervalForPool(BasicPoolName)
	superHours := refreshIntervalForPool(SuperPoolName)
	if basicHours <= 0 {
		basicHours = DefaultRefreshIntervalHours
	}
	if superHours <= 0 {
		superHours = DefaultSuperRefreshIntervalHours
	}
	minHours := basicHours
	if superHours < minHours {
		minHours = superHours
	}
	if minHours <= 0 {
		minHours = DefaultSuperRefreshIntervalHours
	}
	return time.Duration(minHours) * time.Hour
}
