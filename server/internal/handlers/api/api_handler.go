package api

import (
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

	// Get the protocol instance which maintains the agent list
	protocol := h.serverManager.GetProtocol()

	// Access the protocol's agent list through its routes
	if agentListHandler := protocol.GetRoutes()["/agent/list"]; agentListHandler != nil {
		agentListHandler(w, r)
		return
	}

	// If no handler found, return empty list
	w.Header().Set("Content-Type", "application/json")
	w.Write([]byte("[]"))
}
