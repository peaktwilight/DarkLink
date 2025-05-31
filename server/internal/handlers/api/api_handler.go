package api

import (
	"encoding/json"
	"microc2/server/pkg/communication"
	"net/http"
	"strings"
)

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

	// Handle POST /api/agents/{AgentID}/command
	if r.Method == http.MethodPost && strings.HasPrefix(r.URL.Path, "/api/agents/") && strings.HasSuffix(r.URL.Path, "/command") {
		trimmed := strings.TrimPrefix(r.URL.Path, "/api/agents/")
		AgentID := strings.TrimSuffix(trimmed, "/command")
		AgentID = strings.TrimSuffix(AgentID, "/") // Remove trailing slash if present
		h.handleQueueAgentCommand(w, r, AgentID)
		return
	}

	// Add GET /api/agents/{AgentID}/results endpoint
	if r.Method == http.MethodGet && strings.HasPrefix(r.URL.Path, "/api/agents/") && strings.HasSuffix(r.URL.Path, "/results") {
		trimmed := strings.TrimPrefix(r.URL.Path, "/api/agents/")
		AgentID := strings.TrimSuffix(trimmed, "/results")
		AgentID = strings.TrimSuffix(AgentID, "/")
		h.handleGetAgentResults(w, AgentID)
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

// handleQueueAgentCommand handles POST /api/agents/{AgentID}/command
func (h *APIHandler) handleQueueAgentCommand(w http.ResponseWriter, r *http.Request, AgentID string) {
	type cmdReq struct {
		Command string `json:"command"`
	}
	var req cmdReq
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil || req.Command == "" {
		http.Error(w, "Invalid command", http.StatusBadRequest)
		return
	}

	// Find the listener/protocol for this agent
	listenerMgr := h.serverManager.GetListenerManager()
	var queued bool
	for _, listener := range listenerMgr.ListListeners() {
		if listener.Protocol != nil {
			if agenter, ok := listener.Protocol.(interface{ GetAllAgents() map[string]interface{} }); ok {
				agents := agenter.GetAllAgents()
				if _, exists := agents[AgentID]; exists {
					if commander, ok := listener.Protocol.(interface {
						QueueCommand(AgentID, cmd string)
					}); ok {
						commander.QueueCommand(AgentID, req.Command)
						queued = true
						break
					}
				}
			}
		}
	}


	if queued {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"status":"queued"}`))
	} else {
		http.Error(w, "Failed to queue command for agent", http.StatusInternalServerError)
	}
}

// Add handler for agent results
func (h *APIHandler) handleGetAgentResults(w http.ResponseWriter, AgentID string) {
	listenerMgr := h.serverManager.GetListenerManager()
	for _, listener := range listenerMgr.ListListeners() {
		if listener.Protocol != nil {
			if agenter, ok := listener.Protocol.(interface{ GetAllAgents() map[string]interface{} }); ok {
				agents := agenter.GetAllAgents()
				if _, exists := agents[AgentID]; exists {
					if resultGetter, ok := listener.Protocol.(interface {
						GetResults(AgentID string) []map[string]interface{}
					}); ok {
						results := resultGetter.GetResults(AgentID)
							w.Header().Set("Content-Type", "application/json")
						json.NewEncoder(w).Encode(results)
						return
					}
				}
			}
		}
	}
	http.Error(w, "Agent or results not found", http.StatusNotFound)
}
