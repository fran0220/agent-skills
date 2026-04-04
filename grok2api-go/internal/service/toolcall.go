package service

import (
	"encoding/json"
	"fmt"
	"regexp"
	"strings"

	"github.com/google/uuid"
)

const (
	toolCallOpenTag  = "<tool_call>"
	toolCallCloseTag = "</tool_call>"
)

var toolCallBlockRE = regexp.MustCompile(`(?s)<tool_call>\s*(.*?)\s*</tool_call>`)

type Tool map[string]any
type ToolCall map[string]any

func BuildToolPrompt(tools []Tool, toolChoice any, parallelToolCalls bool) string {
	if len(tools) == 0 || isToolChoiceNone(toolChoice) {
		return ""
	}

	lines := []string{
		"# Available Tools",
		"",
		"You have access to the following tools. To call a tool, output a <tool_call> block with a JSON object containing \"name\" and \"arguments\".",
		"",
		"Format:",
		toolCallOpenTag,
		`{"name":"function_name","arguments":{"param":"value"}}`,
		toolCallCloseTag,
		"",
	}
	if parallelToolCalls {
		lines = append(lines,
			"You may make multiple tool calls in a single response by using multiple <tool_call> blocks.",
			"",
		)
	}
	lines = append(lines, "## Tool Definitions", "")
	for _, tool := range tools {
		if strings.TrimSpace(toolString(tool["type"])) != "function" {
			continue
		}
		function, _ := tool["function"].(map[string]any)
		name := strings.TrimSpace(toolString(function["name"]))
		if name == "" {
			continue
		}
		lines = append(lines, "### "+name)
		if description := strings.TrimSpace(toolString(function["description"])); description != "" {
			lines = append(lines, description)
		}
		if params := function["parameters"]; params != nil {
			lines = append(lines, "Parameters: "+marshalCompactJSON(params))
		}
		lines = append(lines, "")
	}

	switch choice := toolChoice.(type) {
	case string:
		if strings.EqualFold(strings.TrimSpace(choice), "required") {
			lines = append(lines, "IMPORTANT: You MUST call at least one tool in your response. Do not respond with only text.")
		} else {
			lines = append(lines, "Decide whether to call a tool based on the user's request. If you don't need a tool, respond normally with text only.")
		}
	case map[string]any:
		function, _ := choice["function"].(map[string]any)
		if forcedName := strings.TrimSpace(toolString(function["name"])); forcedName != "" {
			lines = append(lines, fmt.Sprintf("IMPORTANT: You MUST call the tool %q in your response.", forcedName))
		}
	default:
		lines = append(lines, "Decide whether to call a tool based on the user's request. If you don't need a tool, respond normally with text only.")
	}

	lines = append(lines,
		"",
		"When you call a tool, you may include text before or after the <tool_call> blocks, but each tool call block must contain valid JSON.",
	)
	return strings.Join(lines, "\n")
}

func ParseToolCalls(text string, tools []Tool) []ToolCall {
	_, toolCalls := extractToolCalls(text, tools)
	return toolCalls
}

func ParseToolCallBlock(raw string, tools []Tool) *ToolCall {
	parsed, ok := parseToolCallObject(raw)
	if !ok {
		return nil
	}
	name := strings.TrimSpace(toolString(parsed["name"]))
	if name == "" {
		return nil
	}
	if len(tools) > 0 && !toolExists(name, tools) {
		return nil
	}
	arguments := parsed["arguments"]
	argumentsText := "{}"
	if arguments != nil {
		argumentsText = marshalCompactJSON(arguments)
	}
	toolCall := ToolCall{
		"id":   "call_" + strings.ReplaceAll(uuid.NewString(), "-", "")[:24],
		"type": "function",
		"function": map[string]any{
			"name":      name,
			"arguments": argumentsText,
		},
	}
	return &toolCall
}

func FormatToolHistory(messages []Message) []Message {
	if len(messages) == 0 {
		return nil
	}
	formatted := make([]Message, 0, len(messages))
	for _, message := range messages {
		if message.Role == "assistant" && len(message.ToolCalls) > 0 {
			parts := make([]string, 0, len(message.ToolCalls)+1)
			if content := strings.TrimSpace(toolString(message.Content)); content != "" {
				parts = append(parts, content)
			}
			for _, call := range message.ToolCalls {
				function, _ := call["function"].(map[string]any)
				name := strings.TrimSpace(toolString(function["name"]))
				arguments := strings.TrimSpace(toolString(function["arguments"]))
				if name == "" {
					continue
				}
				if arguments == "" {
					arguments = "{}"
				}
				parts = append(parts, fmt.Sprintf("%s%s%s", toolCallOpenTag, marshalCompactJSON(map[string]any{"name": name, "arguments": json.RawMessage(arguments)}), toolCallCloseTag))
			}
			message.Content = strings.Join(parts, "\n")
			message.ToolCalls = nil
			formatted = append(formatted, message)
			continue
		}
		if message.Role == "tool" {
			toolName := strings.TrimSpace(message.Name)
			if toolName == "" {
				toolName = "unknown"
			}
			message.Role = "user"
			message.Content = fmt.Sprintf("tool (%s, %s): %s", toolName, strings.TrimSpace(message.ToolCallID), toolString(message.Content))
			message.ToolCalls = nil
			message.ToolCallID = ""
			message.Name = ""
		}
		formatted = append(formatted, message)
	}
	return formatted
}

func extractToolCalls(text string, tools []Tool) (string, []ToolCall) {
	trimmed := strings.TrimSpace(text)
	if trimmed == "" {
		return "", nil
	}
	matches := toolCallBlockRE.FindAllStringSubmatchIndex(text, -1)
	if len(matches) == 0 {
		return text, nil
	}
	toolCalls := make([]ToolCall, 0, len(matches))
	textParts := make([]string, 0, len(matches)+1)
	lastEnd := 0
	for _, match := range matches {
		if match[0] > lastEnd {
			if segment := strings.TrimSpace(text[lastEnd:match[0]]); segment != "" {
				textParts = append(textParts, segment)
			}
		}
		raw := text[match[2]:match[3]]
		if toolCall := ParseToolCallBlock(raw, tools); toolCall != nil {
			toolCalls = append(toolCalls, *toolCall)
		}
		lastEnd = match[1]
	}
	if lastEnd < len(text) {
		if segment := strings.TrimSpace(text[lastEnd:]); segment != "" {
			textParts = append(textParts, segment)
		}
	}
	if len(toolCalls) == 0 {
		return text, nil
	}
	return strings.Join(textParts, "\n"), toolCalls
}

func toolExists(name string, tools []Tool) bool {
	for _, tool := range tools {
		if strings.TrimSpace(toolString(tool["type"])) != "function" {
			continue
		}
		function, _ := tool["function"].(map[string]any)
		if strings.TrimSpace(toolString(function["name"])) == name {
			return true
		}
	}
	return false
}

func parseToolCallObject(raw string) (map[string]any, bool) {
	cleaned := strings.TrimSpace(stripCodeFences(raw))
	if cleaned == "" {
		return nil, false
	}
	var parsed map[string]any
	if err := json.Unmarshal([]byte(cleaned), &parsed); err == nil {
		return parsed, true
	}
	repaired := repairToolJSON(cleaned)
	if repaired == "" {
		return nil, false
	}
	if err := json.Unmarshal([]byte(repaired), &parsed); err != nil {
		return nil, false
	}
	return parsed, true
}

func stripCodeFences(value string) string {
	trimmed := strings.TrimSpace(value)
	if !strings.HasPrefix(trimmed, "```") {
		return trimmed
	}
	trimmed = regexp.MustCompile("^```[a-zA-Z0-9_-]*\\s*").ReplaceAllString(trimmed, "")
	trimmed = regexp.MustCompile("\\s*```$").ReplaceAllString(trimmed, "")
	return strings.TrimSpace(trimmed)
}

func repairToolJSON(value string) string {
	trimmed := strings.TrimSpace(value)
	start := strings.Index(trimmed, "{")
	end := strings.LastIndex(trimmed, "}")
	if start >= 0 {
		if end >= start {
			trimmed = trimmed[start : end+1]
		} else {
			trimmed = trimmed[start:]
		}
	}
	trimmed = strings.ReplaceAll(trimmed, "\r\n", "\n")
	trimmed = strings.ReplaceAll(trimmed, "\r", "\n")
	trimmed = strings.ReplaceAll(trimmed, "\n", " ")
	trimmed = regexp.MustCompile(`,\s*([}\]])`).ReplaceAllString(trimmed, `$1`)
	openCount := 0
	closeCount := 0
	inString := false
	escaped := false
	for _, r := range trimmed {
		if escaped {
			escaped = false
			continue
		}
		if inString && r == '\\' {
			escaped = true
			continue
		}
		if r == '"' {
			inString = !inString
			continue
		}
		if inString {
			continue
		}
		if r == '{' {
			openCount++
		}
		if r == '}' {
			closeCount++
		}
	}
	if openCount > closeCount {
		trimmed += strings.Repeat("}", openCount-closeCount)
	}
	return strings.TrimSpace(trimmed)
}

func marshalCompactJSON(value any) string {
	payload, err := json.Marshal(value)
	if err != nil {
		return strings.TrimSpace(fmt.Sprint(value))
	}
	return string(payload)
}

func toolString(value any) string {
	switch typed := value.(type) {
	case nil:
		return ""
	case string:
		return strings.TrimSpace(typed)
	default:
		return strings.TrimSpace(marshalCompactJSON(typed))
	}
}

func isToolChoiceNone(toolChoice any) bool {
	choice, ok := toolChoice.(string)
	return ok && strings.EqualFold(strings.TrimSpace(choice), "none")
}
