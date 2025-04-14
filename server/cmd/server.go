package main

import (
	"encoding/json"
	"flag"
	"log"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"microc2/server/communication"
	"microc2/server/config"

	"github.com/gorilla/websocket"
)

var (
	logClients    = make(map[*websocket.Conn]bool)
	logClientsMux sync.Mutex
	upgrader      = websocket.Upgrader{
		CheckOrigin: func(r *http.Request) bool {
			return true
		},
	}
)

type LogMessage struct {
	Timestamp string `json:"timestamp"`
	Level     string `json:"level"`
	Message   string `json:"message"`
}

// Custom log writer that broadcasts to WebSocket clients
type WebSocketLogWriter struct {
	originalOutput *os.File
}

func (w *WebSocketLogWriter) Write(p []byte) (n int, err error) {
	// Write to original output
	if w.originalOutput != nil {
		w.originalOutput.Write(p)
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
	logClientsChanges := broadcastLog(logMsg)

	// Handle any client changes (connections/disconnections)
	if len(logClientsChanges) > 0 {
		logClientsMux.Lock()
		for client, connected := range logClientsChanges {
			if !connected {
				delete(logClients, client)
			}
		}
		logClientsMux.Unlock()
	}

	return len(p), nil
}

func broadcastLog(msg *LogMessage) map[*websocket.Conn]bool {
	clientChanges := make(map[*websocket.Conn]bool)
	logClientsMux.Lock()
	defer logClientsMux.Unlock()

	for client := range logClients {
		err := client.WriteJSON(msg)
		if err != nil {
			clientChanges[client] = false
			client.Close()
		}
	}

	return clientChanges
}

func handleLogWebSocket(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("[ERROR] WebSocket upgrade failed: %v", err)
		return
	}

	logClientsMux.Lock()
	logClients[conn] = true
	logClientsMux.Unlock()

	// Send initial connection message
	initialMsg := &LogMessage{
		Timestamp: time.Now().Format(time.RFC3339),
		Level:     "info",
		Message:   "Connected to log stream",
	}
	conn.WriteJSON(initialMsg)

	// Keep connection alive and handle disconnection
	for {
		_, _, err := conn.ReadMessage()
		if err != nil {
			logClientsMux.Lock()
			delete(logClients, conn)
			logClientsMux.Unlock()
			conn.Close()
			break
		}
	}
}

type TerminalSession struct {
	WorkingDir string
}

type TerminalResponse struct {
	Output string `json:"output,omitempty"`
	CWD    string `json:"cwd,omitempty"`
	Error  bool   `json:"error,omitempty"`
}

func handleTerminalWebSocket(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Printf("[ERROR] Terminal WebSocket upgrade failed: %v", err)
		return
	}
	defer conn.Close()

	session := &TerminalSession{
		WorkingDir: os.Getenv("HOME"),
	}

	// Send initial connection message with working directory
	initialResponse := TerminalResponse{
		Output: "Connected to server terminal (Bash shell).\n",
		CWD:    formatPath(session.WorkingDir),
	}
	msg, _ := json.Marshal(initialResponse)
	conn.WriteMessage(websocket.TextMessage, msg)

	for {
		// Read message from the WebSocket
		_, message, err := conn.ReadMessage()
		if err != nil {
			log.Printf("[ERROR] Failed to read from terminal WebSocket: %v", err)
			break
		}

		command := string(message)

		// Handle built-in commands
		if command == "pwd" {
			response := TerminalResponse{
				Output: session.WorkingDir + "\n",
				CWD:    formatPath(session.WorkingDir),
			}
			msg, _ := json.Marshal(response)
			conn.WriteMessage(websocket.TextMessage, msg)
			continue
		}

		if strings.HasPrefix(command, "cd ") {
			dir := strings.TrimSpace(strings.TrimPrefix(command, "cd "))
			if dir == "~" {
				dir = os.Getenv("HOME")
			} else if strings.HasPrefix(dir, "~/") {
				dir = filepath.Join(os.Getenv("HOME"), dir[2:])
			} else if !filepath.IsAbs(dir) {
				dir = filepath.Join(session.WorkingDir, dir)
			}

			if _, err := os.Stat(dir); err == nil {
				session.WorkingDir = dir
				response := TerminalResponse{
					CWD: formatPath(session.WorkingDir),
				}
				msg, _ := json.Marshal(response)
				conn.WriteMessage(websocket.TextMessage, msg)
			} else {
				response := TerminalResponse{
					Output: "cd: " + dir + ": No such file or directory\n",
					Error:  true,
					CWD:    formatPath(session.WorkingDir),
				}
				msg, _ := json.Marshal(response)
				conn.WriteMessage(websocket.TextMessage, msg)
			}
			continue
		}

		// Execute command
		cmd := exec.Command("/bin/bash", "-c", command)
		cmd.Dir = session.WorkingDir
		cmd.Env = append(os.Environ(), "TERM=xterm-256color")

		output, err := cmd.CombinedOutput()

		response := TerminalResponse{
			Output: string(output),
			CWD:    formatPath(session.WorkingDir),
			Error:  err != nil,
		}

		msg, _ := json.Marshal(response)
		conn.WriteMessage(websocket.TextMessage, msg)
	}
}

// formatPath formats the path for display in the terminal prompt
func formatPath(path string) string {
	home := os.Getenv("HOME")
	if strings.HasPrefix(path, home) {
		return "~" + strings.TrimPrefix(path, home)
	}
	return path
}

func main() {
	// Replace standard logger output with our custom writer
	logFile, err := os.OpenFile("server.log", os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		log.Fatal(err)
	}
	defer logFile.Close()

	wsLogWriter := &WebSocketLogWriter{originalOutput: logFile}
	log.SetOutput(wsLogWriter)

	// Parse command line flags
	configPath := flag.String("config", "config/settings.yaml", "Path to configuration file")
	flag.Parse()

	// Load configuration
	cfg, err := config.LoadConfig(*configPath)
	if err != nil {
		log.Fatalf("Failed to load configuration: %v", err)
	}

	// Set up server configuration
	serverConfig := &communication.ServerConfig{
		UploadDir:    cfg.Server.UploadDir,
		Port:         cfg.Server.Port,
		StaticDir:    cfg.Server.StaticDir,
		ProtocolType: cfg.Communication.Protocol,
	}

	// Create and start server manager
	serverManager, err := communication.NewServerManager(serverConfig)
	if err != nil {
		log.Fatalf("Failed to create server manager: %v", err)
	}

	// Add WebSocket handlers
	http.HandleFunc("/logs", handleLogWebSocket)
	http.HandleFunc("/ws/terminal", handleTerminalWebSocket)

	log.Printf("[STARTUP] Starting server with %s protocol...", cfg.Communication.Protocol)
	if err := serverManager.Start(); err != nil {
		log.Fatalf("[ERROR] Server error: %v", err)
	}
}
