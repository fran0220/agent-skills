package service

import (
	"context"
	"fmt"
	"net/http"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/token"
)

type NSFWService struct {
	cfg       *config.Config
	semaphore chan struct{}
	acceptTOS *reverse.AcceptTOSReverse
	setBirth  *reverse.SetBirthReverse
	nsfw      *reverse.NsfwMgmtReverse
}

func NewNSFWService(cfg *config.Config) *NSFWService {
	concurrent := 60
	if cfg != nil {
		concurrent = max(1, cfg.GetInt("nsfw.concurrent", 60))
	}
	return &NSFWService{
		cfg:       cfg,
		semaphore: make(chan struct{}, concurrent),
		acceptTOS: reverse.NewAcceptTOSReverse(),
		setBirth:  reverse.NewSetBirthReverse(),
		nsfw:      reverse.NewNsfwMgmtReverse(),
	}
}

func (s *NSFWService) Batch(ctx context.Context, tokens []string, manager *token.TokenManager, task *BatchTask, onItem BatchItemCallback[map[string]any], shouldCancel func() bool) map[string]BatchResult[map[string]any] {
	if manager == nil {
		manager = token.GetInstance()
	}
	batchSize := 30
	if s.cfg != nil {
		batchSize = max(1, s.cfg.GetInt("nsfw.batch_size", 30))
	}
	return RunBatch(ctx, tokens, func(runCtx context.Context, tokenValue string) (map[string]any, error) {
		return s.enable(runCtx, tokenValue, manager)
	}, batchSize, task, onItem, shouldCancel)
}

func (s *NSFWService) enable(ctx context.Context, tokenValue string, manager *token.TokenManager) (map[string]any, error) {
	session := reverse.NewResettableSession(reverse.SessionOptions{Browser: s.cfg.GetString("proxy.browser", "")})
	defer session.Close()

	recordFail := func(err error, reason string) int {
		status := 0
		if upstreamErr, ok := err.(*reverse.UpstreamError); ok {
			status = upstreamErr.StatusCode
		}
		if status == http.StatusUnauthorized && manager != nil {
			_ = manager.RecordFail(tokenValue, status, reason)
		}
		return status
	}

	if err := s.withSlot(ctx, func(slotCtx context.Context) error {
		_, err := s.acceptTOS.Request(slotCtx, session, tokenValue)
		return err
	}); err != nil {
		status := recordFail(err, "tos_auth_failed")
		return map[string]any{"success": false, "http_status": status, "error": fmt.Sprintf("accept tos failed: %v", err)}, nil
	}

	if err := s.withSlot(ctx, func(slotCtx context.Context) error {
		return s.setBirth.Request(slotCtx, session, tokenValue)
	}); err != nil {
		status := recordFail(err, "set_birth_auth_failed")
		return map[string]any{"success": false, "http_status": status, "error": fmt.Sprintf("set birth failed: %v", err)}, nil
	}

	grpcStatus := reverse.GrpcStatus{Code: -1}
	if err := s.withSlot(ctx, func(slotCtx context.Context) error {
		status, err := s.nsfw.Request(slotCtx, session, tokenValue)
		grpcStatus = status
		return err
	}); err != nil {
		status := recordFail(err, "nsfw_mgmt_auth_failed")
		return map[string]any{"success": false, "http_status": status, "error": fmt.Sprintf("nsfw enable failed: %v", err)}, nil
	}

	if manager != nil {
		_ = manager.AddTag(tokenValue, "nsfw")
	}
	return map[string]any{
		"success":      grpcStatus.Code == -1 || grpcStatus.Code == 0,
		"http_status":  http.StatusOK,
		"grpc_status":  grpcStatus.Code,
		"grpc_message": emptyStringToNil(grpcStatus.Message),
		"error":        nil,
	}, nil
}

func (s *NSFWService) withSlot(ctx context.Context, fn func(context.Context) error) error {
	select {
	case <-ctx.Done():
		return ctx.Err()
	case s.semaphore <- struct{}{}:
	}
	defer func() { <-s.semaphore }()
	return fn(ctx)
}

func emptyStringToNil(value string) any {
	if value == "" {
		return nil
	}
	return value
}
