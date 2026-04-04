package api

import (
	"encoding/json"
	"net/http"

	"github.com/fran0220/grok2api-go/internal/config"
	"github.com/fran0220/grok2api-go/internal/model"
	"github.com/fran0220/grok2api-go/internal/service"
)

type ResponsesRequest struct {
	Model              string           `json:"model"`
	Input              any              `json:"input"`
	Instructions       string           `json:"instructions,omitempty"`
	Stream             bool             `json:"stream,omitempty"`
	MaxOutputTokens    *int             `json:"max_output_tokens,omitempty"`
	Temperature        *float64         `json:"temperature,omitempty"`
	TopP               *float64         `json:"top_p,omitempty"`
	Tools              []map[string]any `json:"tools,omitempty"`
	ToolChoice         any              `json:"tool_choice,omitempty"`
	ParallelToolCalls  *bool            `json:"parallel_tool_calls,omitempty"`
	Reasoning          map[string]any   `json:"reasoning,omitempty"`
	Metadata           map[string]any   `json:"metadata,omitempty"`
	User               string           `json:"user,omitempty"`
	Store              *bool            `json:"store,omitempty"`
	PreviousResponseID string           `json:"previous_response_id,omitempty"`
	Truncation         string           `json:"truncation,omitempty"`
}

func handleResponses(cfg *config.Config, modelService *model.Service) http.HandlerFunc {
	responsesService := service.NewResponsesService(cfg, modelService)
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			methodNotAllowed(w, http.MethodPost)
			return
		}
		defer r.Body.Close()
		var request ResponsesRequest
		decoder := json.NewDecoder(r.Body)
		decoder.UseNumber()
		if err := decoder.Decode(&request); err != nil {
			writeChatError(w, false, service.NewValidationError("Invalid JSON body", "invalid_json"))
			return
		}
		if request.Model == "" {
			writeChatError(w, false, service.NewValidationError("model is required", "invalid_request_error"))
			return
		}
		if request.Input == nil {
			writeChatError(w, false, service.NewValidationError("input is required", "invalid_request_error"))
			return
		}
		reasoningEffort := ""
		if request.Reasoning != nil {
			reasoningEffort = anyString(request.Reasoning["effort"])
			if reasoningEffort == "" {
				reasoningEffort = anyString(request.Reasoning["reasoning_effort"])
			}
		}
		result, err := responsesService.Create(r.Context(), service.ResponsesCreateParams{
			Model:              request.Model,
			Input:              request.Input,
			Instructions:       request.Instructions,
			Stream:             request.Stream,
			Temperature:        request.Temperature,
			TopP:               request.TopP,
			Tools:              request.Tools,
			ToolChoice:         request.ToolChoice,
			ParallelToolCalls:  request.ParallelToolCalls,
			ReasoningEffort:    reasoningEffort,
			MaxOutputTokens:    request.MaxOutputTokens,
			Metadata:           request.Metadata,
			User:               request.User,
			Store:              request.Store,
			PreviousResponseID: request.PreviousResponseID,
			Truncation:         request.Truncation,
		})
		if err != nil {
			writeChatError(w, request.Stream, err)
			return
		}
		if request.Stream {
			streamSSE(w, r, result.StreamData)
			return
		}
		writeJSON(w, http.StatusOK, result.Response)
	}
}
