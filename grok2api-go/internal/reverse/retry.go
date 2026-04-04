package reverse

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"math/rand/v2"
	"net"
	"net/http"
	"net/url"
	"strconv"
	"time"
)

const transportRetryStatus = 502

var defaultRetryStatusCodes = []int{401, 429, 403, 502}

type RetryContext struct {
	Attempt       int
	MaxRetry      int
	RetryCodes    map[int]struct{}
	LastError     error
	LastStatus    *int
	TotalDelay    time.Duration
	RetryBudget   time.Duration
	BackoffBase   time.Duration
	BackoffFactor float64
	BackoffMax    time.Duration
	lastDelay     time.Duration
}

// RetryOptions controls generic retry behavior.
type RetryOptions struct {
	ExtractStatus func(error) *int
	OnRetry       func(context.Context, int, int, error, time.Duration)
}

// NewRetryContext builds retry state from config defaults.
func NewRetryContext() *RetryContext {
	base := time.Duration(getConfigFloat64("retry.retry_backoff_base", 0.5) * float64(time.Second))
	if base <= 0 {
		base = 500 * time.Millisecond
	}
	maxDelay := time.Duration(getConfigFloat64("retry.retry_backoff_max", 20) * float64(time.Second))
	if maxDelay <= 0 {
		maxDelay = 20 * time.Second
	}
	return &RetryContext{
		MaxRetry:      getConfigInt("retry.max_retry", 3),
		RetryCodes:    makeRetryCodeSet(getConfigIntSlice("retry.retry_status_codes", defaultRetryStatusCodes)),
		RetryBudget:   time.Duration(getConfigFloat64("retry.retry_budget", 60) * float64(time.Second)),
		BackoffBase:   base,
		BackoffFactor: getConfigFloat64("retry.retry_backoff_factor", 2.0),
		BackoffMax:    maxDelay,
		lastDelay:     base,
	}
}

func makeRetryCodeSet(codes []int) map[int]struct{} {
	result := make(map[int]struct{}, len(codes))
	for _, code := range codes {
		result[code] = struct{}{}
	}
	return result
}

// ShouldRetry reports whether the current error should trigger another attempt.
func (r *RetryContext) ShouldRetry(statusCode int, err error) bool {
	if r.Attempt >= r.MaxRetry {
		return false
	}
	if _, ok := r.RetryCodes[statusCode]; !ok {
		return false
	}
	if r.TotalDelay >= r.RetryBudget {
		return false
	}
	return true
}

func (r *RetryContext) RecordError(statusCode int, err error) {
	r.LastError = err
	r.LastStatus = &statusCode
	r.Attempt++
}

// CalculateDelay applies decorrelated jitter for 429 and full jitter otherwise.
func (r *RetryContext) CalculateDelay(statusCode int, retryAfter *float64) time.Duration {
	if retryAfter != nil && *retryAfter > 0 {
		delay := time.Duration(*retryAfter * float64(time.Second))
		if delay > r.BackoffMax {
			delay = r.BackoffMax
		}
		r.lastDelay = delay
		return delay
	}

	if statusCode == http.StatusTooManyRequests {
		minValue := r.BackoffBase
		maxValue := r.lastDelay * 3
		if maxValue < minValue {
			maxValue = minValue
		}
		delay := minValue
		if delta := maxValue - minValue; delta > 0 {
			delay += time.Duration(rand.Float64() * float64(delta))
		}
		if delay > r.BackoffMax {
			delay = r.BackoffMax
		}
		r.lastDelay = delay
		return delay
	}

	expDelay := float64(r.BackoffBase) * pow(r.BackoffFactor, r.Attempt)
	maxDelay := time.Duration(expDelay)
	if maxDelay > r.BackoffMax {
		maxDelay = r.BackoffMax
	}
	if maxDelay <= 0 {
		return 0
	}
	return time.Duration(rand.Float64() * float64(maxDelay))
}

func pow(base float64, exponent int) float64 {
	result := 1.0
	for range exponent {
		result *= base
	}
	return result
}

func (r *RetryContext) RecordDelay(delay time.Duration) {
	r.TotalDelay += delay
}

// ExtractRetryAfter gets Retry-After from an UpstreamError.
func ExtractRetryAfter(err error) *float64 {
	var upstreamErr *UpstreamError
	if !errors.As(err, &upstreamErr) {
		return nil
	}
	if upstreamErr.RetryAfter != nil {
		value := *upstreamErr.RetryAfter
		return &value
	}
	for _, key := range []string{"Retry-After", "retry-after"} {
		if value := upstreamErr.Headers.Get(key); value != "" {
			parsed, parseErr := strconv.ParseFloat(value, 64)
			if parseErr == nil {
				return &parsed
			}
		}
	}
	return nil
}

// ExtractStatusForRetry maps application and transport failures to retry codes.
func ExtractStatusForRetry(err error) *int {
	var upstreamErr *UpstreamError
	if errors.As(err, &upstreamErr) {
		status := upstreamErr.StatusCode
		return &status
	}

	if isTransportRetryError(err) {
		status := transportRetryStatus
		return &status
	}
	return nil
}

func isTransportRetryError(err error) bool {
	var netErr net.Error
	if errors.As(err, &netErr) {
		return true
	}
	var opErr *net.OpError
	if errors.As(err, &opErr) {
		return true
	}
	var urlErr *url.Error
	if errors.As(err, &urlErr) {
		return true
	}
	return false
}

// RetryOnStatus retries fn according to configured retry status extraction.
func RetryOnStatus(ctx context.Context, fn func(context.Context) (*http.Response, error), opts RetryOptions) (*http.Response, error) {
	retryCtx := NewRetryContext()
	extractStatus := opts.ExtractStatus
	if extractStatus == nil {
		extractStatus = ExtractStatusForRetry
	}

	for retryCtx.Attempt <= retryCtx.MaxRetry {
		response, err := fn(ctx)
		if err == nil {
			if retryCtx.Attempt > 0 {
				slog.Info("retry succeeded", "attempts", retryCtx.Attempt, "total_delay", retryCtx.TotalDelay.String())
			}
			return response, nil
		}

		statusCode := extractStatus(err)
		if statusCode == nil {
			slog.Error("non-retryable error", "type", fmt.Sprintf("%T", err), "error", err)
			return nil, err
		}

		retryCtx.RecordError(*statusCode, err)
		if !retryCtx.ShouldRetry(*statusCode, err) {
			if _, ok := retryCtx.RetryCodes[*statusCode]; ok {
				slog.Error("retry exhausted", "attempts", retryCtx.Attempt, "status", *statusCode, "total_delay", retryCtx.TotalDelay.String())
			} else {
				slog.Error("non-retryable status code", "status", *statusCode)
			}
			return nil, err
		}

		retryAfter := ExtractRetryAfter(err)
		delay := retryCtx.CalculateDelay(*statusCode, retryAfter)
		if retryCtx.TotalDelay+delay > retryCtx.RetryBudget {
			slog.Warn("retry budget exhausted", "current_delay", retryCtx.TotalDelay.String(), "next_delay", delay.String(), "budget", retryCtx.RetryBudget.String())
			return nil, err
		}

		retryCtx.RecordDelay(delay)
		slog.Warn("retrying request", "attempt", retryCtx.Attempt, "max_retry", retryCtx.MaxRetry, "status", *statusCode, "delay", delay.String(), "total_delay", retryCtx.TotalDelay.String())

		if opts.OnRetry != nil {
			opts.OnRetry(ctx, retryCtx.Attempt, *statusCode, err, delay)
		}

		timer := time.NewTimer(delay)
		select {
		case <-ctx.Done():
			if !timer.Stop() {
				<-timer.C
			}
			return nil, ctx.Err()
		case <-timer.C:
		}
	}

	return nil, fmt.Errorf("retry loop terminated unexpectedly")
}
