package protocols

import (
	"io"
	"net/http"
)

// Listener represents a C2 listener configuration
type Listener struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	Protocol string `json:"protocol"`
	Host     string `json:"host"`
	Port     int    `json:"port"`
	Status   string `json:"status"`
}

// Protocol defines the interface that all communication protocols must implement
type Protocol interface {
	// Initialize sets up the protocol
	Initialize() error

	// HandleCommand handles sending commands to agents
	HandleCommand(cmd string) error

	// HandleFileUpload handles file uploads from agents
	HandleFileUpload(filename string, fileData io.Reader) error

	// HandleFileDownload handles file downloads to agents
	HandleFileDownload(filename string) (io.Reader, error)

	// HandleAgentHeartbeat processes agent heartbeats
	HandleAgentHeartbeat(agentData []byte) error

	// GetRoutes returns the HTTP routes this protocol needs
	GetRoutes() map[string]http.HandlerFunc
}

// BaseProtocolConfig contains common configuration for all protocols
type BaseProtocolConfig struct {
	UploadDir string
	Port      string
}
