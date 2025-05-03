// This file will be moved to the new 'listeners' folder as 'listener.go'.

package listeners

import (
	"crypto/tls"
	"encoding/json"
	"fmt"
	"log"
	behaviour "microc2/server/internal/behaviour"
	"microc2/server/internal/common" // Import BaseProtocolConfig
	"net"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

// ListenerStatus represents the current operational state of a listener
type ListenerStatus string

const (
	// StatusActive indicates the listener is running and accepting connections
	StatusActive ListenerStatus = "ACTIVE"

	// StatusStopped indicates the listener is not running
	StatusStopped ListenerStatus = "STOPPED"

	// StatusError indicates the listener encountered an error
	StatusError ListenerStatus = "ERROR"
)

// ListenerConfig holds the configuration for a C2 listener
type ListenerConfig struct {
	ID           string                `json:"id"`
	Name         string                `json:"name"`
	Protocol     string                `json:"protocol"`
	BindHost     string                `json:"host"`
	Port         int                   `json:"port"`
	URIs         []string              `json:"uris,omitempty"`
	Headers      map[string]string     `json:"headers,omitempty"`
	UserAgent    string                `json:"user_agent,omitempty"`
	HostRotation string                `json:"host_rotation,omitempty"`
	Hosts        []string              `json:"hosts,omitempty"`
	Proxy        *ProxyConfig          `json:"proxy,omitempty"`
	TLSConfig    *TLSConfig            `json:"tls_config,omitempty"`
	SOCKS5Config *SOCKS5ListenerConfig `json:"socks5_config,omitempty"`
}

// ProxyConfig holds proxy-related configuration
type ProxyConfig struct {
	Type     string `json:"type"`
	Host     string `json:"host"`
	Port     int    `json:"port"`
	Username string `json:"username,omitempty"`
	Password string `json:"password,omitempty"`
}

// TLSConfig holds TLS configuration for secure listeners
type TLSConfig struct {
	CertFile          string `json:"cert_file"`
	KeyFile           string `json:"key_file"`
	RequireClientCert bool   `json:"requireClientCert"`
}

// SOCKS5ListenerConfig holds SOCKS5-specific listener configuration
type SOCKS5ListenerConfig struct {
	RequireAuth     bool     `json:"require_auth"`
	AllowedIPs      []string `json:"allowed_ips,omitempty"`
	DisallowedPorts []int    `json:"disallowed_ports,omitempty"`
	IdleTimeout     int      `json:"idle_timeout,omitempty"` // Timeout in seconds
}

// Listener represents a communication protocol listener that agents connect to
// It manages the lifecycle of the listening service and tracks its operational state.
type Listener struct {
	Config          ListenerConfig    `json:"config"`
	Status          ListenerStatus    `json:"status"`
	Error           string            `json:"error,omitempty"`
	StartTime       time.Time         `json:"start_time"`
	StopTime        time.Time         `json:"stop_time,omitempty"`
	Stats           ListenerStats     `json:"stats"`
	URIs            []string          `json:"uris,omitempty"`
	Headers         map[string]string `json:"headers,omitempty"`
	UserAgent       string            `json:"user_agent,omitempty"`
	mu              sync.RWMutex
	fileHandler     *FileHandler
	cmdQueue        *CommandQueue
	stopChan        chan struct{}
	listener        net.Listener
	tlsConfig       *tls.Config
	protocolHandler http.Handler // HTTP handler for http
	Protocol        Protocol     // underlying protocol instance
}

// ListenerStats tracks operational statistics for a listener
type ListenerStats struct {
	TotalConnections  int64     `json:"total_connections"`
	ActiveConnections int64     `json:"active_connections"`
	LastConnection    time.Time `json:"last_connection,omitempty"`
	BytesReceived     int64     `json:"bytes_received"`
	BytesSent         int64     `json:"bytes_sent"`
	FailedConnections int64     `json:"failed_connections"`
}

// NewListener creates a new listener instance with the given configuration
//
// Pre-conditions:
//   - config is a valid ListenerConfig instance
//   - Protocol specified in config must be supported
//
// Post-conditions:
//   - Returns an initialized Listener instance with appropriate protocol handler
//   - Listener is in stopped state
//   - Returns error if the protocol is not supported or configuration is invalid
func NewListener(config ListenerConfig) (*Listener, error) {
	// Create listener-specific directory in static/listeners
	listenerDir := filepath.Join("static", "listeners", config.Name)
	if err := os.MkdirAll(listenerDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create listener directory: %v", err)
	}

	// Save configuration to file
	configJson, err := json.MarshalIndent(config, "", "    ")
	if err != nil {
		return nil, fmt.Errorf("failed to marshal listener config: %v", err)
	}

	configPath := filepath.Join(listenerDir, "config.json")
	if err := os.WriteFile(configPath, configJson, 0644); err != nil {
		return nil, fmt.Errorf("failed to save listener config: %v", err)
	}

	// Initialize file handler with the listener-specific directory
	fileHandler, err := NewFileHandler(listenerDir)
	if err != nil {
		return nil, fmt.Errorf("failed to create file handler: %v", err)
	}

	// Initialize protocol handler based on config
	var protoHandler http.Handler
	var proto Protocol
	switch config.Protocol {
	case "http", "https":
		protoConfig := common.BaseProtocolConfig{
			UploadDir: filepath.Join("static", "listeners", config.Name, "uploads"),
			Port:      fmt.Sprintf("%d", config.Port),
		}
		httpProto := behaviour.NewHTTPPollingProtocol(protoConfig)
		protoHandler = httpProto.GetHTTPHandler()
		proto = httpProto
		// Ensure upload directory exists
		os.MkdirAll(protoConfig.UploadDir, 0755)
	case "DNSoverHTTPS":
		// DNSoverHTTPS logic (may be implemented later)
		return nil, fmt.Errorf("DNSoverHTTPS protocol is not implemented yet")
	}

	// Construct listener instance
	l := &Listener{
		Config:          config,
		Status:          StatusStopped,
		stopChan:        make(chan struct{}),
		Stats:           ListenerStats{},
		fileHandler:     fileHandler,
		cmdQueue:        NewCommandQueue(),
		protocolHandler: protoHandler,
		Protocol:        proto,
	}
	return l, nil
}

// GetFileHandler returns the listener's file handler
//
// Pre-conditions:
//   - None
//
// Post-conditions:
//   - Returns the file handler associated with the listener
func (l *Listener) GetFileHandler() *FileHandler {
	return l.fileHandler
}

// GetCommandQueue returns the listener's command queue
//
// Pre-conditions:
//   - None
//
// Post-conditions:
//   - Returns the command queue associated with the listener
func (l *Listener) GetCommandQueue() *CommandQueue {
	return l.cmdQueue
}

// Start initiates the listener
//
// Pre-conditions:
//   - Listener is in stopped state
//   - Required resources (ports, etc.) are available
//
// Post-conditions:
//   - Listener is started and accepting connections
//   - Status is updated to Active
//   - StartTime is updated
//   - Returns error if the listener can't be started
func (l *Listener) Start() error {
	l.mu.Lock()
	defer l.mu.Unlock()

	if l.Status == StatusActive {
		return fmt.Errorf("listener %s is already running", l.Config.Name)
	}

	l.Error = ""
	addr := fmt.Sprintf("%s:%d", l.Config.BindHost, l.Config.Port)

	server := &http.Server{
		Addr:    addr,
		Handler: l.protocolHandler,
	}

	go func() {
		var err error
		if l.Config.Protocol == "https" {
			certFile := "certs/server.crt"
			keyFile := "certs/server.key"
			log.Printf("[DEBUG] Loading TLS configuration from %s and %s", certFile, keyFile)
			log.Printf("[DEBUG] Starting HTTPS server on %s", addr)
			err = server.ListenAndServeTLS(certFile, keyFile)
		} else {
			log.Printf("[DEBUG] Starting HTTP server on %s", addr)
			err = server.ListenAndServe()
		}
		if err != nil && err != http.ErrServerClosed {
			log.Printf("[ERROR] HTTP server error: %v", err)
			l.SetError(err)
		}
	}()

	l.Status = StatusActive
	l.StartTime = time.Now()
	l.StopTime = time.Time{}
	return nil
}

// Stop halts the listener operation
//
// Pre-conditions:
//   - Listener is in active state
//
// Post-conditions:
//   - Listener is stopped and no longer accepting connections
//   - Status is updated to Stopped
//   - StopTime is updated
//   - Resources are released
//   - Returns error if the listener can't be stopped cleanly
func (l *Listener) Stop() error {
	l.mu.Lock()
	defer l.mu.Unlock()

	if l.Status != StatusActive {
		return fmt.Errorf("listener %s is not running", l.Config.Name)
	}

	// Signal the stop channel to shut down the handler
	close(l.stopChan)

	if err := l.listener.Close(); err != nil {
		l.Error = err.Error()
		return fmt.Errorf("error stopping listener: %v", err)
	}

	l.Status = StatusStopped
	l.StopTime = time.Now()
	log.Printf("[INFO] Stopped listener %s", l.Config.Name)
	return nil
}

// GetStatus returns the current status of the listener
//
// Pre-conditions:
//   - None
//
// Post-conditions:
//   - Returns the current listener status in a thread-safe manner
func (l *Listener) GetStatus() ListenerStatus {
	l.mu.RLock()
	defer l.mu.RUnlock()
	return l.Status
}

// GetError returns any error encountered by the listener
//
// Pre-conditions:
//   - None
//
// Post-conditions:
//   - Returns the current error message in a thread-safe manner
//   - Returns empty string if no error
func (l *Listener) GetError() string {
	l.mu.RLock()
	defer l.mu.RUnlock()
	return l.Error
}

// SetError sets an error state for the listener
//
// Pre-conditions:
//   - Error message is meaningful and describes the issue
//
// Post-conditions:
//   - Listener status is updated to Error
//   - Error message is stored
func (l *Listener) SetError(err error) {
	l.mu.Lock()
	defer l.mu.Unlock()

	l.Status = StatusError
	if err != nil {
		l.Error = err.Error()
	} else {
		l.Error = "Unknown error"
	}
}

// Define the oneShotListener type.
type oneShotListener struct {
	conn net.Conn
}

func (o *oneShotListener) Accept() (net.Conn, error) {
	if o.conn == nil {
		return nil, fmt.Errorf("no connection available")
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
	return &net.TCPAddr{IP: net.ParseIP("127.0.0.1"), Port: 0}
}

// Define the GetConnectionHandler function.
func GetConnectionHandler(listener *Listener) (ConnectionHandler, error) {
	switch strings.ToLower(listener.Config.Protocol) {
	case "http", "https":
		return NewPollingHandler(listener), nil
	case "socks5":
		return NewSOCKS5Handler(listener)
	default:
		return nil, fmt.Errorf("unsupported protocol: %s", listener.Config.Protocol)
	}
}

// Define the ConnectionHandler interface.
type ConnectionHandler interface {
	HandleConnection(conn net.Conn) error
	ValidateConnection(conn net.Conn) error
}

// Define the NewPollingHandler function.
func NewPollingHandler(listener *Listener) *PollingHandler {
	return &PollingHandler{
		proto: behaviour.NewHTTPPollingProtocol(common.BaseProtocolConfig{
			UploadDir: filepath.Join("static", "listeners", listener.Config.Name, "uploads"),
		}),
	}
}

// Define the NewSOCKS5Handler function.
func NewSOCKS5Handler(listener *Listener) (*SOCKS5Handler, error) {
	return &SOCKS5Handler{
		listener: listener,
	}, nil
}

// Define the PollingHandler type.
type PollingHandler struct {
	proto *behaviour.HTTPPollingProtocol
}

// Add the HandleConnection method to PollingHandler.
func (h *PollingHandler) HandleConnection(conn net.Conn) error {
	defer conn.Close()
	server := &http.Server{Handler: h.proto.GetHTTPHandler()}
	server.SetKeepAlivesEnabled(false)
	return server.Serve(&oneShotListener{conn: conn})
}

// Add the ValidateConnection method to PollingHandler.
func (h *PollingHandler) ValidateConnection(conn net.Conn) error {
	// Placeholder implementation for validating connections.
	return nil
}

// Define the SOCKS5Handler type.
type SOCKS5Handler struct {
	listener *Listener
}

// Add the HandleConnection method to SOCKS5Handler.
func (h *SOCKS5Handler) HandleConnection(conn net.Conn) error {
	defer conn.Close()
	// Placeholder implementation for SOCKS5 connection handling.
	return nil
}

// Add the ValidateConnection method to SOCKS5Handler.
func (h *SOCKS5Handler) ValidateConnection(conn net.Conn) error {
	// Placeholder implementation for validating SOCKS5 connections.
	return nil
}

// Define missing types
// FileHandler is a placeholder for the actual implementation
type FileHandler struct{}

// NewFileHandler is a placeholder function to resolve errors
func NewFileHandler(dir string) (*FileHandler, error) {
	return &FileHandler{}, nil
}

// CommandQueue is a placeholder for the actual implementation
type CommandQueue struct{}

// NewCommandQueue is a placeholder function to resolve errors
func NewCommandQueue() *CommandQueue {
	return &CommandQueue{}
}
