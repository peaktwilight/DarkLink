package protocols

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"
)

type HTTPPollingProtocol struct {
	config   BaseProtocolConfig
	mux      *http.ServeMux
	commands struct {
		sync.Mutex
		queue []string
	}
	results struct {
		sync.Mutex
		queue []CommandResult
	}
	agents struct {
		sync.Mutex
		list map[string]*Agent
	}
	listeners struct {
		sync.Mutex
		list map[string]*Listener
	}
}

type CommandResult struct {
	Command   string `json:"command"`
	Output    string `json:"output"`
	Timestamp string `json:"timestamp"`
}

type Agent struct {
	ID       string    `json:"id"`
	OS       string    `json:"os"`
	Hostname string    `json:"hostname"`
	IP       string    `json:"ip"`
	LastSeen time.Time `json:"last_seen"`
	Commands []string  `json:"last_commands"`
}

// NewHTTPPollingProtocol creates a new HTTP polling protocol instance
func NewHTTPPollingProtocol(config BaseProtocolConfig) *HTTPPollingProtocol {
	p := &HTTPPollingProtocol{
		config: config,
		mux:    http.NewServeMux(),
		agents: struct {
			sync.Mutex
			list map[string]*Agent
		}{list: make(map[string]*Agent)},
		listeners: struct {
			sync.Mutex
			list map[string]*Listener
		}{list: make(map[string]*Listener)},
	}

	p.registerRoutes()
	return p
}

func (p *HTTPPollingProtocol) registerRoutes() {
	// Register agent communication routes with /api prefix
	p.mux.HandleFunc("/api/agent/", p.handleAgentRequests)
	log.Printf("[DEBUG] Registered agent routes on HTTP polling protocol")
}

// GetHTTPHandler returns the ServeMux that handles HTTP requests
func (p *HTTPPollingProtocol) GetHTTPHandler() http.Handler {
	return p.mux
}

func (p *HTTPPollingProtocol) handleAgentRequests(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)

	// Handle preflight OPTIONS requests
	if r.Method == http.MethodOptions {
		w.WriteHeader(http.StatusOK)
		return
	}

	// Extract agent ID and action from path
	// Expected format: /api/agent/{agentId}/{action}
	parts := strings.Split(r.URL.Path, "/")
	if len(parts) < 5 {
		log.Printf("[ERROR] Invalid request path: %s", r.URL.Path)
		http.Error(w, "Invalid request path", http.StatusBadRequest)
		return
	}

	agentID := parts[3]
	action := parts[4]

	log.Printf("[DEBUG] Handling %s request from agent %s", action, agentID)

	switch action {
	case "heartbeat":
		p.handleAgentHeartbeat(w, r, agentID)
	case "tasks":
		p.handleAgentTasks(w, r, agentID)
	case "results":
		p.handleAgentResults(w, r, agentID)
	default:
		log.Printf("[ERROR] Unknown action %s from agent %s", action, agentID)
		http.Error(w, "Unknown action", http.StatusNotFound)
	}
}

func (p *HTTPPollingProtocol) handleAgentHeartbeat(w http.ResponseWriter, r *http.Request, agentID string) {
	enableCors(&w)

	// Handle preflight OPTIONS request
	if r.Method == http.MethodOptions {
		w.WriteHeader(http.StatusOK)
		return
	}

	if r.Method != http.MethodPost {
		log.Printf("[ERROR] Invalid method %s for agent %s heartbeat", r.Method, agentID)
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	log.Printf("[DEBUG] Processing heartbeat from agent %s", agentID)

	body, err := io.ReadAll(r.Body)
	if err != nil {
		log.Printf("[ERROR] Failed to read heartbeat body from agent %s: %v", agentID, err)
		http.Error(w, "Error reading request body", http.StatusBadRequest)
		return
	}

	log.Printf("[DEBUG] Received heartbeat data from agent %s: %s", agentID, string(body))

	if err := p.HandleAgentHeartbeat(body); err != nil {
		log.Printf("[ERROR] Failed to process heartbeat from agent %s: %v", agentID, err)
		http.Error(w, fmt.Sprintf("Error processing agent data: %v", err), http.StatusBadRequest)
		return
	}

	log.Printf("[INFO] Successfully processed heartbeat from agent %s", agentID)
	// Build JSON response and include Content-Length
	response := map[string]string{"status": "connected", "time": time.Now().UTC().Format(time.RFC3339)}
	respBytes, err := json.Marshal(response)
	if err != nil {
		log.Printf("[ERROR] Failed to marshal response for agent %s: %v", agentID, err)
		http.Error(w, "Internal server error", http.StatusInternalServerError)
		return
	}
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Content-Length", strconv.Itoa(len(respBytes)))
	w.Write(respBytes)
}

func (p *HTTPPollingProtocol) handleAgentTasks(w http.ResponseWriter, r *http.Request, agentID string) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	log.Printf("[DEBUG] Agent %s requesting tasks", agentID)

	// For now, return empty task list
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode([]interface{}{})
}

func (p *HTTPPollingProtocol) handleAgentResults(w http.ResponseWriter, r *http.Request, agentID string) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	log.Printf("[DEBUG] Received results from agent %s", agentID)

	// Read and process results
	body, err := io.ReadAll(r.Body)
	if err != nil {
		log.Printf("[ERROR] Failed to read results from agent %s: %v", agentID, err)
		http.Error(w, "Error reading request body", http.StatusBadRequest)
		return
	}

	result := CommandResult{
		Command:   r.Header.Get("X-Command"),
		Output:    string(body),
		Timestamp: time.Now().Format("2006-01-02 15:04:05"),
	}

	p.results.Lock()
	p.results.queue = append(p.results.queue, result)
	p.results.Unlock()

	// Acknowledge receipt
	w.WriteHeader(http.StatusOK)
}

// Start implements the Protocol interface
func (p *HTTPPollingProtocol) Start() error {
	log.Printf("[DEBUG] Starting HTTP polling protocol")
	return nil
}

// Stop implements the Protocol interface
func (p *HTTPPollingProtocol) Stop() error {
	log.Printf("[DEBUG] Stopping HTTP polling protocol")
	return nil
}

func (p *HTTPPollingProtocol) Initialize() error {
	return os.MkdirAll(p.config.UploadDir, 0755)
}

func (p *HTTPPollingProtocol) HandleCommand(cmd string) error {
	p.commands.Lock()
	p.commands.queue = append(p.commands.queue, cmd)
	p.commands.Unlock()
	return nil
}

func (p *HTTPPollingProtocol) HandleFileUpload(filename string, fileData io.Reader) error {
	filepath := filepath.Join(p.config.UploadDir, filename)
	file, err := os.Create(filepath)
	if err != nil {
		return err
	}
	defer file.Close()
	_, err = io.Copy(file, fileData)
	return err
}

func (p *HTTPPollingProtocol) HandleFileDownload(filename string) (io.Reader, error) {
	return os.Open(filepath.Join(p.config.UploadDir, filename))
}

func (p *HTTPPollingProtocol) HandleAgentHeartbeat(agentData []byte) error {
	var agent Agent
	if err := json.Unmarshal(agentData, &agent); err != nil {
		return err
	}

	p.agents.Lock()
	agent.LastSeen = time.Now()
	p.agents.list[agent.ID] = &agent
	p.agents.Unlock()

	return nil
}

func (p *HTTPPollingProtocol) GetRoutes() map[string]http.HandlerFunc {
	return map[string]http.HandlerFunc{
		"/queue_command": p.handleQueueCommand,
		"/get_command":   p.handleGetCommand,
		"/submit_result": p.handleSubmitResult,
		"/get_results":   p.handleGetResults,
		"/files/upload":  p.handleFileUpload,
		"/files/list":    p.handleListFiles,
		"/agent/list":    p.handleListAgents,
	}
}

func enableCors(w *http.ResponseWriter) {
	(*w).Header().Set("Access-Control-Allow-Origin", "*")
	(*w).Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS, PUT, DELETE")
	(*w).Header().Set("Access-Control-Allow-Headers", "Content-Type, X-Filename, X-Command")
	(*w).Header().Set("Access-Control-Max-Age", "86400")
}

// HTTP Handlers
func (p *HTTPPollingProtocol) handleQueueCommand(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	cmd := make([]byte, r.ContentLength)
	r.Body.Read(cmd)
	p.HandleCommand(string(cmd))
	fmt.Fprintf(w, "Command queued")
}

func (p *HTTPPollingProtocol) handleGetCommand(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	p.commands.Lock()
	defer p.commands.Unlock()

	if len(p.commands.queue) == 0 {
		w.WriteHeader(http.StatusNoContent)
		return
	}

	cmd := p.commands.queue[0]
	p.commands.queue = p.commands.queue[1:]
	w.Write([]byte(cmd))
}

func (p *HTTPPollingProtocol) handleSubmitResult(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	if r.Method == http.MethodGet {
		w.Header().Set("Content-Type", "application/json")
		p.results.Lock()
		defer p.results.Unlock()

		if len(p.results.queue) == 0 {
			w.Write([]byte("[]"))
			return
		}

		json.NewEncoder(w).Encode(p.results.queue)
		p.results.queue = nil
		return
	}

	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusBadRequest)
		return
	}

	result := CommandResult{
		Command:   r.Header.Get("X-Command"),
		Output:    string(body),
		Timestamp: time.Now().Format("2006-01-02 15:04:05"),
	}

	p.results.Lock()
	p.results.queue = append(p.results.queue, result)
	p.results.Unlock()

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{"status": "success"})
}

func (p *HTTPPollingProtocol) handleGetResults(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	w.Header().Set("Content-Type", "application/json")

	p.results.Lock()
	defer p.results.Unlock()

	if len(p.results.queue) == 0 {
		w.Write([]byte("[]"))
		return
	}

	json.NewEncoder(w).Encode(p.results.queue)
	p.results.queue = nil
}

func (p *HTTPPollingProtocol) handleFileUpload(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	filename := r.Header.Get("X-Filename")
	if filename == "" {
		http.Error(w, "Missing X-Filename header", http.StatusBadRequest)
		return
	}

	if err := p.HandleFileUpload(filename, r.Body); err != nil {
		log.Printf("Error handling file upload: %v", err)
		http.Error(w, "Failed to handle file upload", http.StatusInternalServerError)
		return
	}
}

func (p *HTTPPollingProtocol) handleListFiles(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	files, err := os.ReadDir(p.config.UploadDir)
	if err != nil {
		http.Error(w, "Failed to list files", http.StatusInternalServerError)
		return
	}

	type FileInfo struct {
		Name    string `json:"name"`
		Size    int64  `json:"size"`
		ModTime string `json:"modified"`
	}

	var fileList []FileInfo
	for _, file := range files {
		info, err := file.Info()
		if err != nil {
			continue
		}
		fileList = append(fileList, FileInfo{
			Name:    file.Name(),
			Size:    info.Size(),
			ModTime: info.ModTime().Format(time.RFC3339),
		})
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(fileList)
}

func (p *HTTPPollingProtocol) handleListAgents(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	p.agents.Lock()
	defer p.agents.Unlock()

	// Clean up stale agents (not seen in last 5 minutes)
	for id, agent := range p.agents.list {
		if time.Since(agent.LastSeen) > 5*time.Minute {
			delete(p.agents.list, id)
		}
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(p.agents.list)
}

// Keep this method for internal use even though we're not exposing it via HTTP
func (p *HTTPPollingProtocol) handleListeners(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	w.Header().Set("Content-Type", "application/json")

	p.listeners.Lock()
	defer p.listeners.Unlock()

	// Convert map to slice for JSON response
	listenersList := make([]*Listener, 0, len(p.listeners.list))
	for _, listener := range p.listeners.list {
		listenersList = append(listenersList, listener)
	}

	if err := json.NewEncoder(w).Encode(listenersList); err != nil {
		http.Error(w, "Error encoding listeners", http.StatusInternalServerError)
		return
	}
}
