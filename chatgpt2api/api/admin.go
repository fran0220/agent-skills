package api

import (
	"encoding/json"
	"net/http"

	"chatgpt2api/internal/token"
)

func handleTokens() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		tokenMgr := token.GetInstance()

		switch r.Method {
		case http.MethodGet:
			tokens := tokenMgr.ListTokens()
			items := make([]map[string]any, 0, len(tokens))
			for _, t := range tokens {
				items = append(items, map[string]any{
					"token":      maskToken(t.Token),
					"note":       t.Note,
					"status":     string(t.Status),
					"use_count":  t.UseCount,
					"fail_count": t.FailCount,
				})
			}
			stats := tokenMgr.GetStats()
			writeJSON(w, http.StatusOK, map[string]any{
				"tokens": map[string]any{
					"default": items,
				},
				"stats": stats,
			})

		case http.MethodPost:
			var body struct {
				Token string `json:"token"`
				Note  string `json:"note"`
			}
			if err := json.NewDecoder(r.Body).Decode(&body); err != nil || body.Token == "" {
				writeError(w, http.StatusBadRequest, "token is required")
				return
			}
			if err := tokenMgr.AddToken(body.Token, body.Note); err != nil {
				writeError(w, http.StatusConflict, err.Error())
				return
			}
			tokenMgr.Save()
			writeJSON(w, http.StatusCreated, map[string]string{"status": "added"})

		case http.MethodDelete:
			var body struct {
				Token string `json:"token"`
			}
			if err := json.NewDecoder(r.Body).Decode(&body); err != nil || body.Token == "" {
				writeError(w, http.StatusBadRequest, "token is required")
				return
			}
			if !tokenMgr.RemoveToken(body.Token) {
				writeError(w, http.StatusNotFound, "token not found")
				return
			}
			tokenMgr.Save()
			writeJSON(w, http.StatusOK, map[string]string{"status": "removed"})

		default:
			writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		}
	}
}

func maskToken(t string) string {
	if len(t) <= 8 {
		return t
	}
	return "***" + t[len(t)-8:]
}
