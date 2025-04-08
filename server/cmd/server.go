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
    (*w).Header().Set("Access-Control-Allow-Headers", "Content-Type, X-Filename, X-Command")
    (*w).Header().Set("Access-Control-Allow-Credentials", "true")
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

func main() {
    // Create uploads directory if it doesn't exist
    uploadDir := os.Getenv("UPLOAD_DIR")
    if uploadDir == "" {
        uploadDir = UPLOAD_DIR
    }
    os.MkdirAll(uploadDir, 0755)

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
        http.FileServer(http.Dir(uploadDir))))

    // Serve static files from uploads directory
    http.Handle("/download/", http.StripPrefix("/download/", http.FileServer(http.Dir(uploadDir))))

    // Get bind address and port
    port := os.Getenv("PORT")
    if port == "" {
        port = "8080"
    }
    
    bindAddr := os.Getenv("BIND_ADDR")
    if bindAddr == "" {
        bindAddr = "0.0.0.0"
    }

    localIP := getLocalIP()

    fmt.Printf("\nServer starting...\n")
    fmt.Printf("Local:   http://localhost:%s\n", port)
    fmt.Printf("Network: http://%s:%s\n\n", localIP, port)
    fmt.Printf("Other devices on your network can connect using the Network address\n")

    log.Printf("Server starting on %s:%s", bindAddr, port)
    log.Fatal(http.ListenAndServe(fmt.Sprintf("%s:%s", bindAddr, port), nil))
}
