package main

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"syscall"
	"time"

	"github.com/fran0220/grok2api-go/api"
	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/token"
)

func main() {
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo}))
	slog.SetDefault(logger)

	rootDir, err := os.Getwd()
	if err != nil {
		logger.Error("resolve working directory", slog.Any("error", err))
		os.Exit(1)
	}

	cfg := config.New(rootDir)
	if err := cfg.Load(); err != nil {
		logger.Error("load config", slog.Any("error", err))
		os.Exit(1)
	}

	host := envString("SERVER_HOST", "0.0.0.0")
	port := envInt("SERVER_PORT", 8000)
	workers := envInt("SERVER_WORKERS", 1)
	addr := net.JoinHostPort(host, strconv.Itoa(port))

	server := &http.Server{
		Addr:              addr,
		Handler:           api.SetupRouter(cfg, logger),
		ReadHeaderTimeout: 5 * time.Second,
	}
	scheduler := token.GetScheduler()
	scheduler.Start()

	logger.Info("starting grok2api-go server",
		slog.String("addr", addr),
		slog.Int("workers", workers),
		slog.String("defaults_config", cfg.Paths().Defaults),
		slog.String("override_config", cfg.Paths().Override),
	)

	errCh := make(chan error, 1)
	go func() {
		if err := server.ListenAndServe(); err != nil && !errors.Is(err, http.ErrServerClosed) {
			errCh <- err
		}
	}()

	shutdownCtx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	select {
	case err := <-errCh:
		scheduler.Stop()
		logger.Error("server exited unexpectedly", slog.Any("error", err))
		os.Exit(1)
	case <-shutdownCtx.Done():
		logger.Info("shutdown signal received")
	}
	scheduler.Stop()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	if err := server.Shutdown(ctx); err != nil {
		logger.Error("graceful shutdown failed", slog.Any("error", err))
		os.Exit(1)
	}

	logger.Info("server stopped")
}

func envString(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}

func envInt(key string, fallback int) int {
	value := os.Getenv(key)
	if value == "" {
		return fallback
	}
	parsed, err := strconv.Atoi(value)
	if err != nil {
		slog.Warn(fmt.Sprintf("invalid %s, using fallback", key), slog.String("value", value), slog.Int("fallback", fallback))
		return fallback
	}
	return parsed
}
