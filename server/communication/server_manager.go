package communication

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"microc2/server/communication/protocols"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"
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
		// Serve other static files from the webpage directory
		if _, err := os.Stat(sm.config.StaticDir + "/webpage" + r.URL.Path); err == nil {
			http.ServeFile(w, r, sm.config.StaticDir+"/webpage"+r.URL.Path)
			return
		}
		http.NotFound(w, r)
		return
	}
	http.ServeFile(w, r, sm.config.StaticDir+"/webpage/index.html")
}

func (sm *ServerManager) handleFileDropUpload(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	err := r.ParseMultipartForm(32 << 20) // 32MB max memory
	if err != nil {
		http.Error(w, "Failed to parse form", http.StatusBadRequest)
		return
	}

	files := r.MultipartForm.File["files"]
	for _, fileHeader := range files {
		file, err := fileHeader.Open()
		if err != nil {
			http.Error(w, "Failed to open uploaded file", http.StatusInternalServerError)
			return
		}
		defer file.Close()

		// Create the file_drop directory if it doesn't exist
		if err := os.MkdirAll(filepath.Join(sm.config.StaticDir, "file_drop"), 0755); err != nil {
			http.Error(w, "Failed to create upload directory", http.StatusInternalServerError)
			return
		}

		// Create the destination file
		dst, err := os.Create(filepath.Join(sm.config.StaticDir, "file_drop", fileHeader.Filename))
		if err != nil {
			http.Error(w, "Failed to create destination file", http.StatusInternalServerError)
			return
		}
		defer dst.Close()

		// Copy the uploaded file to the destination
		if _, err := io.Copy(dst, file); err != nil {
			http.Error(w, "Failed to save file", http.StatusInternalServerError)
			return
		}
	}

	w.WriteHeader(http.StatusOK)
}

func (sm *ServerManager) handleFileDropList(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	fileDropDir := filepath.Join(sm.config.StaticDir, "file_drop")
	if err := os.MkdirAll(fileDropDir, 0755); err != nil {
		http.Error(w, "Failed to create file_drop directory", http.StatusInternalServerError)
		return
	}

	files, err := os.ReadDir(fileDropDir)
	if err != nil {
		http.Error(w, "Failed to list files", http.StatusInternalServerError)
		return
	}

	type FileInfo struct {
		Name     string `json:"name"`
		Size     int64  `json:"size"`
		Modified string `json:"modified"`
	}

	fileList := make([]FileInfo, 0)
	for _, file := range files {
		info, err := file.Info()
		if err != nil {
			continue
		}
		fileList = append(fileList, FileInfo{
			Name:     info.Name(),
			Size:     info.Size(),
			Modified: info.ModTime().Format(time.RFC3339),
		})
	}

	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(fileList); err != nil {
		http.Error(w, "Failed to encode file list", http.StatusInternalServerError)
		return
	}
}

func (sm *ServerManager) handleFileDropDownload(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	filename := strings.TrimPrefix(r.URL.Path, "/api/file_drop/download/")
	if filename == "" {
		http.Error(w, "No filename provided", http.StatusBadRequest)
		return
	}

	// Prevent directory traversal
	if strings.Contains(filename, "..") {
		http.Error(w, "Invalid filename", http.StatusBadRequest)
		return
	}

	filePath := filepath.Join(sm.config.StaticDir, "file_drop", filename)
	http.ServeFile(w, r, filePath)
}

func (sm *ServerManager) handleFileDropDelete(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodDelete {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	filename := strings.TrimPrefix(r.URL.Path, "/api/file_drop/delete/")
	if filename == "" {
		http.Error(w, "No filename provided", http.StatusBadRequest)
		return
	}

	// Prevent directory traversal
	if strings.Contains(filename, "..") {
		http.Error(w, "Invalid filename", http.StatusBadRequest)
		return
	}

	filePath := filepath.Join(sm.config.StaticDir, "file_drop", filename)
	if err := os.Remove(filePath); err != nil {
		http.Error(w, "Failed to delete file", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusOK)
}

func (sm *ServerManager) Start() error {
	// Add root handler for index.html and other static files
	http.HandleFunc("/", sm.handleRoot)

	// Set up static file serving
	fs := http.FileServer(http.Dir(sm.config.StaticDir))
	http.Handle("/static/", http.StripPrefix("/static/", fs))

	// Set up webpage serving
	webpageFs := http.FileServer(http.Dir(sm.config.StaticDir + "/webpage"))
	http.Handle("/webpage/", http.StripPrefix("/webpage/", webpageFs))

	// Set up file drop endpoints
	http.HandleFunc("/api/file_drop/upload", sm.handleFileDropUpload)
	http.HandleFunc("/api/file_drop/list", sm.handleFileDropList)
	http.HandleFunc("/api/file_drop/download/", sm.handleFileDropDownload)
	http.HandleFunc("/api/file_drop/delete/", sm.handleFileDropDelete)

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
