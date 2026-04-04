package service

import (
	"strings"
	"testing"
)

func TestParseToolCallBlockRepairsJSON(t *testing.T) {
	tools := []Tool{{
		"type": "function",
		"function": map[string]any{
			"name": "weather",
		},
	}}

	toolCall := ParseToolCallBlock("```json\n{\"name\":\"weather\",\"arguments\":{\"city\":\"SF\",}}\n```", tools)
	if toolCall == nil {
		t.Fatal("expected tool call to be parsed")
	}
	function, _ := (*toolCall)["function"].(map[string]any)
	if got := function["name"]; got != "weather" {
		t.Fatalf("expected function name weather, got %v", got)
	}
	arguments, _ := function["arguments"].(string)
	if !strings.Contains(arguments, `"city":"SF"`) {
		t.Fatalf("expected repaired arguments to contain city, got %s", arguments)
	}
}

func TestFormatToolHistoryConvertsAssistantAndToolMessages(t *testing.T) {
	messages := []Message{
		{
			Role:    "assistant",
			Content: "Let me call a tool",
			ToolCalls: []ToolCall{{
				"function": map[string]any{
					"name":      "weather",
					"arguments": `{"city":"SF"}`,
				},
			}},
		},
		{
			Role:       "tool",
			Name:       "weather",
			ToolCallID: "call_123",
			Content:    `{"forecast":"sunny"}`,
		},
	}

	formatted := FormatToolHistory(messages)
	if len(formatted) != 2 {
		t.Fatalf("expected 2 formatted messages, got %d", len(formatted))
	}
	assistantContent, _ := formatted[0].Content.(string)
	if !strings.Contains(assistantContent, toolCallOpenTag) || !strings.Contains(assistantContent, `"name":"weather"`) {
		t.Fatalf("expected assistant message to include serialized tool call, got %s", assistantContent)
	}
	if formatted[1].Role != "user" {
		t.Fatalf("expected tool message to become user role, got %s", formatted[1].Role)
	}
	toolContent, _ := formatted[1].Content.(string)
	if !strings.Contains(toolContent, "tool (weather, call_123)") {
		t.Fatalf("expected tool history text, got %s", toolContent)
	}
}

func TestProcessorStateHandlesSplitToolCallStream(t *testing.T) {
	tools := []Tool{{
		"type": "function",
		"function": map[string]any{
			"name": "weather",
		},
	}}
	state := newProcessorState(nil, "grok-4", 10, tools)
	contentChunks := make([]string, 0)
	parsedCalls := make([]ToolCall, 0)
	state.callbacks = processorCallbacks{
		EmitContent: func(content string) error {
			contentChunks = append(contentChunks, content)
			return nil
		},
		EmitToolCall: func(toolCall ToolCall, index int) error {
			parsedCalls = append(parsedCalls, toolCall)
			return nil
		},
	}

	parts := []string{
		"Before <tool_",
		"call>{\"name\":\"weather\",\"arguments\":{\"city\":\"SF\"}}",
		"</tool_call> after",
	}
	for _, part := range parts {
		if err := state.handleModelToken(part); err != nil {
			t.Fatalf("handleModelToken failed: %v", err)
		}
	}
	if err := state.flushToolRemainder(true); err != nil {
		t.Fatalf("flushToolRemainder failed: %v", err)
	}

	if len(parsedCalls) != 1 {
		t.Fatalf("expected 1 parsed tool call, got %d", len(parsedCalls))
	}
	function, _ := parsedCalls[0]["function"].(map[string]any)
	if got := function["name"]; got != "weather" {
		t.Fatalf("expected function weather, got %v", got)
	}
	if got := strings.Join(contentChunks, ""); got != "Before  after" {
		t.Fatalf("expected surrounding text to be preserved, got %q", got)
	}
	if got := state.content.String(); got != "Before  after" {
		t.Fatalf("expected collected content to match emitted content, got %q", got)
	}
}
