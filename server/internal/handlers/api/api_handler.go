package api

import (
	"encoding/json"
	"microc2/server/pkg/communication"
	"net/http"
)

type APIHandler struct {
	serverManager *communication.ServerManager
}

func NewAPIHandler(manager *communication.ServerManager) *APIHandler {
	return &APIHandler{
		serverManager: manager,
	}
}

func (h *APIHandler) HandleRequest(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path == "/api/agents/list" {
		h.handleListAgents(w, r)
		return
	}

	// Default handler for API requests
	w.Header().Set("Content-Type", "application/json")
	w.Write([]byte(`{"status":"ok"}`))
}

func (h *APIHandler) handleListAgents(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Aggregate agents from all listeners
	agents := h.serverManager.GetListenerManager().AllAgents()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(agents)
}
