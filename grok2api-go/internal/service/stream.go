package service

import (
	"encoding/json"
	"log/slog"
	"math"
	"regexp"
	"strings"

	"github.com/fran0220/grok2api-go/internal/model"
	"github.com/fran0220/grok2api-go/internal/token"
)

var tokenSegmentRE = regexp.MustCompile(`\w+|[^\w\s]`)

const promptOverheadTokens = 4

// StreamChunk represents one streaming payload or terminal state.
type StreamChunk struct {
	Data string
	Err  error
	Done bool
}

// WrapStreamWithUsage records token usage after a successful stream completes.
func WrapStreamWithUsage(stream <-chan StreamChunk, tokenMgr *token.TokenManager, tokenValue, modelID string) <-chan StreamChunk {
	wrapped := make(chan StreamChunk)
	go func() {
		defer close(wrapped)
		success := false
		for chunk := range stream {
			if chunk.Err != nil {
				wrapped <- chunk
				return
			}
			if chunk.Done {
				success = true
			}
			wrapped <- chunk
		}
		if !success || tokenMgr == nil || tokenValue == "" {
			return
		}
		if err := tokenMgr.Consume(tokenValue, effortForModelID(modelID)); err != nil {
			slog.Warn("record stream usage failed", "model", modelID, "error", err)
		}
	}()
	return wrapped
}

func EstimateTokens(value string) int {
	text := strings.TrimSpace(value)
	if text == "" {
		return 0
	}
	byteEstimate := int(math.Ceil(float64(len([]byte(text))) / 4.0))
	segmentEstimate := int(math.Ceil(float64(len(tokenSegmentRE.FindAllString(text, -1))) * 0.75))
	result := maxInt(1, byteEstimate)
	return maxInt(result, segmentEstimate)
}

func EstimatePromptTokens(prompt string) int {
	if strings.TrimSpace(prompt) == "" {
		return 0
	}
	return EstimateTokens(prompt) + promptOverheadTokens
}

func EstimateCompletionTokens(content string, toolCalls []ToolCall) int {
	completionTokens := EstimateTokens(content)
	if len(toolCalls) == 0 {
		return completionTokens
	}
	payload, err := json.Marshal(toolCalls)
	if err != nil {
		return completionTokens + EstimateTokens("tool_calls")
	}
	return completionTokens + EstimateTokens(string(payload))
}

func BuildChatUsage(promptTokens, completionTokens int) map[string]any {
	if promptTokens < 0 {
		promptTokens = 0
	}
	if completionTokens < 0 {
		completionTokens = 0
	}
	totalTokens := promptTokens + completionTokens
	return map[string]any{
		"prompt_tokens":     promptTokens,
		"completion_tokens": completionTokens,
		"total_tokens":      totalTokens,
		"prompt_tokens_details": map[string]any{
			"cached_tokens": 0,
			"text_tokens":   promptTokens,
			"audio_tokens":  0,
			"image_tokens":  0,
		},
		"completion_tokens_details": map[string]any{
			"text_tokens":      completionTokens,
			"audio_tokens":     0,
			"reasoning_tokens": 0,
		},
	}
}

func effortForModelID(modelID string) token.EffortType {
	models := model.NewService()
	info, ok := models.Get(modelID)
	if ok && info.Cost == model.CostHigh {
		return token.EffortHigh
	}
	return token.EffortLow
}

func maxInt(left, right int) int {
	if left > right {
		return left
	}
	return right
}
