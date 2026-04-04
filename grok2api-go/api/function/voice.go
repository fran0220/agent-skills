package functionapi

import (
	"net/http"
	"strconv"

	"github.com/fran0220/grok2api-go/internal/reverse"
	"github.com/fran0220/grok2api-go/internal/service"
	"github.com/fran0220/grok2api-go/internal/token"
)

func (h *Handler) VoiceToken() http.Handler {
	voiceService := service.NewVoiceService(h.cfg)
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			methodNotAllowed(w, http.MethodGet)
			return
		}
		voice := r.URL.Query().Get("voice")
		personality := r.URL.Query().Get("personality")
		speed, _ := strconv.ParseFloat(r.URL.Query().Get("speed"), 64)
		if speed == 0 {
			speed = 1.0
		}
		tokenMgr := token.GetInstance()
		tokenMgr.ReloadIfStale()
		ssoToken := tokenMgr.GetToken(token.BasicPoolName, nil, nil)
		if ssoToken == "" {
			ssoToken = tokenMgr.GetToken(token.SuperPoolName, nil, nil)
		}
		if ssoToken == "" {
			writeJSON(w, http.StatusServiceUnavailable, map[string]string{"error": "No available tokens for voice mode"})
			return
		}
		data, err := voiceService.GetToken(r.Context(), ssoToken, voice, personality, speed)
		if err != nil {
			writeJSON(w, http.StatusBadGateway, map[string]string{"error": err.Error()})
			return
		}
		livekitToken, _ := data["token"].(string)
		if livekitToken == "" {
			writeJSON(w, http.StatusBadGateway, map[string]string{"error": "Upstream returned no voice token"})
			return
		}
		writeJSON(w, http.StatusOK, map[string]any{"token": livekitToken, "url": reverse.LivekitWSURL, "participant_name": "", "room_name": ""})
	})
}
