package websocket

import (
	"net/http"
	"os"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

type LogMessage struct {
	Timestamp string `json:"timestamp"`
	Level     string `json:"level"`
	Message   string `json:"message"`
}

// LogStreamer handles broadcasting logs to WebSocket clients
type LogStreamer struct {
	clients    map[*websocket.Conn]bool
	clientsMux sync.Mutex
	upgrader   websocket.Upgrader
	logFile    *os.File
}

// NewLogStreamer creates a new log streamer instance
func NewLogStreamer(logFile *os.File) *LogStreamer {
	return &LogStreamer{
		clients: make(map[*websocket.Conn]bool),
		upgrader: websocket.Upgrader{
			CheckOrigin: func(r *http.Request) bool {
				return true
			},
			HandshakeTimeout:  10 * time.Second,
			ReadBufferSize:    1024,
			WriteBufferSize:   1024,
			EnableCompression: true,
		},
		logFile: logFile,
	}
}

// setupPingPong configures ping/pong handlers for the connection
func (s *LogStreamer) setupPingPong(conn *websocket.Conn) {
	pongWait := 60 * time.Second
	pingPeriod := (pongWait * 9) / 10

	conn.SetReadDeadline(time.Now().Add(pongWait))
	conn.SetPongHandler(func(string) error {
		conn.SetReadDeadline(time.Now().Add(pongWait))
		return nil
	})

	// Start ping ticker
	ticker := time.NewTicker(pingPeriod)
	go func() {
		defer ticker.Stop()
		for {
			select {
			case <-ticker.C:
				if err := conn.WriteControl(websocket.PingMessage, []byte{}, time.Now().Add(time.Second)); err != nil {
					return
				}
			}
		}
	}()
}

// Write implements io.Writer for integration with log package
func (s *LogStreamer) Write(p []byte) (n int, err error) {
	// Write to original output
	if s.logFile != nil {
		s.logFile.Write(p)
	}

	// Prepare log message
	logMsg := &LogMessage{
		Timestamp: time.Now().Format(time.RFC3339),
		Level:     "info",
		Message:   string(p),
	}

	if len(p) > 0 && (p[0] == '[' || p[0] == '*') {
		// Parse log level from prefix like [ERROR] or [INFO]
		for i := 1; i < len(p); i++ {
			if p[i] == ']' {
				logMsg.Level = string(p[1:i])
				logMsg.Message = string(p[i+1:])
				break
			}
		}
	}

	// Broadcast to all connected WebSocket clients
	clientChanges := s.broadcastLog(logMsg)

	// Handle any client changes (connections/disconnections)
	if len(clientChanges) > 0 {
		s.clientsMux.Lock()
		for client, connected := range clientChanges {
			if !connected {
				delete(s.clients, client)
			}
		}
		s.clientsMux.Unlock()
	}

	return len(p), nil
}

func (s *LogStreamer) broadcastLog(msg *LogMessage) map[*websocket.Conn]bool {
	clientChanges := make(map[*websocket.Conn]bool)
	s.clientsMux.Lock()
	defer s.clientsMux.Unlock()

	for client := range s.clients {
		client.SetWriteDeadline(time.Now().Add(10 * time.Second))
		if err := client.WriteJSON(msg); err != nil {
			clientChanges[client] = false
			client.Close()
		}
	}

	return clientChanges
}

// AddClient adds a new WebSocket client to receive log messages
func (s *LogStreamer) AddClient(conn *websocket.Conn) {
	s.setupPingPong(conn)

	s.clientsMux.Lock()
	s.clients[conn] = true
	s.clientsMux.Unlock()

	// Send initial connection message
	initialMsg := &LogMessage{
		Timestamp: time.Now().Format(time.RFC3339),
		Level:     "info",
		Message:   "Connected to log stream",
	}
	conn.WriteJSON(initialMsg)
}

// RemoveClient removes a WebSocket client
func (s *LogStreamer) RemoveClient(conn *websocket.Conn) {
	s.clientsMux.Lock()
	delete(s.clients, conn)
	s.clientsMux.Unlock()
}

// GetUpgrader returns the WebSocket upgrader
func (s *LogStreamer) GetUpgrader() *websocket.Upgrader {
	return &s.upgrader
}
