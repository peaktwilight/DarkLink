package websocket

import (
	"encoding/json"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/gorilla/websocket"
)

// TerminalSession represents a user's terminal session on the server
// It maintains state about the current working directory and command context.
type TerminalSession struct {
	WorkingDir string
}

// TerminalResponse defines the structure of responses sent back to the client
// It provides command output, current working directory, and error status.
type TerminalResponse struct {
	Output string `json:"output,omitempty"`
	CWD    string `json:"cwd,omitempty"`
	Error  bool   `json:"error,omitempty"`
}

// TerminalHandler manages terminal websocket sessions
type TerminalHandler struct {
	upgrader websocket.Upgrader
}

// NewTerminalHandler creates a new terminal handler with configured websocket settings
//
// Pre-conditions:
//   - None
//
// Post-conditions:
//   - Returns a properly initialized TerminalHandler with CORS support
func NewTerminalHandler() *TerminalHandler {
	return &TerminalHandler{
		upgrader: websocket.Upgrader{
			CheckOrigin: func(r *http.Request) bool {
				return true
			},
		},
	}
}

// HandleConnection handles a new terminal websocket connection
//
// Pre-conditions:
//   - Valid HTTP request and response writer
//   - Client supports WebSocket protocol
//
// Post-conditions:
//   - WebSocket connection established with the client
//   - Terminal session started and commands processed until disconnection
//   - Resources properly cleaned up when the connection is closed
func (h *TerminalHandler) HandleConnection(w http.ResponseWriter, r *http.Request) {
	conn, err := h.upgrader.Upgrade(w, r, nil)
	if err != nil {
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
// It replaces the home directory with ~ for better readability
//
// Pre-conditions:
//   - path is a valid filesystem path
//
// Post-conditions:
//   - Returns the formatted path with home directory replaced by ~
func formatPath(path string) string {
	home := os.Getenv("HOME")
	if strings.HasPrefix(path, home) {
		return "~" + strings.TrimPrefix(path, home)
	}
	return path
}
