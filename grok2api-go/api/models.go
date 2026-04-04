package api

import (
	"encoding/json"
	"net/http"

	"github.com/fran0220/grok2api-go/internal/model"
)

type modelResponse struct {
	ID      string `json:"id"`
	Object  string `json:"object"`
	Created int64  `json:"created"`
	OwnedBy string `json:"owned_by"`
}

type listModelsResponse struct {
	Object string          `json:"object"`
	Data   []modelResponse `json:"data"`
}

func handleModels(service *model.Service) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w, http.MethodGet)
			return
		}

		models := service.List()
		data := make([]modelResponse, 0, len(models))
		for _, item := range models {
			data = append(data, modelResponse{
				ID:      item.ModelID,
				Object:  "model",
				Created: 0,
				OwnedBy: "grok2api@chenyme",
			})
		}

		writeJSON(w, http.StatusOK, listModelsResponse{Object: "list", Data: data})
	}
}

func writeJSON(w http.ResponseWriter, status int, payload any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(payload)
}

func methodNotAllowed(w http.ResponseWriter, allowed string) {
	w.Header().Set("Allow", allowed)
	writeJSON(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
}
