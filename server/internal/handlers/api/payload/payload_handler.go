package payload

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/google/uuid"
)

// PayloadConfig defines the structure for payload generation configuration
type PayloadConfig struct {
	AgentType       string `json:"agentType"`
	ListenerID      string `json:"listener"`
	Architecture    string `json:"architecture"`
	Format          string `json:"format"`
	Sleep           int    `json:"sleep"`
	IndirectSyscall bool   `json:"indirectSyscall"`
	SleepTechnique  string `json:"sleepTechnique"`
	DllSideloading  bool   `json:"dllSideloading"`
	SideloadDll     string `json:"sideloadDll,omitempty"`
	ExportName      string `json:"exportName,omitempty"`
}

// PayloadResult contains information about a generated payload
type PayloadResult struct {
	ID       string `json:"id"`
	Filename string `json:"filename"`
	Path     string `json:"path"`
	Size     int64  `json:"size"`
	Created  string `json:"created"`
}

// PayloadHandler manages payload generation operations
// It coordinates all aspects of agent payload creation, including
// building, storing, and serving payload files to users.
type PayloadHandler struct {
	payloadsDir    string
	agentSourceDir string
	listeners      ListenerGetter
	mutex          sync.Mutex
	payloads       map[string]PayloadResult
}

// ListenerGetter defines an interface for retrieving listener information
type ListenerGetter interface {
	GetListener(id string) (Listener, error)
}

// Listener represents a simplified version of a listener for payload generation
type Listener struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	Protocol string `json:"protocol"`
	Host     string `json:"host"`
	Port     int    `json:"port"`
}

// NewPayloadHandler creates a new payload handler
//
// Pre-conditions:
//   - payloadsDir is a valid directory path with write permissions
//   - agentSourceDir points to a valid agent source code directory
//   - listeners is a properly initialized ListenerGetter implementation
//
// Post-conditions:
//   - Returns an initialized PayloadHandler
//   - Directory structure for payloads is created if it doesn't exist
//   - Tracking map for generated payloads is initialized
func NewPayloadHandler(payloadsDir, agentSourceDir string, listeners ListenerGetter) *PayloadHandler {
	// Ensure directories exist
	for _, dir := range []string{payloadsDir, filepath.Join(payloadsDir, "debug"), filepath.Join(payloadsDir, "release")} {
		if err := os.MkdirAll(dir, 0755); err != nil {
			log.Printf("[ERROR] Failed to create directory %s: %v", dir, err)
		}
	}

	return &PayloadHandler{
		payloadsDir:    payloadsDir,
		agentSourceDir: agentSourceDir,
		listeners:      listeners,
		payloads:       make(map[string]PayloadResult),
	}
}

// HandleGeneratePayload processes a request to generate a payload
//
// Pre-conditions:
//   - HTTP request contains a valid JSON payload configuration
//   - Request method is POST
//
// Post-conditions:
//   - Payload is generated according to the provided configuration
//   - Response contains the generated payload details or an error
//   - Generated payload is stored and tracked for later retrieval
func (h *PayloadHandler) HandleGeneratePayload(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var config PayloadConfig
	if err := json.NewDecoder(r.Body).Decode(&config); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	// Generate payload
	result, err := h.GeneratePayload(config)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Store result for later retrieval
	h.mutex.Lock()
	h.payloads[result.ID] = result
	h.mutex.Unlock()

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(result)
}

// HandleDownloadPayload serves a generated payload for download
//
// Pre-conditions:
//   - Request contains a valid payload ID in the URL path
//   - Payload with the specified ID exists in the handler's registry
//
// Post-conditions:
//   - Payload file is streamed to the client for download
//   - Appropriate headers for file download are set
//   - Error response is sent if the payload is not found
func (h *PayloadHandler) HandleDownloadPayload(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Extract payload ID from URL path
	id := strings.TrimPrefix(r.URL.Path, "/api/payload/download/")
	if id == "" {
		http.Error(w, "Payload ID is required", http.StatusBadRequest)
		return
	}

	// Look up payload result
	h.mutex.Lock()
	result, exists := h.payloads[id]
	h.mutex.Unlock()

	if !exists {
		http.Error(w, "Payload not found", http.StatusNotFound)
		return
	}

	// Open file
	file, err := os.Open(result.Path)
	if err != nil {
		http.Error(w, "Failed to read payload file", http.StatusInternalServerError)
		log.Printf("[ERROR] Failed to open payload file %s: %v", result.Path, err)
		return
	}
	defer file.Close()

	// Set appropriate headers
	w.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=\"%s\"", result.Filename))
	w.Header().Set("Content-Type", "application/octet-stream")
	w.Header().Set("Content-Length", fmt.Sprintf("%d", result.Size))

	// Stream file to response
	if _, err := io.Copy(w, file); err != nil {
		log.Printf("[ERROR] Failed to stream payload file %s: %v", result.Path, err)
	}
}

// GeneratePayload creates a payload based on the provided configuration
//
// Pre-conditions:
//   - config contains valid payload generation parameters
//   - Listener specified in config exists and is accessible
//   - Agent source code is available and can be built
//
// Post-conditions:
//   - Agent payload is built and stored in the payloads directory
//   - Returns PayloadResult with details about the generated payload
//   - Returns error if payload generation fails at any step
func (h *PayloadHandler) GeneratePayload(config PayloadConfig) (PayloadResult, error) {
	log.Printf("[INFO] Generating payload with config: %+v", config)

	// Get listener details
	listener, err := h.listeners.GetListener(config.ListenerID)
	if err != nil {
		return PayloadResult{}, fmt.Errorf("failed to get listener: %w", err)
	}

	// Generate unique ID for this payload
	payloadID := uuid.New().String()

	// Determine build type (debug or release)
	buildType := "release"
	if config.AgentType == "debugAgent" {
		buildType = "debug"
	}

	// Create a directory for build artifacts
	outputDir := filepath.Join(h.payloadsDir, buildType, payloadID)
	if err := os.MkdirAll(outputDir, 0755); err != nil {
		return PayloadResult{}, fmt.Errorf("failed to create output directory: %w", err)
	}

	// Create agent config file
	configPath := filepath.Join(outputDir, "config.json")
	agentConfig := map[string]interface{}{
		"server_url":     fmt.Sprintf("%s:%d", listener.Host, listener.Port),
		"sleep_interval": config.Sleep,
		"jitter":         2, // Default jitter value
	}

	configJSON, err := json.MarshalIndent(agentConfig, "", "  ")
	if err != nil {
		return PayloadResult{}, fmt.Errorf("failed to marshal agent config: %w", err)
	}

	if err := os.WriteFile(configPath, configJSON, 0644); err != nil {
		return PayloadResult{}, fmt.Errorf("failed to write agent config: %w", err)
	}

	// Determine build target
	var buildTarget string
	switch {
	case config.Format == "windows_exe" || config.Format == "windows_dll" || config.Format == "windows_service":
		buildTarget = "x86_64-pc-windows-gnu"
	case config.Format == "linux_elf":
		buildTarget = "x86_64-unknown-linux-gnu"
	case config.Architecture == "arm64":
		buildTarget = "aarch64-unknown-linux-gnu"
	default:
		buildTarget = "x86_64-unknown-linux-gnu" // Default to Linux x64
	}

	// Get the path to the build script
	buildScript := filepath.Join(h.agentSourceDir, "build.sh")

	// Set up the command
	cmd := exec.Command("/bin/bash", buildScript,
		"--target", buildTarget,
		"--output", outputDir,
		"--build-type", buildType)

	// Set working directory to agent source directory
	cmd.Dir = h.agentSourceDir

	// Add environment variables
	cmd.Env = append(os.Environ(),
		fmt.Sprintf("TARGET=%s", buildTarget),
		fmt.Sprintf("OUTPUT_DIR=%s", outputDir),
		fmt.Sprintf("BUILD_TYPE=%s", buildType),
		fmt.Sprintf("LISTENER_HOST=%s", listener.Host),
		fmt.Sprintf("LISTENER_PORT=%d", listener.Port),
		fmt.Sprintf("SLEEP_INTERVAL=%d", config.Sleep),
	)

	// Execute build command
	output, err := cmd.CombinedOutput()
	if err != nil {
		log.Printf("[ERROR] Build command failed: %v\nOutput: %s", err, output)
		return PayloadResult{}, fmt.Errorf("build failed: %v - %s", err, output)
	}

	log.Printf("[INFO] Build output: %s", output)

	// Determine payload filename
	var payloadFileName string
	switch {
	case config.Format == "windows_exe":
		payloadFileName = "agent.exe"
	case config.Format == "windows_dll":
		payloadFileName = "agent.dll"
	case config.Format == "windows_service":
		payloadFileName = "agent_service.exe"
	default:
		payloadFileName = "agent"
	}

	// Find the generated payload
	payloadPath := filepath.Join(outputDir, payloadFileName)

	// Check if file exists
	fileInfo, err := os.Stat(payloadPath)
	if err != nil {
		return PayloadResult{}, fmt.Errorf("payload not found at expected location: %w", err)
	}

	// Create the result
	result := PayloadResult{
		ID:       payloadID,
		Filename: payloadFileName,
		Path:     payloadPath,
		Size:     fileInfo.Size(),
		Created:  time.Now().Format(time.RFC3339),
	}

	log.Printf("[INFO] Generated payload: %+v", result)
	return result, nil
}

// SetupRoutes registers all payload-related routes
//
// Pre-conditions:
//   - HTTP server is initialized and ready to accept route registrations
//
// Post-conditions:
//   - Routes for payload generation and download are registered
//   - Requests to these routes will be handled by the appropriate methods
func (h *PayloadHandler) SetupRoutes() {
	http.HandleFunc("/api/payload/generate", h.HandleGeneratePayload)
	http.HandleFunc("/api/payload/download/", h.HandleDownloadPayload)
}
