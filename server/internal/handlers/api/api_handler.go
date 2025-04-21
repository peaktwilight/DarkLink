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
	// Default handler for API requests
	w.Header().Set("Content-Type", "application/json")
	w.Write([]byte(`{"status":"ok"}`))
}
