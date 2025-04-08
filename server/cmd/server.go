package main

import (
	"encoding/json"
	"fmt"
	"io"
	"io/ioutil"
	"log"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"sync"
	"time"
)

const UPLOAD_DIR = "uploads"

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

var (
	commands = struct {
		sync.Mutex
		queue []string
	}{}

	results = struct {
		sync.Mutex
		queue []CommandResult
	}{}

	agents = struct {
		sync.Mutex
		list map[string]*Agent
	}{list: make(map[string]*Agent)}
)

func getLocalIP() string {
	addrs, err := net.InterfaceAddrs()
	if err != nil {
		return "0.0.0.0"
	}

	for _, addr := range addrs {
		if ipnet, ok := addr.(*net.IPNet); ok && !ipnet.IP.IsLoopback() {
			if ipnet.IP.To4() != nil {
				return ipnet.IP.String()
			}
		}
	}
	return "0.0.0.0"
}

func enableCors(w *http.ResponseWriter) {
	(*w).Header().Set("Access-Control-Allow-Origin", "*")
	(*w).Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS, PUT, DELETE")
	(*w).Header().Set("Access-Control-Allow-Headers", "Content-Type, X-Filename, X-Command")
	(*w).Header().Set("Access-Control-Max-Age", "86400")
}

func handleRoot(w http.ResponseWriter, r *http.Request) {
	if r.URL.Path != "/" {
		http.NotFound(w, r)
		return
	}

	http.ServeFile(w, r, "static/index.html")
}

func handleQueueCommand(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	cmd := make([]byte, r.ContentLength)
	r.Body.Read(cmd)

	commands.Lock()
	commands.queue = append(commands.queue, string(cmd))
	commands.Unlock()

	fmt.Fprintf(w, "Command queued")
}

func handleGetCommand(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	commands.Lock()
	defer commands.Unlock()

	if len(commands.queue) == 0 {
		w.WriteHeader(http.StatusNoContent)
		return
	}

	cmd := commands.queue[0]
	commands.queue = commands.queue[1:]
	fmt.Fprintf(w, cmd)
}

func handleSubmitResult(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Read the complete body
	body, err := ioutil.ReadAll(r.Body)
	if err != nil {
		log.Printf("Error reading body: %v", err)
		http.Error(w, "Error reading request body", http.StatusBadRequest)
		return
	}

	// Create result from the body
	result := CommandResult{
		Command:   r.Header.Get("X-Command"),
		Output:    string(body),
		Timestamp: time.Now().Format("2006-01-02 15:04:05"),
	}

	log.Printf("Received result - Command: %s, Output: %s", result.Command, result.Output)

	results.Lock()
	results.queue = append(results.queue, result)
	results.Unlock()

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{"status": "success"})
}

func handleGetResults(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	w.Header().Set("Content-Type", "application/json")
	results.Lock()
	defer results.Unlock()

	if len(results.queue) == 0 {
		w.Write([]byte("[]"))
		return
	}

	log.Printf("Sending %d results", len(results.queue))

	// Make sure results are properly formatted
	formattedResults := make([]CommandResult, len(results.queue))
	for i, res := range results.queue {
		formattedResults[i] = CommandResult{
			Command:   res.Command,
			Output:    res.Output,
			Timestamp: res.Timestamp,
		}
	}

	if err := json.NewEncoder(w).Encode(formattedResults); err != nil {
		log.Printf("Error encoding results: %v", err)
		http.Error(w, "Error encoding results", http.StatusInternalServerError)
		return
	}
	results.queue = nil
}

func handleFileUpload(w http.ResponseWriter, r *http.Request) {
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

	// Sanitize filename
	filename = filepath.Clean(filename)
	filename = filepath.Base(filename)

	filepath := filepath.Join(UPLOAD_DIR, filename)
	file, err := os.Create(filepath)
	if err != nil {
		log.Printf("Error creating file: %v", err)
		http.Error(w, "Failed to create file", http.StatusInternalServerError)
		return
	}
	defer file.Close()

	if _, err := io.Copy(file, r.Body); err != nil {
		log.Printf("Error writing file: %v", err)
		http.Error(w, "Failed to write file", http.StatusInternalServerError)
		return
	}

	log.Printf("File uploaded: %s", filename)
	w.WriteHeader(http.StatusOK)
}

func handleListFiles(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	files, err := os.ReadDir(UPLOAD_DIR)
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

func handleAgentHeartbeat(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)

	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var agent Agent
	if err := json.NewDecoder(r.Body).Decode(&agent); err != nil {
		log.Printf("[ERROR] Failed to decode agent data: %v", err)
		http.Error(w, "Invalid agent data", http.StatusBadRequest)
		return
	}

	// Get the real IP address
	ip := r.Header.Get("X-Forwarded-For")
	if ip == "" {
		ip = r.Header.Get("X-Real-IP")
	}
	if ip == "" {
		ip, _, _ = net.SplitHostPort(r.RemoteAddr)
	}

	agents.Lock()
	_, exists := agents.list[agent.ID]
	if exists {
		log.Printf("[HEARTBEAT] Agent %s (%s) checked in", agent.ID, ip)
	} else {
		log.Printf("[NEW] Agent %s connected from %s (OS: %s, Hostname: %s)",
			agent.ID, ip, agent.OS, agent.Hostname)
	}

	agent.IP = ip
	agent.LastSeen = time.Now()
	agents.list[agent.ID] = &agent
	agents.Unlock()

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status":      "connected",
		"server_time": time.Now().Format(time.RFC3339),
	})
}

func handleListAgents(w http.ResponseWriter, r *http.Request) {
	enableCors(&w)
	agents.Lock()
	defer agents.Unlock()

	// Clean up stale agents (not seen in last 5 minutes)
	for id, agent := range agents.list {
		if time.Since(agent.LastSeen) > 5*time.Minute {
			delete(agents.list, id)
		}
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(agents.list)
}

func main() {
	// Create required directories
	os.MkdirAll(UPLOAD_DIR, 0755)
	os.MkdirAll("static/agents", 0755)

	// Get local IP and port
	port := "8080"
	if p := os.Getenv("PORT"); p != "" {
		port = p
	}
	localIP := getLocalIP()

	log.Printf("[STARTUP] Server initializing...")
	log.Printf("[NETWORK] Local IP: %s", localIP)
	log.Printf("[NETWORK] Port: %s", port)

	// Set up routes
	http.HandleFunc("/", handleRoot)
	http.HandleFunc("/queue_command", handleQueueCommand)
	http.HandleFunc("/get_command", handleGetCommand)
	http.HandleFunc("/submit_result", handleSubmitResult)
	http.HandleFunc("/get_results", handleGetResults)

	// Add new routes
	http.HandleFunc("/files/upload", handleFileUpload)
	http.HandleFunc("/files/list", handleListFiles)
	http.Handle("/files/", http.StripPrefix("/files/",
		http.FileServer(http.Dir(UPLOAD_DIR))))

	// Add agent routes
	http.HandleFunc("/agent/heartbeat", handleAgentHeartbeat)
	http.HandleFunc("/agent/list", handleListAgents)

	// Serve static files and agents
	fs := http.FileServer(http.Dir("static"))
	http.Handle("/static/", http.StripPrefix("/static/", fs))
	http.Handle("/agents/", http.StripPrefix("/agents/",
		http.FileServer(http.Dir("static/agents"))))

	// Serve static files from uploads directory
	http.Handle("/download/", http.StripPrefix("/download/", http.FileServer(http.Dir(uploadDir))))

	// Update server startup to explicitly set TCP keep-alive
	server := &http.Server{
		Addr:         ":" + port,
		Handler:      nil,
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 10 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	fmt.Printf("\nServer starting...\n")
	fmt.Printf("Local:   http://localhost:%s\n", port)
	fmt.Printf("Network: http://%s:%s\n\n", localIP, port)
	fmt.Printf("Other devices on your network can connect using the Network address\n")

	// Start server with error logging
	log.Printf("Starting server on 0.0.0.0:%s", port)
	log.Fatal(server.ListenAndServe())
}
