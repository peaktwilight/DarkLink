package communication

import (
	"fmt"
	"log"
	"net/http"
	"os"
	"strings"

	"darklink/server/internal/behaviour"
	"darklink/server/internal/common"
	"darklink/server/internal/listeners"
	"darklink/server/internal/protocols"
)

type ServerManager struct {
	protocol        listeners.Protocol
	config          *ServerConfig
	listenerManager *listeners.ListenerManager
}

type ServerConfig struct {
	UploadDir    string
	Port         int
	StaticDir    string
	ProtocolType string
}

func NewServerManager(config *ServerConfig) (*ServerManager, error) {
	if err := os.MkdirAll(config.UploadDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create upload directory: %v", err)
	}

	baseConfig := common.BaseProtocolConfig{
		UploadDir: config.UploadDir,
		Port:      fmt.Sprintf("%d", config.Port),
	}

	var protocol listeners.Protocol
	switch config.ProtocolType {
	case "http":
		protocol = behaviour.NewHTTPPollingProtocol(baseConfig)
	case "socks5":
		protocol = protocols.NewSOCKS5Protocol(baseConfig)
	default:
		return nil, fmt.Errorf("unsupported protocol type: %s", config.ProtocolType)
	}

	if err := protocol.Initialize(); err != nil {
		return nil, fmt.Errorf("failed to initialize protocol: %v", err)
	}

	listenerManager := listeners.NewListenerManager(protocol)

	return &ServerManager{
		protocol:        protocol,
		config:          config,
		listenerManager: listenerManager,
	}, nil
}

// GetProtocol returns the current protocol instance
func (sm *ServerManager) GetProtocol() listeners.Protocol {
	return sm.protocol
}

func (sm *ServerManager) Start() error {
	// Register protocol-specific routes
	for path, handler := range sm.protocol.GetRoutes() {
		// Skip routes that might conflict with API handlers
		if strings.HasPrefix(path, "/api/agent/") {
			log.Printf("[ROUTES] Skipping protocol route %s to avoid conflicts with API handlers", path)
			continue
		}
		http.HandleFunc(path, handler)
	}

	if httpProto, ok := sm.protocol.(*behaviour.HTTPPollingProtocol); ok {
		http.HandleFunc("/api/agent/", httpProto.HandleAgentRequests)
	}

	log.Printf("[STARTUP] Server initializing with %s protocol...", sm.config.ProtocolType)
	log.Printf("[CONFIG] Upload directory: %s", sm.config.UploadDir)
	log.Printf("[CONFIG] Static directory: %s", sm.config.StaticDir)
	log.Printf("[CONFIG] File Drop directory: %s/file_drop", sm.config.StaticDir)
	log.Printf("[NETWORK] Port: %d", sm.config.Port)

	return http.ListenAndServe(fmt.Sprintf(":%d", sm.config.Port), nil)
}

func (sm *ServerManager) GetListenerManager() *listeners.ListenerManager {
	return sm.listenerManager
}
