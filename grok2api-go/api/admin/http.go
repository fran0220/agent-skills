package admin

import (
	"encoding/json"
	"net/http"
)

func writeJSON(w http.ResponseWriter, status int, payload any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(payload)
}

func methodNotAllowed(w http.ResponseWriter, allowed ...string) {
	if len(allowed) > 0 {
		w.Header().Set("Allow", joinAllowed(allowed))
	}
	writeJSON(w, http.StatusMethodNotAllowed, map[string]string{"error": "method not allowed"})
}

func badRequest(w http.ResponseWriter, message string) {
	writeJSON(w, http.StatusBadRequest, map[string]string{"error": message})
}

func internalServerError(w http.ResponseWriter, err error) {
	message := http.StatusText(http.StatusInternalServerError)
	if err != nil {
		message = err.Error()
	}
	writeJSON(w, http.StatusInternalServerError, map[string]string{"error": message})
}

func joinAllowed(values []string) string {
	if len(values) == 0 {
		return ""
	}
	joined := values[0]
	for i := 1; i < len(values); i++ {
		joined += ", " + values[i]
	}
	return joined
}
