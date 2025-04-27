package communication

import (
	"fmt"
	"log"
	"net/http"
	"os"
	"strings"

	"microc2/server/internal/networking"
)

type ServerManager struct {
	protocol        networking.Protocol
	config          *ServerConfig
	listenerManager *networking.ListenerManager
}

type ServerConfig struct {
	UploadDir    string
	Port         string
	StaticDir    string
	ProtocolType string
}

func NewServerManager(config *ServerConfig) (*ServerManager, error) {
	if err := os.MkdirAll(config.UploadDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create upload directory: %v", err)
	}

	baseConfig := networking.BaseProtocolConfig{
		UploadDir: config.UploadDir,
		Port:      config.Port,
	}

	var protocol networking.Protocol
	switch config.ProtocolType {
	case "http-polling":
		protocol = networking.NewHTTPPollingProtocol(baseConfig)
	case "socks5":
		protocol = networking.NewSOCKS5Protocol(baseConfig)
	default:
		return nil, fmt.Errorf("unsupported protocol type: %s", config.ProtocolType)
	}

	if err := protocol.Initialize(); err != nil {
		return nil, fmt.Errorf("failed to initialize protocol: %v", err)
	}

	// Initialize ListenerManager with the protocol instance
	listenerManager := networking.NewListenerManager(protocol)

	return &ServerManager{
		protocol:        protocol,
		config:          config,
		listenerManager: listenerManager,
	}, nil
}

// GetProtocol returns the current protocol instance
func (sm *ServerManager) GetProtocol() networking.Protocol {
	return sm.protocol
}

func (sm *ServerManager) Start() error {
	// Register protocol-specific routes
	for path, handler := range sm.protocol.GetRoutes() {
		// Skip routes that might conflict with API handlers
		if strings.HasPrefix(path, "/api/") {
			log.Printf("[ROUTES] Skipping protocol route %s to avoid conflicts with API handlers", path)
			continue
		}
		http.HandleFunc(path, handler)
	}

	log.Printf("[STARTUP] Server initializing with %s protocol...", sm.config.ProtocolType)
	log.Printf("[CONFIG] Upload directory: %s", sm.config.UploadDir)
	log.Printf("[CONFIG] Static directory: %s", sm.config.StaticDir)
	log.Printf("[CONFIG] File Drop directory: %s/file_drop", sm.config.StaticDir)
	log.Printf("[NETWORK] Port: %s", sm.config.Port)

	return http.ListenAndServe(":"+sm.config.Port, nil)
}

func (sm *ServerManager) GetListenerManager() *networking.ListenerManager {
	return sm.listenerManager
}
