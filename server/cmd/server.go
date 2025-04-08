package main

import (
    "encoding/json"
    "fmt"
    "io/ioutil"
    "log"
    "net"
    "net/http"
    "os"
    "sync"
    "time"
)

const UPLOAD_DIR = "uploads"

type CommandResult struct {
    Command   string `json:"command"`
    Output    string `json:"output"`
    Timestamp string `json:"timestamp"`
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
    (*w).Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
    (*w).Header().Set("Access-Control-Allow-Headers", "Content-Type, X-Filename")
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

func main() {
    // Create uploads directory if it doesn't exist
    os.MkdirAll(UPLOAD_DIR, 0755)

    // Set up routes
    http.HandleFunc("/", handleRoot)
    http.HandleFunc("/queue_command", handleQueueCommand)
    http.HandleFunc("/get_command", handleGetCommand)
    http.HandleFunc("/submit_result", handleSubmitResult)
    http.HandleFunc("/get_results", handleGetResults)

    // Serve static files from uploads directory
    http.Handle("/download/", http.StripPrefix("/download/", http.FileServer(http.Dir(UPLOAD_DIR))))

    // Get local IP and port
    port := "8080"
    if p := os.Getenv("PORT"); p != "" {
        port = p
    }
    localIP := getLocalIP()

    fmt.Printf("\nServer starting...\n")
    fmt.Printf("Local:   http://localhost:%s\n", port)
    fmt.Printf("Network: http://%s:%s\n\n", localIP, port)
    fmt.Printf("Other devices on your network can connect using the Network address\n")

    // Start server on all interfaces
    log.Fatal(http.ListenAndServe("0.0.0.0:"+port, nil))
}
