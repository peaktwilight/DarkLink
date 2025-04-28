// This file will be moved to the new 'handlers' folder as 'connection_handler.go'.
package handlers

import (
	"bufio"
	"fmt"
	"io"
	"net"
	"net/http"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"

	"microc2/server/internal/behaviour" // Corrected path to the `listeners` package
	"microc2/server/internal/common"    // Import the `common` package for BaseProtocolConfig

	"github.com/google/uuid"
)

// Define missing types
// Listener represents a communication protocol listener
// This is a placeholder definition to resolve errors.
type Listener struct {
	Config struct {
		Protocol  string
		URIs      []string
		Headers   map[string]string
		UserAgent string
		BindHost  string
		Port      int
		Proxy     *ProxyConfig
		Name      string
	}
	mu    sync.Mutex
	Stats struct {
		FailedConnections int
		TotalConnections  int
		ActiveConnections int
	}
	GetFileHandler func() *FileHandler
}

// Define the ProxyConfig type.
type ProxyConfig struct {
	Type     string
	Host     string
	Port     int
	Username string
	Password string
}

// Define the BaseProtocolConfig type.
type BaseProtocolConfig struct {
	UploadDir string
	Port      string
}

// SOCKS5Server is a placeholder for the actual implementation
type SOCKS5Server struct{}

// NewSOCKS5Server is a placeholder function to resolve errors
func NewSOCKS5Server(config interface{}) (*SOCKS5Server, error) {
	return &SOCKS5Server{}, nil
}

// Add the handleConnection method to SOCKS5Server.
func (s *SOCKS5Server) handleConnection(conn net.Conn) error {
	// Placeholder implementation for handling SOCKS5 connections.
	return nil
}

// SOCKS5Config is a placeholder for the actual implementation
type SOCKS5Config struct {
	ListenAddr  string
	ListenPort  int
	RequireAuth bool
	Username    string
	Password    string
	Timeout     int
}

// ConnectionHandler defines the interface for protocol-specific connection handling
type ConnectionHandler interface {
	HandleConnection(conn net.Conn) error
	ValidateConnection(conn net.Conn) error
}

// HTTPHandler implements connection handling for HTTP/HTTPS listeners
type HTTPHandler struct {
	listener *Listener
}

// NewHTTPHandler creates a new HTTP connection handler
func NewHTTPHandler(listener *Listener) *HTTPHandler {
	return &HTTPHandler{
		listener: listener,
	}
}

func (h *HTTPHandler) ValidateConnection(conn net.Conn) error {
	// Set initial read deadline for the HTTP request
	conn.SetReadDeadline(time.Now().Add(time.Second * 10))

	// Create a buffered reader
	reader := bufio.NewReader(conn)

	// Read the first line to get the request method and path
	line, err := reader.ReadString('\n')
	if err != nil {
		return fmt.Errorf("failed to read request line: %v", err)
	}

	// Parse the request line
	parts := strings.Split(strings.TrimSpace(line), " ")
	if len(parts) != 3 {
		return fmt.Errorf("invalid HTTP request line")
	}

	_, path, proto := parts[0], parts[1], parts[2]
	if !strings.HasPrefix(proto, "HTTP/") {
		return fmt.Errorf("invalid protocol: %s", proto)
	}

	// Check if the path matches any configured URIs
	validPath := false
	if len(h.listener.Config.URIs) == 0 {
		// No specific URIs configured, accept all paths
		validPath = true
	} else {
		for _, uri := range h.listener.Config.URIs {
			if strings.HasPrefix(path, uri) {
				validPath = true
				break
			}
		}
	}

	if !validPath {
		return fmt.Errorf("invalid path: %s", path)
	}

	// Read and validate headers
	headers := make(map[string]string)
	for {
		line, err := reader.ReadString('\n')
		if err != nil {
			return fmt.Errorf("error reading headers: %v", err)
		}

		line = strings.TrimSpace(line)
		if line == "" {
			break // End of headers
		}

		parts := strings.SplitN(line, ":", 2)
		if len(parts) != 2 {
			continue
		}

		key := strings.TrimSpace(parts[0])
		value := strings.TrimSpace(parts[1])
		headers[key] = value
	}

	// Validate required headers if configured
	for key, value := range h.listener.Config.Headers {
		if headers[key] != value {
			return fmt.Errorf("missing or invalid header: %s", key)
		}
	}

	// Validate User-Agent if configured
	if h.listener.Config.UserAgent != "" {
		if headers["User-Agent"] != h.listener.Config.UserAgent {
			return fmt.Errorf("invalid User-Agent")
		}
	}

	return nil
}

func (h *HTTPHandler) HandleConnection(conn net.Conn) error {
	defer conn.Close()

	if err := h.ValidateConnection(conn); err != nil {
		h.listener.mu.Lock()
		h.listener.Stats.FailedConnections++
		h.listener.mu.Unlock()
		return fmt.Errorf("connection validation failed: %v", err)
	}

	// Create buffered reader for the connection
	reader := bufio.NewReader(conn)

	// Read the request line
	requestLine, err := reader.ReadString('\n')
	if err != nil {
		return fmt.Errorf("error reading request: %v", err)
	}

	// Parse request line
	parts := strings.Split(strings.TrimSpace(requestLine), " ")
	if len(parts) != 3 {
		return fmt.Errorf("invalid request line")
	}

	method, path := parts[0], parts[1]
	_ = method // Suppress unused variable warning

	// Read headers
	headers := make(map[string]string)
	var contentLength int64
	for {
		line, err := reader.ReadString('\n')
		if err != nil {
			return fmt.Errorf("error reading headers: %v", err)
		}

		line = strings.TrimSpace(line)
		if line == "" {
			break // End of headers
		}

		parts := strings.SplitN(line, ":", 2)
		if len(parts) != 2 {
			continue
		}

		key := strings.ToLower(strings.TrimSpace(parts[0]))
		value := strings.TrimSpace(parts[1])
		headers[key] = value

		if key == "content-length" {
			contentLength, _ = strconv.ParseInt(value, 10, 64)
		}
	}

	// Handle the request based on the path
	switch {
	case strings.HasPrefix(path, "/upload"):
		return h.handleFileUpload(conn, reader, headers, contentLength)
	case strings.HasPrefix(path, "/download"):
		return h.handleFileDownload(conn, path[10:]) // Remove "/download/" prefix
	default:
		return h.handleStandardRequest(conn, method, path, headers, reader, contentLength)
	}
}

func (h *HTTPHandler) handleFileUpload(conn net.Conn, reader *bufio.Reader, headers map[string]string, contentLength int64) error {
	// Get filename from headers
	filename := headers["x-filename"]
	if filename == "" {
		return h.sendErrorResponse(conn, 400, "Missing X-Filename header")
	}

	// Start new upload
	transferID := uuid.New().String()
	_, err := h.listener.GetFileHandler().StartUpload(transferID, filename, contentLength)
	if err != nil {
		return h.sendErrorResponse(conn, 500, fmt.Sprintf("Failed to start upload: %v", err))
	}

	// Read and write file data in chunks
	buffer := make([]byte, 32*1024) // 32KB chunks
	remaining := contentLength

	for remaining > 0 {
		n := int64(len(buffer))
		if remaining < n {
			n = remaining
		}

		read, err := io.ReadFull(reader, buffer[:n])
		if err != nil && err != io.ErrUnexpectedEOF {
			h.listener.GetFileHandler().CancelUpload(transferID)
			return fmt.Errorf("error reading upload data: %v", err)
		}

		if read > 0 {
			if _, err := h.listener.GetFileHandler().WriteChunk(transferID, buffer[:read]); err != nil {
				h.listener.GetFileHandler().CancelUpload(transferID)
				return fmt.Errorf("error writing chunk: %v", err)
			}
		}

		remaining -= int64(read)
	}

	// Send success response
	response := "HTTP/1.1 200 OK\r\n" +
		"Content-Type: application/json\r\n" +
		"Connection: close\r\n" +
		"\r\n" +
		fmt.Sprintf(`{"status":"success","transferId":"%s"}`, transferID)

	_, err = conn.Write([]byte(response))
	return err
}

func (h *HTTPHandler) handleFileDownload(conn net.Conn, filename string) error {
	file, err := h.listener.GetFileHandler().DownloadFile(filename)
	if err != nil {
		return h.sendErrorResponse(conn, 404, "File not found")
	}
	defer file.Close()

	// Write response headers
	response := "HTTP/1.1 200 OK\r\n" +
		"Content-Type: application/octet-stream\r\n" +
		fmt.Sprintf("Content-Disposition: attachment; filename=\"%s\"\r\n", filename) +
		"Connection: close\r\n" +
		"\r\n"

	if _, err := conn.Write([]byte(response)); err != nil {
		return err
	}

	// Copy file data to connection
	if _, err := io.Copy(conn, file); err != nil {
		return fmt.Errorf("error sending file: %v", err)
	}

	return nil
}

func (h *HTTPHandler) handleStandardRequest(conn net.Conn, method, path string, headers map[string]string, reader *bufio.Reader, contentLength int64) error {
	// Create standard response
	response := "HTTP/1.1 200 OK\r\n" +
		"Content-Type: application/json\r\n" +
		"Connection: close\r\n" +
		"\r\n" +
		`{"status":"connected"}`

	_, err := conn.Write([]byte(response))
	return err
}

func (h *HTTPHandler) sendErrorResponse(conn net.Conn, statusCode int, message string) error {
	response := fmt.Sprintf("HTTP/1.1 %d %s\r\n"+
		"Content-Type: application/json\r\n"+
		"Connection: close\r\n"+
		"\r\n"+
		`{"error":"%s"}`, statusCode, http.StatusText(statusCode), message)

	_, err := conn.Write([]byte(response))
	return err
}

// SOCKS5Handler implements connection handling for SOCKS5 listeners
type SOCKS5Handler struct {
	listener *Listener
	server   *SOCKS5Server
}

// NewSOCKS5Handler creates a new SOCKS5 connection handler
func NewSOCKS5Handler(listener *Listener) (*SOCKS5Handler, error) {
	config := SOCKS5Config{
		ListenAddr:  listener.Config.BindHost,
		ListenPort:  listener.Config.Port,
		RequireAuth: false, // Set from listener config if auth is needed
		Timeout:     300,   // 5 minutes default timeout
	}

	// If proxy auth is configured in the listener, set up SOCKS5 auth
	if listener.Config.Proxy != nil && listener.Config.Proxy.Username != "" {
		config.RequireAuth = true
		config.Username = listener.Config.Proxy.Username
		config.Password = listener.Config.Proxy.Password
	}

	server, err := NewSOCKS5Server(config)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize SOCKS5 server: %v", err)
	}
	return &SOCKS5Handler{
		listener: listener,
		server:   server,
	}, nil
}

func (h *SOCKS5Handler) ValidateConnection(conn net.Conn) error {
	// SOCKS5 validation happens during the handshake phase
	return nil
}

func (h *SOCKS5Handler) HandleConnection(conn net.Conn) error {
	defer conn.Close()

	// Update connection stats
	h.listener.mu.Lock()
	h.listener.Stats.TotalConnections++
	h.listener.Stats.ActiveConnections++
	h.listener.mu.Unlock()

	defer func() {
		h.listener.mu.Lock()
		h.listener.Stats.ActiveConnections--
		h.listener.mu.Unlock()
	}()

	// Let the SOCKS5 server handle the connection
	h.server.handleConnection(conn)
	return nil
}

// PollingHandler wraps HTTPPollingProtocol for per-connection serving
type PollingHandler struct {
	proto *behaviour.HTTPPollingProtocol
}

// NewPollingHandler creates a new polling handler for this listener
func NewPollingHandler(listener *Listener) *PollingHandler {
	// Upload directory scoped to listener
	uploadDir := filepath.Join("static", "listeners", listener.Config.Name, "uploads")
	protoConfig := common.BaseProtocolConfig{UploadDir: uploadDir}
	return &PollingHandler{proto: behaviour.NewHTTPPollingProtocol(protoConfig)}
}

func (h *PollingHandler) ValidateConnection(conn net.Conn) error {
	// Always accept; HTTP mux will route
	return nil
}

func (h *PollingHandler) HandleConnection(conn net.Conn) error {
	defer conn.Close()
	server := &http.Server{Handler: h.proto.GetHTTPHandler()}
	server.SetKeepAlivesEnabled(false)
	return server.Serve(&oneShotListener{conn: conn})
}

// oneShotListener implements net.Listener for a single connection
type oneShotListener struct {
	conn net.Conn
}

func (o *oneShotListener) Accept() (net.Conn, error) {
	if o.conn == nil {
		return nil, http.ErrServerClosed
	}
	c := o.conn
	o.conn = nil
	return c, nil
}

func (o *oneShotListener) Close() error {
	return nil
}

func (o *oneShotListener) Addr() net.Addr {
	if o.conn != nil {
		return o.conn.LocalAddr()
	}
	// Dummy address
	return &net.TCPAddr{IP: net.ParseIP("127.0.0.1"), Port: 0}
}

// GetConnectionHandler returns the appropriate connection handler for a protocol
func GetConnectionHandler(listener *Listener) (ConnectionHandler, error) {
	switch strings.ToLower(listener.Config.Protocol) {
	case "http-polling", "http":
		return NewPollingHandler(listener), nil
	case "socks5":
		return NewSOCKS5Handler(listener)
	default:
		return nil, fmt.Errorf("unsupported protocol: %s", listener.Config.Protocol)
	}
}

// Custom ResponseWriter implementation for connection handling
type responseWriter struct {
	headers    http.Header
	body       []byte
	statusCode int
}

func newResponseWriter() *responseWriter {
	return &responseWriter{
		headers:    make(http.Header),
		statusCode: http.StatusOK,
	}
}

func (w *responseWriter) Header() http.Header {
	return w.headers
}

func (w *responseWriter) Write(body []byte) (int, error) {
	w.body = append(w.body, body...)
	return len(body), nil
}

func (w *responseWriter) WriteHeader(statusCode int) {
	w.statusCode = statusCode
}
