package service

import (
	"context"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/reverse"
)

type VoiceService struct {
	cfg     *config.Config
	reverse *reverse.LivekitTokenReverse
}

func NewVoiceService(cfg *config.Config) *VoiceService {
	return &VoiceService{cfg: cfg, reverse: reverse.NewLivekitTokenReverse()}
}

func (s *VoiceService) GetToken(ctx context.Context, tokenValue, voice, personality string, speed float64) (map[string]any, error) {
	session := reverse.NewResettableSession(reverse.SessionOptions{Browser: s.cfg.GetString("proxy.browser", "")})
	defer session.Close()
	return s.reverse.Request(ctx, session, tokenValue, voice, personality, speed)
}
