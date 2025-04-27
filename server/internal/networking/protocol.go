package networking

import (
	"io"
	"net/http"
)

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

	// GetHTTPHandler returns the HTTP handler for the protocol (if applicable)
	GetHTTPHandler() http.Handler
}

// BaseProtocolConfig contains common configuration for all protocols
type BaseProtocolConfig struct {
	UploadDir string
	Port      string
}
