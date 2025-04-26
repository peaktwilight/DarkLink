package protocols

import (
	"crypto/tls"
	"encoding/json"
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	"path/filepath"
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
	Config          ListenerConfig `json:"config"`
	Status          ListenerStatus `json:"status"`
	Error           string         `json:"error,omitempty"`
	StartTime       time.Time      `json:"start_time"`
	StopTime        time.Time      `json:"stop_time,omitempty"`
	Stats           ListenerStats  `json:"stats"`
	fileHandler     *FileHandler
	cmdQueue        *CommandQueue
	stopChan        chan struct{}
	listener        net.Listener
	tlsConfig       *tls.Config
	mu              sync.RWMutex
	protocolHandler http.Handler // HTTP handler for http-polling
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
	if config.Protocol == "http-polling" || config.Protocol == "http" {
		protoConfig := BaseProtocolConfig{
			UploadDir: filepath.Join("static", "listeners", config.Name, "uploads"),
			Port:      fmt.Sprintf("%d", config.Port),
		}
		httpProto := NewHTTPPollingProtocol(protoConfig)
		protoHandler = httpProto.GetHTTPHandler()
		proto = httpProto
		// Ensure upload directory exists
		os.MkdirAll(protoConfig.UploadDir, 0755)
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

	// Clear any previous error
	l.Error = ""

	// Create the appropriate listener based on protocol
	var err error
	addr := fmt.Sprintf("%s:%d", l.Config.BindHost, l.Config.Port)

	if l.Config.TLSConfig != nil {
		cert, err := tls.LoadX509KeyPair(l.Config.TLSConfig.CertFile, l.Config.TLSConfig.KeyFile)
		if err != nil {
			return fmt.Errorf("failed to load TLS certificates: %v", err)
		}
		l.tlsConfig = &tls.Config{
			Certificates: []tls.Certificate{cert},
		}
		l.listener, err = tls.Listen("tcp", addr, l.tlsConfig)
	} else {
		l.listener, err = net.Listen("tcp", addr)
	}

	if err != nil {
		l.Status = StatusError
		l.Error = err.Error()
		return fmt.Errorf("failed to start listener: %v", err)
	}

	l.Status = StatusActive
	l.StartTime = time.Now()
	l.StopTime = time.Time{}

	// Start accepting connections
	go l.acceptConnections()

	log.Printf("[INFO] Started listener %s on %s:%d", l.Config.Name, l.Config.BindHost, l.Config.Port)
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

// acceptConnections handles incoming connections
//
// Pre-conditions:
//   - Listener is in active state
//
// Post-conditions:
//   - Accepts and processes incoming connections
//   - Updates listener statistics
//   - Handles errors gracefully
func (l *Listener) acceptConnections() {
	for {
		select {
		case <-l.stopChan:
			return
		default:
			conn, err := l.listener.Accept()
			if err != nil {
				select {
				case <-l.stopChan:
					return
				default:
					l.mu.Lock()
					l.Stats.FailedConnections++
					l.mu.Unlock()
					log.Printf("[ERROR] Failed to accept connection on listener %s: %v", l.Config.Name, err)
					continue
				}
			}

			l.mu.Lock()
			l.Stats.TotalConnections++
			l.Stats.ActiveConnections++
			l.Stats.LastConnection = time.Now()
			l.mu.Unlock()

			// Handle the connection in a goroutine
			go l.handleConnection(conn)
		}
	}
}

// handleConnection processes an individual client connection
//
// Pre-conditions:
//   - Connection is valid and established
//
// Post-conditions:
//   - Processes the connection using the appropriate protocol handler
//   - Updates listener statistics
//   - Handles errors gracefully
func (l *Listener) handleConnection(conn net.Conn) {
	defer func() {
		conn.Close()
		l.mu.Lock()
		l.Stats.ActiveConnections--
		l.mu.Unlock()
	}()

	// Get the appropriate protocol handler
	handler, err := GetConnectionHandler(l)
	if err != nil {
		l.mu.Lock()
		l.Stats.FailedConnections++
		l.Error = err.Error()
		l.mu.Unlock()
		log.Printf("[ERROR] Failed to get connection handler for listener %s: %v", l.Config.Name, err)
		return
	}

	// Handle the connection using the protocol-specific handler
	if err := handler.HandleConnection(conn); err != nil {
		log.Printf("[ERROR] Error handling connection on listener %s: %v", l.Config.Name, err)
		return
	}
}

// handleHTTPConnection processes an individual HTTP client connection
//
// Pre-conditions:
//   - Connection is valid and established
//
// Post-conditions:
//   - Processes the connection using the appropriate protocol handler
//   - Updates listener statistics
//   - Handles errors gracefully
func (l *Listener) handleHTTPConnection(conn net.Conn) error {
	if l.protocolHandler == nil {
		// ...existing code...
	}

	server := &http.Server{
		Handler: l.protocolHandler,
	}
	server.SetKeepAlivesEnabled(false)

	// Create one-shot listener for this connection
	connListener := &oneShotListener{conn: conn}
	return server.Serve(connListener)
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
