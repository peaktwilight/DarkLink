package communication

import (
	"fmt"
	"log"
	"microc2/server/internal/protocols"
	"net/http"
	"os"
)

type ServerManager struct {
	protocol protocols.Protocol
	config   *ServerConfig
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

	baseConfig := protocols.BaseProtocolConfig{
		UploadDir: config.UploadDir,
		Port:      config.Port,
	}

	var protocol protocols.Protocol
	switch config.ProtocolType {
	case "http-polling":
		protocol = protocols.NewHTTPPollingProtocol(baseConfig)
	case "dns-over-https":
		protocol = protocols.NewDNSOverHTTPSProtocol(baseConfig)
	default:
		return nil, fmt.Errorf("unsupported protocol type: %s", config.ProtocolType)
	}

	if err := protocol.Initialize(); err != nil {
		return nil, fmt.Errorf("failed to initialize protocol: %v", err)
	}

	return &ServerManager{
		protocol: protocol,
		config:   config,
	}, nil
}

func (sm *ServerManager) Start() error {
	// Register protocol-specific routes
	for path, handler := range sm.protocol.GetRoutes() {
		http.HandleFunc(path, handler)
	}

	log.Printf("[STARTUP] Server initializing with %s protocol...", sm.config.ProtocolType)
	log.Printf("[CONFIG] Upload directory: %s", sm.config.UploadDir)
	log.Printf("[CONFIG] Static directory: %s", sm.config.StaticDir)
	log.Printf("[CONFIG] File Drop directory: %s/file_drop", sm.config.StaticDir)
	log.Printf("[NETWORK] Port: %s", sm.config.Port)

	return http.ListenAndServe(":"+sm.config.Port, nil)
}
