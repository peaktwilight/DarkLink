package api

import (
	"encoding/json"
	"microc2/server/internal/networking"
	"net/http"
)

// NewSOCKS5Handler creates a new SOCKS5 management handler
func NewSOCKS5Handler(protocol *networking.SOCKS5Protocol) *SOCKS5Handler {
	return &SOCKS5Handler{
		protocol: protocol,
	}
}

// RegisterRoutes registers the SOCKS5 management API routes
func (h *SOCKS5Handler) RegisterRoutes() map[string]http.HandlerFunc {
	return map[string]http.HandlerFunc{
		"/api/socks5/tunnels":       h.handleListTunnels,
		"/api/socks5/tunnels/get":   h.handleGetTunnel,
		"/api/socks5/tunnels/close": h.handleCloseTunnel,
		"/api/socks5/config":        h.handleGetConfig,
		"/api/socks5/config/update": h.handleUpdateConfig,
	}
}

// handleListTunnels returns a list of all active SOCKS5 tunnels
func (h *SOCKS5Handler) handleListTunnels(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	h.protocol.ListTunnels(w, r)
}

// handleGetTunnel returns details about a specific tunnel
func (h *SOCKS5Handler) handleGetTunnel(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	h.protocol.GetTunnel(w, r)
}

// handleCloseTunnel closes a specific tunnel
func (h *SOCKS5Handler) handleCloseTunnel(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	h.protocol.CloseTunnel(w, r)
}

// handleGetConfig returns the current SOCKS5 configuration
func (h *SOCKS5Handler) handleGetConfig(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(h.protocol.GetServer().GetConfig())
}

// handleUpdateConfig updates the SOCKS5 configuration
func (h *SOCKS5Handler) handleUpdateConfig(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var config networking.SOCKS5Config
	if err := json.NewDecoder(r.Body).Decode(&config); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	h.protocol.GetServer().SetConfig(config)
	w.WriteHeader(http.StatusOK)
}
