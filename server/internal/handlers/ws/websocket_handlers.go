package ws

import (
	"net/http"

	wsutil "microc2/server/internal/websocket"

	"github.com/gorilla/websocket"
)

// Handler manages WebSocket endpoints
type Handler struct {
	logStreamer     *wsutil.LogStreamer
	terminalHandler *wsutil.TerminalHandler
}

// New creates a new WebSocket handlers instance
func New(logStreamer *wsutil.LogStreamer) *Handler {
	return &Handler{
		logStreamer:     logStreamer,
		terminalHandler: wsutil.NewTerminalHandler(),
	}
}

// HandleLogStream handles WebSocket connections for log streaming
func (h *Handler) HandleLogStream(w http.ResponseWriter, r *http.Request) {
	conn, err := h.logStreamer.GetUpgrader().Upgrade(w, r, nil)
	if err != nil {
		http.Error(w, "WebSocket upgrade failed", http.StatusInternalServerError)
		return
	}

	h.logStreamer.AddClient(conn)

	// Keep connection alive and handle disconnection
	for {
		messageType, p, err := conn.ReadMessage()
		if err != nil {
			h.logStreamer.RemoveClient(conn)
			conn.Close()
			break
		}

		// Handle ping messages from client
		if messageType == websocket.TextMessage && string(p) == "ping" {
			if err := conn.WriteMessage(websocket.TextMessage, []byte("pong")); err != nil {
				h.logStreamer.RemoveClient(conn)
				conn.Close()
				break
			}
		}
	}
}

// HandleTerminal handles WebSocket connections for terminal sessions
func (h *Handler) HandleTerminal(w http.ResponseWriter, r *http.Request) {
	h.terminalHandler.HandleConnection(w, r)
}
