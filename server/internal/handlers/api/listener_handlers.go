package api

import (
	"encoding/json"
	"microc2/server/internal/protocols"
	"net/http"
	"strings"
)

// ListenerHandlers manages HTTP handlers for listener operations
type ListenerHandlers struct {
	manager *protocols.ListenerManager
}

// NewListenerHandlers creates a new listener handlers instance
func NewListenerHandlers(manager *protocols.ListenerManager) *ListenerHandlers {
	return &ListenerHandlers{
		manager: manager,
	}
}

// HandleCreateListener handles requests to create a new listener
func (h *ListenerHandlers) HandleCreateListener(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var config protocols.ListenerConfig
	if err := json.NewDecoder(r.Body).Decode(&config); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	listener, err := h.manager.CreateListener(config)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(listener)
}

// HandleListListeners handles requests to list all listeners
func (h *ListenerHandlers) HandleListListeners(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	listeners := h.manager.ListListeners()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(listeners)
}

// HandleGetListener handles requests to get a specific listener
func (h *ListenerHandlers) HandleGetListener(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	id := strings.TrimPrefix(r.URL.Path, "/api/listeners/")
	listener, err := h.manager.GetListener(id)
	if err != nil {
		http.Error(w, err.Error(), http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(listener)
}

// HandleStopListener handles requests to stop a listener
func (h *ListenerHandlers) HandleStopListener(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	id := strings.TrimPrefix(r.URL.Path, "/api/listeners/")
	id = strings.TrimSuffix(id, "/stop")

	if err := h.manager.StopListener(id); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusOK)
}

// HandleDeleteListener handles requests to delete a listener
func (h *ListenerHandlers) HandleDeleteListener(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodDelete {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	id := strings.TrimPrefix(r.URL.Path, "/api/listeners/")
	if err := h.manager.StopListener(id); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusOK)
}

// SetupRoutes registers all listener-related routes
func (h *ListenerHandlers) SetupRoutes() {
	http.HandleFunc("/api/listeners/create", h.HandleCreateListener)
	http.HandleFunc("/api/listeners/list", h.HandleListListeners)
	http.HandleFunc("/api/listeners/", func(w http.ResponseWriter, r *http.Request) {
		path := strings.TrimPrefix(r.URL.Path, "/api/listeners/")
		if strings.HasSuffix(path, "/stop") {
			h.HandleStopListener(w, r)
			return
		}
		switch r.Method {
		case http.MethodGet:
			h.HandleGetListener(w, r)
		case http.MethodDelete:
			h.HandleDeleteListener(w, r)
		default:
			http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		}
	})
}
