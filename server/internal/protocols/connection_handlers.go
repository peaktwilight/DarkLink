package protocols

import (
	"bufio"
	"fmt"
	"io"
	"net"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/google/uuid"
)

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
	for _, uri := range h.listener.Config.URIs {
		if strings.HasPrefix(path, uri) {
			validPath = true
			break
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

// DNSHandler implements connection handling for DNS-over-HTTPS listeners
type DNSHandler struct {
	listener *Listener
}

// NewDNSHandler creates a new DNS connection handler
func NewDNSHandler(listener *Listener) *DNSHandler {
	return &DNSHandler{
		listener: listener,
	}
}

func (d *DNSHandler) ValidateConnection(conn net.Conn) error {
	// DNS-over-HTTPS validation logic
	// TODO: Implement DNS-specific validation
	return nil
}

func (d *DNSHandler) HandleConnection(conn net.Conn) error {
	// DNS-over-HTTPS connection handling
	// TODO: Implement DNS protocol handling
	return nil
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

// GetConnectionHandler returns the appropriate connection handler for a protocol
func GetConnectionHandler(listener *Listener) (ConnectionHandler, error) {
	switch strings.ToLower(listener.Config.Protocol) {
	case "http", "https":
		return NewHTTPHandler(listener), nil
	case "dns-over-https":
		return NewDNSHandler(listener), nil
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
