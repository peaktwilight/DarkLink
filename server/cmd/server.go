package main

import (
	"flag"
	"log"
	"net/http"
	"os"
	"path/filepath"

	"microc2/server/config"
	"microc2/server/internal/filestore"
	"microc2/server/internal/handlers/api"
	"microc2/server/internal/handlers/web"
	"microc2/server/internal/handlers/ws"
	"microc2/server/internal/websocket"
	"microc2/server/pkg/communication"
)

// main is the entry point of the MicroC2 server application
//
// Pre-conditions:
//   - Configuration file exists at the specified path or default location
//   - Required directories are accessible with proper permissions
//
// Post-conditions:
//   - Server is initialized with configured handlers and services
//   - Server starts listening on the configured port
//   - Log files are properly set up and streamed
func main() {
	// Set up logging
	logFile, err := os.OpenFile("server.log", os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0644)
	if err != nil {
		log.Fatal(err)
	}
	defer logFile.Close()

	// Create and configure log streamer
	logStreamer := websocket.NewLogStreamer(logFile)
	log.SetOutput(logStreamer)

	// Parse command line flags
	configPath := flag.String("config", "config/settings.yaml", "Path to configuration file")
	flag.Parse()

	// Load configuration
	cfg, err := config.LoadConfig(*configPath)
	if err != nil {
		log.Fatalf("Failed to load configuration: %v", err)
	}

	// Create required directories
	listenersDir := filepath.Join(cfg.Server.StaticDir, "listeners")
	if err := os.MkdirAll(listenersDir, 0755); err != nil {
		log.Fatalf("Failed to create listeners directory: %v", err)
	}
	log.Printf("[CONFIG] Created listeners directory: %s", listenersDir)

	// Initialize components
	fileStore, err := filestore.New(cfg.Server.UploadDir)
	if err != nil {
		log.Fatalf("Failed to initialize file store: %v", err)
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

	// Initialize handlers
	fileHandlers := api.NewFileHandlers(fileStore)
	staticHandlers := web.New(cfg.Server.StaticDir)
	wsHandlers := ws.New(logStreamer)
	listenerHandlers := api.NewListenerHandlers(serverManager.GetListenerManager())

	// Initialize payload handler
	payloadDir := filepath.Join(cfg.Server.StaticDir, "payloads")
	agentSourceDir := "../agent" // Relative path to agent source code
	payloadHandler := api.PayloadHandlerSetup(payloadDir, agentSourceDir, serverManager.GetListenerManager())

	// Set up HTTP routes
	staticHandlers.SetupStaticRoutes()

	// Set up file handling routes
	http.HandleFunc("/api/file_drop/upload", fileHandlers.HandleFileUpload)
	http.HandleFunc("/api/file_drop/list", fileHandlers.HandleFileList)
	http.HandleFunc("/api/file_drop/download/", fileHandlers.HandleFileDownload)
	http.HandleFunc("/api/file_drop/delete/", fileHandlers.HandleFileDelete)

	// Set up WebSocket routes
	http.HandleFunc("/ws/logs", wsHandlers.HandleLogStream)
	http.HandleFunc("/ws/terminal", wsHandlers.HandleTerminal)

	// Set up listener management routes
	listenerHandlers.SetupRoutes()

	// Set up payload generator routes
	payloadHandler.SetupRoutes()

	// Set up root handler
	http.HandleFunc("/", staticHandlers.HandleRoot)

	// Start the server
	log.Printf("[STARTUP] Starting server with %s protocol...", cfg.Communication.Protocol)
	log.Printf("[CONFIG] Upload directory: %s", cfg.Server.UploadDir)
	log.Printf("[CONFIG] Static directory: %s", cfg.Server.StaticDir)
	log.Printf("[CONFIG] File Drop directory: %s/file_drop", cfg.Server.StaticDir)
	log.Printf("[CONFIG] Payloads directory: %s", payloadDir)
	log.Printf("[NETWORK] Port: %s", cfg.Server.Port)

	if err := serverManager.Start(); err != nil {
		log.Fatalf("[ERROR] Server error: %v", err)
	}
}
