package api

import (
	"encoding/json"
	"log"
	"microc2/server/pkg/communication"
	"net/http"
	"reflect"
	"strings"
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
	log.Printf("[DEBUG] HandleRequest called: %s %s", r.Method, r.URL.Path)
	if r.URL.Path == "/api/agents/list" {
		h.handleListAgents(w, r)
		return
	}

	// Handle POST /api/agents/{agentId}/command
	if r.Method == http.MethodPost && strings.HasPrefix(r.URL.Path, "/api/agents/") && strings.HasSuffix(r.URL.Path, "/command") {
		trimmed := strings.TrimPrefix(r.URL.Path, "/api/agents/")
		agentId := strings.TrimSuffix(trimmed, "/command")
		agentId = strings.TrimSuffix(agentId, "/") // Remove trailing slash if present
		h.handleQueueAgentCommand(w, r, agentId)
		return
	}

	// Add GET /api/agents/{agentId}/results endpoint
	if r.Method == http.MethodGet && strings.HasPrefix(r.URL.Path, "/api/agents/") && strings.HasSuffix(r.URL.Path, "/results") {
		trimmed := strings.TrimPrefix(r.URL.Path, "/api/agents/")
		agentId := strings.TrimSuffix(trimmed, "/results")
		agentId = strings.TrimSuffix(agentId, "/")
		h.handleGetAgentResults(w, r, agentId)
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

// handleQueueAgentCommand handles POST /api/agents/{agentId}/command
func (h *APIHandler) handleQueueAgentCommand(w http.ResponseWriter, r *http.Request, agentId string) {
	log.Printf("[DEBUG] handleQueueAgentCommand entered for agentId=%s", agentId)
	type cmdReq struct {
		Command string `json:"command"`
	}
	var req cmdReq
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil || req.Command == "" {
		log.Printf("[DEBUG] JSON decode failed or empty command: err=%v, req=%+v", err, req)
		http.Error(w, "Invalid command", http.StatusBadRequest)
		return
	}
	log.Printf("[DEBUG] handleQueueAgentCommand: agentId=%s, command=%s", agentId, req.Command)

	// Find the listener/protocol for this agent
	listenerMgr := h.serverManager.GetListenerManager()
	var queued bool
	for _, listener := range listenerMgr.ListListeners() {
		if listener.Protocol != nil {
			if agenter, ok := listener.Protocol.(interface{ GetAllAgents() map[string]interface{} }); ok {
				agents := agenter.GetAllAgents()
				log.Printf("[DEBUG] Listener %s has agents: %v", listener.Config.ID, reflect.ValueOf(agents).MapKeys())
				if _, exists := agents[agentId]; exists {
					if commander, ok := listener.Protocol.(interface {
						QueueCommand(agentId, cmd string)
					}); ok {
						commander.QueueCommand(agentId, req.Command)
						queued = true
						break
					}
				}
			}
		}
	}

	log.Printf("[DEBUG] Command queued for agent %s: %s (queued=%v)", agentId, req.Command, queued)

	if queued {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"status":"queued"}`))
	} else {
		http.Error(w, "Failed to queue command for agent", http.StatusInternalServerError)
	}
}

// Add handler for agent results
func (h *APIHandler) handleGetAgentResults(w http.ResponseWriter, r *http.Request, agentId string) {
	listenerMgr := h.serverManager.GetListenerManager()
	for _, listener := range listenerMgr.ListListeners() {
		if listener.Protocol != nil {
			if agenter, ok := listener.Protocol.(interface{ GetAllAgents() map[string]interface{} }); ok {
				agents := agenter.GetAllAgents()
				if _, exists := agents[agentId]; exists {
					if resultGetter, ok := listener.Protocol.(interface {
						GetResults(agentId string) []map[string]interface{}
					}); ok {
						results := resultGetter.GetResults(agentId)
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
