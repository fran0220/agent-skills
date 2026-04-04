package service

import (
	"context"
	"encoding/json"
	"fmt"
	"sort"
	"strings"
	"time"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/model"
)

var responseToolOutputTypes = map[string]bool{
	"tool_output":          true,
	"function_call_output": true,
	"tool_call_output":     true,
	"input_tool_output":    true,
}

var responseBuiltinToolTypes = map[string]bool{
	"web_search":            true,
	"web_search_2025_08_26": true,
	"file_search":           true,
	"code_interpreter":      true,
}

type ResponsesCreateParams struct {
	Model              string
	Input              any
	Instructions       string
	Stream             bool
	Temperature        *float64
	TopP               *float64
	Tools              []map[string]any
	ToolChoice         any
	ParallelToolCalls  *bool
	ReasoningEffort    string
	MaxOutputTokens    *int
	Metadata           map[string]any
	User               string
	Store              *bool
	PreviousResponseID string
	Truncation         string
}

type ResponsesResult struct {
	StreamData <-chan string
	Response   map[string]any
}

type ResponsesService struct {
	chat *ChatService
}

func NewResponsesService(cfg *config.Config, models *model.Service) *ResponsesService {
	return &ResponsesService{chat: NewChatService(cfg, models)}
}

func (s *ResponsesService) Create(ctx context.Context, params ResponsesCreateParams) (*ResponsesResult, error) {
	messages := coerceResponsesInputToMessages(params.Input)
	if strings.TrimSpace(params.Instructions) != "" {
		messages = append([]Message{{Role: "system", Content: params.Instructions}}, messages...)
	}
	if len(messages) == 0 {
		return nil, NewValidationError("input is required", "invalid_request_error")
	}

	chatOpts := ChatOptions{Stream: params.Stream, Temperature: 1.0, TopP: 1.0, ParallelToolCalls: true}
	if params.Temperature != nil {
		chatOpts.Temperature = *params.Temperature
	}
	if params.TopP != nil {
		chatOpts.TopP = *params.TopP
	}
	if params.ParallelToolCalls != nil {
		chatOpts.ParallelToolCalls = *params.ParallelToolCalls
	}
	if strings.TrimSpace(params.ReasoningEffort) != "" {
		chatOpts.ReasoningEffort = params.ReasoningEffort
	}
	chatOpts.Tools = normalizeResponsesToolsForChat(params.Tools)
	chatOpts.ToolChoice = normalizeResponsesToolChoice(params.ToolChoice)

	chatResult, err := s.chat.Completions(ctx, params.Model, messages, chatOpts)
	if err != nil {
		return nil, err
	}
	if !params.Stream {
		if chatResult == nil || chatResult.Response == nil {
			return nil, NewStatusError(500, "Unexpected stream response for non-stream request", "server_error", "internal_error", nil)
		}
		choice := responsesMapAt(chatResult.Response["choices"], 0)
		message := mapValue(choice["message"])
		content := stringValue(message["content"])
		toolCalls := responsesToToolCallSlice(message["tool_calls"])
		return &ResponsesResult{Response: buildResponsesObject(responsesObjectParams{
			Model:              params.Model,
			OutputText:         content,
			ToolCalls:          toolCalls,
			Usage:              toResponsesUsage(mapValue(chatResult.Response["usage"])),
			Instructions:       params.Instructions,
			MaxOutputTokens:    params.MaxOutputTokens,
			ParallelToolCalls:  params.ParallelToolCalls,
			PreviousResponseID: params.PreviousResponseID,
			ReasoningEffort:    params.ReasoningEffort,
			Store:              params.Store,
			Temperature:        params.Temperature,
			ToolChoice:         params.ToolChoice,
			Tools:              params.Tools,
			TopP:               params.TopP,
			Truncation:         params.Truncation,
			User:               params.User,
			Metadata:           params.Metadata,
		})}, nil
	}
	if chatResult == nil || chatResult.Stream == nil {
		return nil, NewStatusError(500, "Unexpected non-stream response for stream request", "server_error", "internal_error", nil)
	}

	adapter := newResponsesStreamAdapter(responsesStreamParams{
		Model:              params.Model,
		Instructions:       params.Instructions,
		MaxOutputTokens:    params.MaxOutputTokens,
		ParallelToolCalls:  params.ParallelToolCalls,
		PreviousResponseID: params.PreviousResponseID,
		ReasoningEffort:    params.ReasoningEffort,
		Store:              params.Store,
		Temperature:        params.Temperature,
		ToolChoice:         params.ToolChoice,
		Tools:              params.Tools,
		TopP:               params.TopP,
		Truncation:         params.Truncation,
		User:               params.User,
		Metadata:           params.Metadata,
	})

	stream := make(chan string, 16)
	go func() {
		defer close(stream)
		stream <- adapter.createdEvent()
		stream <- adapter.inProgressEvent()
		var finalUsage map[string]any
		for chunk := range chatResult.Stream {
			if chunk.Err != nil {
				stream <- sseEvent("error", AsChatError(chunk.Err).SSEErrorPayload())
				return
			}
			if chunk.Done {
				break
			}
			var payload map[string]any
			if err := json.Unmarshal([]byte(strings.TrimSpace(chunk.Data)), &payload); err != nil {
				continue
			}
			if stringValue(payload["object"]) != "chat.completion.chunk" {
				continue
			}
			if usage := mapValue(payload["usage"]); usage != nil {
				finalUsage = toResponsesUsage(usage)
			}
			choice := responsesMapAt(payload["choices"], 0)
			delta := mapValue(choice["delta"])
			if content := stringValue(delta["content"]); content != "" {
				for _, event := range adapter.ensureMessageStarted() {
					stream <- event
				}
				adapter.outputTextParts = append(adapter.outputTextParts, content)
				stream <- adapter.outputDeltaEvent(content)
			}
			for _, tool := range responsesToToolCallSlice(delta["tool_calls"]) {
				toolIndex, _ := anyInt(tool["index"])
				callID := stringValue(tool["id"])
				if callID == "" {
					callID = newResponsesToolCallID()
				}
				fn := mapValue(tool["function"])
				name := stringValue(fn["name"])
				argumentsDelta := stringValue(fn["arguments"])
				adapter.recordToolCall(toolIndex, callID, name, argumentsDelta)
				for _, event := range adapter.ensureToolItem(toolIndex, callID, name) {
					stream <- event
				}
				if event := adapter.toolArgumentsDeltaEvent(toolIndex, argumentsDelta); event != "" {
					stream <- event
				}
			}
		}
		fullText := strings.Join(adapter.outputTextParts, "")
		if fullText != "" && adapter.messageStarted {
			for _, event := range adapter.outputDoneEvents(fullText) {
				stream <- event
			}
		}
		for _, event := range adapter.toolArgumentsDoneEvents() {
			stream <- event
		}
		stream <- adapter.completedEvent(finalUsage)
	}()
	return &ResponsesResult{StreamData: stream}, nil
}

func normalizeResponsesToolChoice(toolChoice any) any {
	mapped := mapValue(toolChoice)
	if mapped == nil {
		return toolChoice
	}
	toolType := stringValue(mapped["type"])
	if toolType != "" && toolType != "function" {
		return map[string]any{"type": "function", "function": map[string]any{"name": toolType}}
	}
	return toolChoice
}

func normalizeResponsesToolsForChat(tools []map[string]any) []Tool {
	if len(tools) == 0 {
		return nil
	}
	normalized := make([]Tool, 0, len(tools))
	for _, tool := range tools {
		toolType := stringValue(tool["type"])
		if toolType == "function" {
			normalized = append(normalized, Tool(tool))
			continue
		}
		if !responseBuiltinToolTypes[toolType] {
			continue
		}
		description := ""
		parameters := map[string]any{"type": "object", "properties": map[string]any{}, "required": []string{}}
		switch {
		case strings.HasPrefix(toolType, "web_search"):
			description = "Search the web for information and return results."
			parameters = map[string]any{"type": "object", "properties": map[string]any{"query": map[string]any{"type": "string"}}, "required": []string{"query"}}
		case toolType == "file_search":
			description = "Search provided files for relevant information."
			parameters = map[string]any{"type": "object", "properties": map[string]any{"query": map[string]any{"type": "string"}}, "required": []string{"query"}}
		case toolType == "code_interpreter":
			description = "Execute code to solve tasks and return results."
			parameters = map[string]any{"type": "object", "properties": map[string]any{"code": map[string]any{"type": "string"}}, "required": []string{"code"}}
		}
		normalized = append(normalized, Tool(map[string]any{
			"type": "function",
			"function": map[string]any{
				"name":        toolType,
				"description": description,
				"parameters":  parameters,
			},
		}))
	}
	if len(normalized) == 0 {
		return nil
	}
	return normalized
}

type responsesNormalizedInput struct {
	Kind    string
	Message Message
	Block   map[string]any
}

func coerceResponsesInputToMessages(input any) []Message {
	if input == nil {
		return nil
	}
	if text, ok := input.(string); ok {
		return []Message{{Role: "user", Content: text}}
	}
	if mapped := mapValue(input); mapped != nil {
		normalized := normalizeResponsesInputItem(mapped)
		if normalized == nil {
			return nil
		}
		switch normalized.Kind {
		case "message", "tool":
			return []Message{normalized.Message}
		case "block":
			return []Message{{Role: "user", Content: []map[string]any{normalized.Block}}}
		}
	}
	items, ok := input.([]any)
	if !ok {
		return []Message{{Role: "user", Content: fmt.Sprint(input)}}
	}
	messages := make([]Message, 0)
	pendingBlocks := make([]map[string]any, 0)
	flush := func() {
		if len(pendingBlocks) == 0 {
			return
		}
		copyBlocks := make([]map[string]any, 0, len(pendingBlocks))
		copyBlocks = append(copyBlocks, pendingBlocks...)
		messages = append(messages, Message{Role: "user", Content: copyBlocks})
		pendingBlocks = pendingBlocks[:0]
	}
	for _, item := range items {
		normalized := normalizeResponsesInputItem(item)
		if normalized == nil {
			continue
		}
		switch normalized.Kind {
		case "message", "tool":
			flush()
			messages = append(messages, normalized.Message)
		case "block":
			pendingBlocks = append(pendingBlocks, normalized.Block)
		}
	}
	flush()
	return messages
}

func normalizeResponsesInputItem(item any) *responsesNormalizedInput {
	if item == nil {
		return nil
	}
	if text, ok := item.(string); ok {
		return &responsesNormalizedInput{Kind: "block", Block: map[string]any{"type": "text", "text": text}}
	}
	mapped := mapValue(item)
	if mapped == nil {
		return nil
	}
	itemType := stringValue(mapped["type"])
	if itemType == "message" || (mapped["role"] != nil && mapped["content"] != nil) {
		role := stringValue(mapped["role"])
		if role == "" {
			role = "user"
		}
		return &responsesNormalizedInput{Kind: "message", Message: Message{Role: role, Content: normalizeResponsesContent(mapped["content"])}}
	}
	if responseToolOutputTypes[itemType] {
		callID := stringValue(mapped["call_id"])
		if callID == "" {
			callID = stringValue(mapped["tool_call_id"])
		}
		if callID == "" {
			callID = stringValue(mapped["id"])
		}
		if callID == "" {
			callID = newResponsesToolCallID()
		}
		output := mapped["output"]
		if output == nil {
			output = mapped["content"]
		}
		return &responsesNormalizedInput{Kind: "tool", Message: Message{Role: "tool", ToolCallID: callID, Content: stringify(output)}}
	}
	if itemType == "input_text" || itemType == "text" || itemType == "output_text" {
		text := stringValue(mapped["text"])
		if text == "" {
			text = stringify(mapped["content"])
		}
		return &responsesNormalizedInput{Kind: "block", Block: map[string]any{"type": "text", "text": text}}
	}
	if itemType == "input_image" || itemType == "image" || itemType == "image_url" || itemType == "output_image" {
		payload := map[string]any{}
		if imageURL := mapValue(mapped["image_url"]); imageURL != nil {
			payload["url"] = stringValue(imageURL["url"])
			if detail := stringValue(imageURL["detail"]); detail != "" {
				payload["detail"] = detail
			}
		} else if raw := stringValue(mapped["image_url"]); raw != "" {
			payload["url"] = raw
		} else if raw := stringValue(mapped["url"]); raw != "" {
			payload["url"] = raw
		} else if raw := stringValue(mapped["image"]); raw != "" {
			payload["url"] = raw
		}
		if strings.TrimSpace(stringValue(payload["url"])) == "" {
			return nil
		}
		return &responsesNormalizedInput{Kind: "block", Block: map[string]any{"type": "image_url", "image_url": payload}}
	}
	if itemType == "input_file" || itemType == "file" {
		filePayload := map[string]any{}
		if fileData := stringValue(mapped["file_data"]); fileData != "" {
			filePayload["file_data"] = fileData
		}
		if fileID := stringValue(mapped["file_id"]); fileID != "" {
			filePayload["file_id"] = fileID
		}
		if len(filePayload) == 0 {
			if nested := mapValue(mapped["file"]); nested != nil {
				if fileData := stringValue(nested["file_data"]); fileData != "" {
					filePayload["file_data"] = fileData
				}
				if fileID := stringValue(nested["file_id"]); fileID != "" {
					filePayload["file_id"] = fileID
				}
			}
		}
		if len(filePayload) == 0 {
			return nil
		}
		return &responsesNormalizedInput{Kind: "block", Block: map[string]any{"type": "file", "file": filePayload}}
	}
	if itemType == "input_audio" || itemType == "audio" {
		data := ""
		if nested := mapValue(mapped["audio"]); nested != nil {
			data = stringValue(nested["data"])
		}
		if data == "" {
			data = stringValue(mapped["data"])
		}
		if data == "" {
			return nil
		}
		return &responsesNormalizedInput{Kind: "block", Block: map[string]any{"type": "input_audio", "input_audio": map[string]any{"data": data}}}
	}
	return nil
}

func normalizeResponsesContent(content any) any {
	if content == nil {
		return ""
	}
	if text, ok := content.(string); ok {
		return text
	}
	if mapped := mapValue(content); mapped != nil {
		content = []any{mapped}
	}
	items, ok := content.([]any)
	if !ok {
		return fmt.Sprint(content)
	}
	blocks := make([]map[string]any, 0, len(items))
	for _, item := range items {
		normalized := normalizeResponsesInputItem(item)
		if normalized != nil && normalized.Kind == "block" {
			blocks = append(blocks, normalized.Block)
		}
	}
	if len(blocks) == 0 {
		return ""
	}
	return blocks
}

type responsesObjectParams struct {
	Model              string
	OutputText         string
	ToolCalls          []map[string]any
	ResponseID         string
	Usage              map[string]any
	CreatedAt          int64
	CompletedAt        int64
	Status             string
	Instructions       string
	MaxOutputTokens    *int
	ParallelToolCalls  *bool
	PreviousResponseID string
	ReasoningEffort    string
	Store              *bool
	Temperature        *float64
	ToolChoice         any
	Tools              []map[string]any
	TopP               *float64
	Truncation         string
	User               string
	Metadata           map[string]any
}

func buildResponsesObject(params responsesObjectParams) map[string]any {
	responseID := params.ResponseID
	if responseID == "" {
		responseID = newResponsesResponseID()
	}
	createdAt := params.CreatedAt
	if createdAt == 0 {
		createdAt = time.Now().Unix()
	}
	status := params.Status
	if status == "" {
		status = "completed"
	}
	completedAt := params.CompletedAt
	if status == "completed" && completedAt == 0 {
		completedAt = time.Now().Unix()
	}
	parallelToolCalls := true
	if params.ParallelToolCalls != nil {
		parallelToolCalls = *params.ParallelToolCalls
	}
	store := true
	if params.Store != nil {
		store = *params.Store
	}
	temperature := 1.0
	if params.Temperature != nil {
		temperature = *params.Temperature
	}
	topP := 1.0
	if params.TopP != nil {
		topP = *params.TopP
	}
	toolChoice := params.ToolChoice
	if toolChoice == nil || toolChoice == "" {
		toolChoice = "auto"
	}
	truncation := params.Truncation
	if truncation == "" {
		truncation = "disabled"
	}
	tools := params.Tools
	if tools == nil {
		tools = []map[string]any{}
	}
	metadata := params.Metadata
	if metadata == nil {
		metadata = map[string]any{}
	}
	output := make([]map[string]any, 0, 1+len(params.ToolCalls))
	if params.OutputText != "" || len(params.ToolCalls) == 0 {
		output = append(output, buildResponsesOutputMessage(params.OutputText, "", "completed"))
	}
	for _, call := range params.ToolCalls {
		output = append(output, buildResponsesOutputToolCall(call, "", "completed"))
	}
	return map[string]any{
		"id":                   responseID,
		"object":               "response",
		"created_at":           createdAt,
		"completed_at":         completedAt,
		"status":               status,
		"error":                nil,
		"incomplete_details":   nil,
		"instructions":         responsesEmptyToNil(params.Instructions),
		"max_output_tokens":    params.MaxOutputTokens,
		"model":                params.Model,
		"output":               output,
		"parallel_tool_calls":  parallelToolCalls,
		"previous_response_id": responsesEmptyToNil(params.PreviousResponseID),
		"reasoning": map[string]any{
			"effort":  responsesEmptyToNil(params.ReasoningEffort),
			"summary": nil,
		},
		"store":       store,
		"temperature": temperature,
		"text":        map[string]any{"format": map[string]any{"type": "text"}},
		"tool_choice": toolChoice,
		"tools":       tools,
		"top_p":       topP,
		"truncation":  truncation,
		"usage":       params.Usage,
		"user":        responsesEmptyToNil(params.User),
		"metadata":    metadata,
	}
}

func buildResponsesOutputMessage(text, messageID, status string) map[string]any {
	if messageID == "" {
		messageID = newResponsesMessageID()
	}
	if status == "" {
		status = "completed"
	}
	return map[string]any{
		"id":     messageID,
		"type":   "message",
		"role":   "assistant",
		"status": status,
		"content": []map[string]any{{
			"type":        "output_text",
			"text":        text,
			"annotations": []any{},
		}},
	}
}

func buildResponsesOutputToolCall(toolCall map[string]any, itemID, status string) map[string]any {
	fn := mapValue(toolCall["function"])
	callID := stringValue(toolCall["id"])
	if callID == "" {
		callID = newResponsesToolCallID()
	}
	if itemID == "" {
		itemID = newResponsesFunctionCallID()
	}
	if status == "" {
		status = "completed"
	}
	return map[string]any{
		"id":        itemID,
		"type":      "function_call",
		"status":    status,
		"call_id":   callID,
		"name":      responsesEmptyToNil(stringValue(fn["name"])),
		"arguments": stringValue(fn["arguments"]),
	}
}

type responsesStreamParams struct {
	Model              string
	Instructions       string
	MaxOutputTokens    *int
	ParallelToolCalls  *bool
	PreviousResponseID string
	ReasoningEffort    string
	Store              *bool
	Temperature        *float64
	ToolChoice         any
	Tools              []map[string]any
	TopP               *float64
	Truncation         string
	User               string
	Metadata           map[string]any
}

type responsesStreamAdapter struct {
	params             responsesStreamParams
	responseID         string
	createdAt          int64
	outputTextParts    []string
	toolCallsByIndex   map[int]map[string]any
	toolItems          map[int]map[string]any
	nextOutputIndex    int
	contentIndex       int
	messageID          string
	messageStarted     bool
	messageOutputIndex int
}

func newResponsesStreamAdapter(params responsesStreamParams) *responsesStreamAdapter {
	return &responsesStreamAdapter{
		params:             params,
		responseID:         newResponsesResponseID(),
		createdAt:          time.Now().Unix(),
		toolCallsByIndex:   make(map[int]map[string]any),
		toolItems:          make(map[int]map[string]any),
		messageID:          newResponsesMessageID(),
		messageOutputIndex: -1,
	}
}

func (a *responsesStreamAdapter) event(eventType string, payload map[string]any) string {
	data, _ := json.Marshal(payload)
	return fmt.Sprintf("event: %s\ndata: %s\n\n", eventType, data)
}

func (a *responsesStreamAdapter) responsePayload(status string, outputText string, usage map[string]any) map[string]any {
	toolCalls := make([]map[string]any, 0, len(a.toolCallsByIndex))
	if status == "completed" && len(a.toolCallsByIndex) > 0 {
		indexes := make([]int, 0, len(a.toolCallsByIndex))
		for index := range a.toolCallsByIndex {
			indexes = append(indexes, index)
		}
		sort.Ints(indexes)
		for _, index := range indexes {
			toolCalls = append(toolCalls, a.toolCallsByIndex[index])
		}
	}
	return buildResponsesObject(responsesObjectParams{
		Model:              a.params.Model,
		OutputText:         outputText,
		ToolCalls:          toolCalls,
		ResponseID:         a.responseID,
		Usage:              usage,
		CreatedAt:          a.createdAt,
		Status:             status,
		Instructions:       a.params.Instructions,
		MaxOutputTokens:    a.params.MaxOutputTokens,
		ParallelToolCalls:  a.params.ParallelToolCalls,
		PreviousResponseID: a.params.PreviousResponseID,
		ReasoningEffort:    a.params.ReasoningEffort,
		Store:              a.params.Store,
		Temperature:        a.params.Temperature,
		ToolChoice:         a.params.ToolChoice,
		Tools:              a.params.Tools,
		TopP:               a.params.TopP,
		Truncation:         a.params.Truncation,
		User:               a.params.User,
		Metadata:           a.params.Metadata,
	})
}

func (a *responsesStreamAdapter) createdEvent() string {
	return a.event("response.created", map[string]any{"type": "response.created", "response": a.responsePayload("in_progress", "", nil)})
}

func (a *responsesStreamAdapter) inProgressEvent() string {
	return a.event("response.in_progress", map[string]any{"type": "response.in_progress", "response": a.responsePayload("in_progress", "", nil)})
}

func (a *responsesStreamAdapter) allocOutputIndex() int {
	idx := a.nextOutputIndex
	a.nextOutputIndex++
	return idx
}

func (a *responsesStreamAdapter) ensureMessageStarted() []string {
	if a.messageStarted {
		return nil
	}
	a.messageStarted = true
	a.messageOutputIndex = a.allocOutputIndex()
	item := buildResponsesOutputMessage("", a.messageID, "in_progress")
	item["content"] = []any{}
	return []string{
		a.event("response.output_item.added", map[string]any{"type": "response.output_item.added", "response_id": a.responseID, "output_index": a.messageOutputIndex, "item": item}),
		a.event("response.content_part.added", map[string]any{"type": "response.content_part.added", "response_id": a.responseID, "item_id": a.messageID, "output_index": a.messageOutputIndex, "content_index": a.contentIndex, "part": map[string]any{"type": "output_text", "text": "", "annotations": []any{}}}),
	}
}

func (a *responsesStreamAdapter) outputDeltaEvent(delta string) string {
	return a.event("response.output_text.delta", map[string]any{"type": "response.output_text.delta", "response_id": a.responseID, "item_id": a.messageID, "output_index": a.messageOutputIndex, "content_index": a.contentIndex, "delta": delta})
}

func (a *responsesStreamAdapter) outputDoneEvents(text string) []string {
	if a.messageOutputIndex < 0 {
		return nil
	}
	return []string{
		a.event("response.output_text.done", map[string]any{"type": "response.output_text.done", "response_id": a.responseID, "item_id": a.messageID, "output_index": a.messageOutputIndex, "content_index": a.contentIndex, "text": text}),
		a.event("response.content_part.done", map[string]any{"type": "response.content_part.done", "response_id": a.responseID, "item_id": a.messageID, "output_index": a.messageOutputIndex, "content_index": a.contentIndex, "part": map[string]any{"type": "output_text", "text": text, "annotations": []any{}}}),
		a.event("response.output_item.done", map[string]any{"type": "response.output_item.done", "response_id": a.responseID, "output_index": a.messageOutputIndex, "item": buildResponsesOutputMessage(text, a.messageID, "completed")}),
	}
}

func (a *responsesStreamAdapter) ensureToolItem(toolIndex int, callID, name string) []string {
	if item, ok := a.toolItems[toolIndex]; ok {
		if name != "" && stringValue(item["name"]) == "" {
			item["name"] = name
		}
		return nil
	}
	outputIndex := a.allocOutputIndex()
	itemID := newResponsesFunctionCallID()
	item := map[string]any{"item_id": itemID, "output_index": outputIndex, "call_id": callID, "name": name, "arguments": ""}
	a.toolItems[toolIndex] = item
	toolItem := buildResponsesOutputToolCall(map[string]any{"id": callID, "function": map[string]any{"name": name, "arguments": ""}}, itemID, "in_progress")
	return []string{a.event("response.output_item.added", map[string]any{"type": "response.output_item.added", "response_id": a.responseID, "output_index": outputIndex, "item": toolItem})}
}

func (a *responsesStreamAdapter) toolArgumentsDeltaEvent(toolIndex int, delta string) string {
	if delta == "" {
		return ""
	}
	item := a.toolItems[toolIndex]
	if item == nil {
		return ""
	}
	item["arguments"] = stringValue(item["arguments"]) + delta
	return a.event("response.function_call_arguments.delta", map[string]any{"type": "response.function_call_arguments.delta", "response_id": a.responseID, "item_id": item["item_id"], "output_index": item["output_index"], "delta": delta})
}

func (a *responsesStreamAdapter) toolArgumentsDoneEvents() []string {
	events := make([]string, 0, len(a.toolItems)*2)
	indexes := make([]int, 0, len(a.toolItems))
	for index := range a.toolItems {
		indexes = append(indexes, index)
	}
	sort.Ints(indexes)
	for _, index := range indexes {
		item := a.toolItems[index]
		events = append(events, a.event("response.function_call_arguments.done", map[string]any{"type": "response.function_call_arguments.done", "response_id": a.responseID, "item_id": item["item_id"], "output_index": item["output_index"], "arguments": item["arguments"]}))
		toolItem := buildResponsesOutputToolCall(map[string]any{"id": item["call_id"], "function": map[string]any{"name": item["name"], "arguments": item["arguments"]}}, stringValue(item["item_id"]), "completed")
		events = append(events, a.event("response.output_item.done", map[string]any{"type": "response.output_item.done", "response_id": a.responseID, "output_index": item["output_index"], "item": toolItem}))
	}
	return events
}

func (a *responsesStreamAdapter) recordToolCall(toolIndex int, callID, name, argumentsDelta string) {
	toolCall, ok := a.toolCallsByIndex[toolIndex]
	if !ok {
		toolCall = map[string]any{"id": callID, "type": "function", "function": map[string]any{"name": name, "arguments": ""}}
		a.toolCallsByIndex[toolIndex] = toolCall
	}
	fn := mapValue(toolCall["function"])
	if name != "" && stringValue(fn["name"]) == "" {
		fn["name"] = name
	}
	if argumentsDelta != "" {
		fn["arguments"] = stringValue(fn["arguments"]) + argumentsDelta
	}
	toolCall["function"] = fn
}

func (a *responsesStreamAdapter) completedEvent(usage map[string]any) string {
	if usage == nil {
		usage = map[string]any{"total_tokens": 0, "input_tokens": 0, "output_tokens": 0}
	}
	outputText := ""
	if a.messageStarted {
		outputText = strings.Join(a.outputTextParts, "")
	}
	return a.event("response.completed", map[string]any{"type": "response.completed", "response": a.responsePayload("completed", outputText, usage)})
}

func toResponsesUsage(usage map[string]any) map[string]any {
	promptTokens := intValue(usage["prompt_tokens"])
	if promptTokens == 0 {
		promptTokens = intValue(usage["input_tokens"])
	}
	completionTokens := intValue(usage["completion_tokens"])
	if completionTokens == 0 {
		completionTokens = intValue(usage["output_tokens"])
	}
	chatUsage := BuildChatUsage(promptTokens, completionTokens)
	promptDetails := mapValue(chatUsage["prompt_tokens_details"])
	completionDetails := mapValue(chatUsage["completion_tokens_details"])
	return map[string]any{
		"input_tokens": promptTokens,
		"input_tokens_details": map[string]any{
			"cached_tokens": intValue(promptDetails["cached_tokens"]),
			"text_tokens":   maxInt(promptTokens, intValue(promptDetails["text_tokens"])),
			"image_tokens":  intValue(promptDetails["image_tokens"]),
		},
		"output_tokens": completionTokens,
		"output_tokens_details": map[string]any{
			"text_tokens":      completionTokens,
			"reasoning_tokens": intValue(completionDetails["reasoning_tokens"]),
		},
		"total_tokens": intValue(chatUsage["total_tokens"]),
	}
}

func responsesToToolCallSlice(value any) []map[string]any {
	items, ok := value.([]any)
	if !ok {
		if typed, ok := value.([]map[string]any); ok {
			return typed
		}
		return nil
	}
	result := make([]map[string]any, 0, len(items))
	for _, item := range items {
		if mapped := mapValue(item); mapped != nil {
			result = append(result, mapped)
		}
	}
	return result
}

func responsesMapAt(value any, index int) map[string]any {
	items := responsesToToolCallSlice(value)
	if index < 0 || index >= len(items) {
		return nil
	}
	return items[index]
}

func newResponsesResponseID() string     { return fmt.Sprintf("resp_%d", time.Now().UnixNano()) }
func newResponsesMessageID() string      { return fmt.Sprintf("msg_%d", time.Now().UnixNano()) }
func newResponsesToolCallID() string     { return fmt.Sprintf("call_%d", time.Now().UnixNano()) }
func newResponsesFunctionCallID() string { return fmt.Sprintf("fc_%d", time.Now().UnixNano()) }

func responsesEmptyToNil(value string) any {
	if strings.TrimSpace(value) == "" {
		return nil
	}
	return value
}
