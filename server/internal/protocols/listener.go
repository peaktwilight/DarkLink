package protocols

import (
	"crypto/tls"
	"fmt"
	"log"
	"net"
	"path/filepath"
	"sync"
	"time"
)

// ListenerStatus represents the current state of a listener
type ListenerStatus string

const (
	StatusStopped ListenerStatus = "STOPPED"
	StatusActive  ListenerStatus = "ACTIVE"
	StatusError   ListenerStatus = "ERROR"
)

// ListenerConfig holds the configuration for a C2 listener
type ListenerConfig struct {
	ID           string            `json:"id"`
	Name         string            `json:"name"`
	Protocol     string            `json:"protocol"`
	BindHost     string            `json:"host"`
	Port         int               `json:"port"`
	URIs         []string          `json:"uris,omitempty"`
	Headers      map[string]string `json:"headers,omitempty"`
	UserAgent    string            `json:"user_agent,omitempty"`
	HostRotation string            `json:"host_rotation,omitempty"`
	Hosts        []string          `json:"hosts,omitempty"`
	Proxy        *ProxyConfig      `json:"proxy,omitempty"`
	TLSConfig    *TLSConfig        `json:"tls_config,omitempty"`
}

// ProxyConfig holds proxy-related configuration
type ProxyConfig struct {
	Type     string `json:"type"`
	Host     string `json:"host"`
	Port     int    `json:"port"`
	Username string `json:"username,omitempty"`
	Password string `json:"password,omitempty"`
}

// TLSConfig holds TLS/SSL configuration
type TLSConfig struct {
	CertFile string `json:"cert_file"`
	KeyFile  string `json:"key_file"`
}

// Listener represents a running C2 listener instance
type Listener struct {
	Config      ListenerConfig `json:"config"`
	Status      ListenerStatus `json:"status"`
	LastError   string         `json:"last_error,omitempty"`
	StartTime   time.Time      `json:"start_time"`
	StopTime    time.Time      `json:"stop_time,omitempty"`
	Stats       ListenerStats  `json:"stats"`
	fileHandler *FileHandler
	cmdQueue    *CommandQueue
	stopChan    chan struct{}
	listener    net.Listener
	tlsConfig   *tls.Config
	mu          sync.RWMutex
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

// NewListener creates a new listener instance from a configuration
func NewListener(config ListenerConfig) (*Listener, error) {
	fileHandler, err := NewFileHandler(filepath.Join("uploads", config.Name))
	if err != nil {
		return nil, fmt.Errorf("failed to create file handler: %v", err)
	}

	return &Listener{
		Config:      config,
		Status:      StatusStopped,
		stopChan:    make(chan struct{}),
		Stats:       ListenerStats{},
		fileHandler: fileHandler,
		cmdQueue:    NewCommandQueue(),
	}, nil
}

// GetFileHandler returns the listener's file handler
func (l *Listener) GetFileHandler() *FileHandler {
	return l.fileHandler
}

// GetCommandQueue returns the listener's command queue
func (l *Listener) GetCommandQueue() *CommandQueue {
	return l.cmdQueue
}

// Start initializes and starts the listener
func (l *Listener) Start() error {
	l.mu.Lock()
	defer l.mu.Unlock()

	if l.Status == StatusActive {
		return fmt.Errorf("listener %s is already running", l.Config.Name)
	}

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
		l.LastError = err.Error()
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

// Stop gracefully shuts down the listener
func (l *Listener) Stop() error {
	l.mu.Lock()
	defer l.mu.Unlock()

	if l.Status != StatusActive {
		return fmt.Errorf("listener %s is not running", l.Config.Name)
	}

	close(l.stopChan)
	if err := l.listener.Close(); err != nil {
		return fmt.Errorf("error stopping listener: %v", err)
	}

	l.Status = StatusStopped
	l.StopTime = time.Now()
	log.Printf("[INFO] Stopped listener %s", l.Config.Name)
	return nil
}

// acceptConnections handles incoming connections
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
		l.LastError = err.Error()
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
