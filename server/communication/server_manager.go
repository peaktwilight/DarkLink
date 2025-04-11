package communication

import (
	"fmt"
	"log"
	"microc2/server/communication/protocols"
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

func (sm *ServerManager) handleRoot(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path != "/" {
		http.NotFound(w, r)
		return
	}
	http.ServeFile(w, r, sm.config.StaticDir+"/index.html")
}

func (sm *ServerManager) Start() error {
	// Add root handler for index.html
	http.HandleFunc("/", sm.handleRoot)

	// Set up static file serving
	fs := http.FileServer(http.Dir(sm.config.StaticDir))
	http.Handle("/static/", http.StripPrefix("/static/", fs))

	// Set up upload directory serving
	http.Handle("/download/", http.StripPrefix("/download/",
		http.FileServer(http.Dir(sm.config.UploadDir))))

	// Register protocol-specific routes
	for path, handler := range sm.protocol.GetRoutes() {
		http.HandleFunc(path, handler)
	}

	// Simple logging
	log.Printf("[STARTUP] Server initializing with %s protocol...", sm.config.ProtocolType)
	log.Printf("[CONFIG] Upload directory: %s", sm.config.UploadDir)
	log.Printf("[CONFIG] Static directory: %s", sm.config.StaticDir)
	log.Printf("[NETWORK] Port: %s", sm.config.Port)

	return http.ListenAndServe(":"+sm.config.Port, nil)
}
