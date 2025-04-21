package protocols

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/google/uuid"
)

// SOCKS5 protocol constants following RFC 1928
const (
	// Version identifier/number
	SOCKS5Version = 0x05

	// Authentication methods
	AuthNone     = 0x00 // No authentication
	AuthGSSAPI   = 0x01 // GSSAPI
	AuthPassword = 0x02 // Username/Password
	AuthNoAccept = 0xFF // No acceptable methods

	// Commands
	CmdConnect  = 0x01 // Establish a TCP/IP stream connection
	CmdBind     = 0x02 // Establish a TCP/IP port binding
	CmdUDPAssoc = 0x03 // Associate a UDP port

	// Address types
	AddrTypeIPv4   = 0x01 // IPv4 address
	AddrTypeDomain = 0x03 // Domain name
	AddrTypeIPv6   = 0x04 // IPv6 address

	// Reply codes
	RepSuccess          = 0x00 // Succeeded
	RepServerFailure    = 0x01 // General SOCKS server failure
	RepNotAllowed       = 0x02 // Connection not allowed by ruleset
	RepNetworkUnreach   = 0x03 // Network unreachable
	RepHostUnreach      = 0x04 // Host unreachable
	RepConnRefused      = 0x05 // Connection refused
	RepTTLExpired       = 0x06 // TTL expired
	RepCmdNotSupported  = 0x07 // Command not supported
	RepAddrNotSupported = 0x08 // Address type not supported
)

// SOCKS5Config holds the configuration for a SOCKS5 server or client
type SOCKS5Config struct {
	// Server configuration
	ListenAddr string
	ListenPort int

	// Authentication
	RequireAuth bool
	Username    string
	Password    string

	// Connection settings
	Timeout int // Timeout in seconds

	// Access control
	AllowedIPs      []string // List of allowed client IPs
	DisallowedPorts []int    // List of ports that are not allowed to be accessed
}

// SOCKS5AuthMethod represents the authentication method chosen for a session
type SOCKS5AuthMethod byte

// SOCKS5Command represents a SOCKS5 command
type SOCKS5Command byte

// SOCKS5AddressType represents the type of address in a SOCKS5 request
type SOCKS5AddressType byte

// SOCKS5Reply represents a reply code from the SOCKS5 server
type SOCKS5Reply byte

// SOCKS5TunnelState represents the current state of a SOCKS5 tunnel
type SOCKS5TunnelState struct {
	TunnelID      string    `json:"tunnel_id"`
	SourceAddr    string    `json:"source_addr"`
	TargetAddr    string    `json:"target_addr"`
	CreatedAt     time.Time `json:"created_at"`
	BytesReceived int64     `json:"bytes_received"`
	BytesSent     int64     `json:"bytes_sent"`
	LastActive    time.Time `json:"last_active"`
}

// SOCKS5ServerState represents the state of the SOCKS5 server
type SOCKS5ServerState struct {
	mu            sync.RWMutex
	activeTunnels map[string]*SOCKS5TunnelState
}

// NewSOCKS5ServerState creates a new server state tracker
func NewSOCKS5ServerState() *SOCKS5ServerState {
	return &SOCKS5ServerState{
		activeTunnels: make(map[string]*SOCKS5TunnelState),
	}
}

// trackTunnel adds a new tunnel to the state tracker
func (s *SOCKS5ServerState) trackTunnel(src, dst string) string {
	s.mu.Lock()
	defer s.mu.Unlock()

	tunnelID := uuid.New().String()
	s.activeTunnels[tunnelID] = &SOCKS5TunnelState{
		TunnelID:      tunnelID,
		SourceAddr:    src,
		TargetAddr:    dst,
		CreatedAt:     time.Now(),
		LastActive:    time.Now(),
		BytesReceived: 0,
		BytesSent:     0,
	}
	return tunnelID
}

// updateTunnelStats updates the statistics for a tunnel
func (s *SOCKS5ServerState) updateTunnelStats(tunnelID string, bytesReceived, bytesSent int64) {
	s.mu.Lock()
	defer s.mu.Unlock()

	if tunnel, exists := s.activeTunnels[tunnelID]; exists {
		tunnel.BytesReceived += bytesReceived
		tunnel.BytesSent += bytesSent
		tunnel.LastActive = time.Now()
	}
}

// removeTunnel removes a tunnel from tracking
func (s *SOCKS5ServerState) removeTunnel(tunnelID string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	delete(s.activeTunnels, tunnelID)
}

// listTunnels returns all active tunnels
func (s *SOCKS5ServerState) listTunnels() []*SOCKS5TunnelState {
	s.mu.RLock()
	defer s.mu.RUnlock()

	tunnels := make([]*SOCKS5TunnelState, 0, len(s.activeTunnels))
	for _, tunnel := range s.activeTunnels {
		tunnels = append(tunnels, tunnel)
	}
	return tunnels
}

// getTunnel gets a specific tunnel by ID
func (s *SOCKS5ServerState) getTunnel(tunnelID string) (*SOCKS5TunnelState, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	tunnel, exists := s.activeTunnels[tunnelID]
	return tunnel, exists
}

// SOCKS5Server represents a SOCKS5 proxy server
type SOCKS5Server struct {
	config   SOCKS5Config
	listener net.Listener
	state    *SOCKS5ServerState
}

// NewSOCKS5Server creates a new SOCKS5 server instance
func NewSOCKS5Server(config SOCKS5Config) (*SOCKS5Server, error) {
	return &SOCKS5Server{
		config: config,
		state:  NewSOCKS5ServerState(),
	}, nil
}

// Start starts the SOCKS5 server
func (s *SOCKS5Server) Start() error {
	addr := fmt.Sprintf("%s:%d", s.config.ListenAddr, s.config.ListenPort)
	listener, err := net.Listen("tcp", addr)
	if err != nil {
		return fmt.Errorf("failed to start SOCKS5 server: %v", err)
	}
	s.listener = listener

	log.Printf("SOCKS5 server listening on %s", addr)

	for {
		conn, err := listener.Accept()
		if err != nil {
			log.Printf("Failed to accept connection: %v", err)
			continue
		}

		go s.handleConnection(conn)
	}
}

// Stop stops the SOCKS5 server
func (s *SOCKS5Server) Stop() error {
	if s.listener != nil {
		return s.listener.Close()
	}
	return nil
}

// handleConnection processes a new client connection
func (s *SOCKS5Server) handleConnection(conn net.Conn) {
	defer conn.Close()

	// Set connection timeout if configured
	if s.config.Timeout > 0 {
		conn.SetDeadline(time.Now().Add(time.Duration(s.config.Timeout) * time.Second))
	}

	// Check if client IP is allowed
	if !s.isIPAllowed(conn.RemoteAddr()) {
		log.Printf("Connection from %s denied: IP not allowed", conn.RemoteAddr())
		return
	}

	// Perform handshake
	authMethod, err := s.handleHandshake(conn)
	if err != nil {
		log.Printf("Handshake failed: %v", err)
		return
	}

	// Handle authentication if required
	if authMethod == AuthPassword && s.config.RequireAuth {
		if err := s.handleAuthentication(conn); err != nil {
			log.Printf("Authentication failed: %v", err)
			return
		}
	}

	// Handle client request
	if err := s.handleRequest(conn); err != nil {
		log.Printf("Request handling failed: %v", err)
		return
	}
}

// handleHandshake performs the SOCKS5 handshake
func (s *SOCKS5Server) handleHandshake(conn net.Conn) (SOCKS5AuthMethod, error) {
	// Read version and number of methods
	header := make([]byte, 2)
	if _, err := io.ReadFull(conn, header); err != nil {
		return AuthNoAccept, err
	}

	if header[0] != SOCKS5Version {
		return AuthNoAccept, fmt.Errorf("unsupported SOCKS version: %d", header[0])
	}

	// Read supported methods
	methods := make([]byte, header[1])
	if _, err := io.ReadFull(conn, methods); err != nil {
		return AuthNoAccept, err
	}

	// Select authentication method
	var method SOCKS5AuthMethod = AuthNoAccept
	if s.config.RequireAuth {
		for _, m := range methods {
			if m == AuthPassword {
				method = AuthPassword
				break
			}
		}
	} else {
		for _, m := range methods {
			if m == AuthNone {
				method = AuthNone
				break
			}
		}
	}

	// Send response
	response := []byte{SOCKS5Version, byte(method)}
	if _, err := conn.Write(response); err != nil {
		return AuthNoAccept, err
	}

	return method, nil
}

// handleAuthentication handles username/password authentication
func (s *SOCKS5Server) handleAuthentication(conn net.Conn) error {
	// Read auth version
	header := make([]byte, 2)
	if _, err := io.ReadFull(conn, header); err != nil {
		return err
	}

	// Read username
	username := make([]byte, header[1])
	if _, err := io.ReadFull(conn, username); err != nil {
		return err
	}

	// Read password length
	passLen := make([]byte, 1)
	if _, err := io.ReadFull(conn, passLen); err != nil {
		return err
	}

	// Read password
	password := make([]byte, passLen[0])
	if _, err := io.ReadFull(conn, password); err != nil {
		return err
	}

	// Verify credentials
	if string(username) != s.config.Username || string(password) != s.config.Password {
		conn.Write([]byte{0x01, 0x01}) // Authentication failed
		return fmt.Errorf("invalid credentials")
	}

	// Send success response
	_, err := conn.Write([]byte{0x01, 0x00})
	return err
}

// handleRequest processes the client's connection request
func (s *SOCKS5Server) handleRequest(conn net.Conn) error {
	// Read request header
	header := make([]byte, 4)
	if _, err := io.ReadFull(conn, header); err != nil {
		return err
	}

	if header[0] != SOCKS5Version {
		return fmt.Errorf("invalid SOCKS version")
	}

	// We only support CONNECT command for now
	if header[1] != CmdConnect {
		s.sendReply(conn, RepCmdNotSupported, nil)
		return fmt.Errorf("unsupported command: %d", header[1])
	}

	// Parse target address
	target, err := s.readAddress(conn, header[3])
	if err != nil {
		s.sendReply(conn, RepAddrNotSupported, nil)
		return err
	}

	// Check if port is allowed
	port := target.Port
	if s.isPortDisallowed(port) {
		s.sendReply(conn, RepNotAllowed, nil)
		return fmt.Errorf("port %d is not allowed", port)
	}

	// Connect to target
	targetConn, err := net.DialTimeout("tcp", target.String(), time.Duration(s.config.Timeout)*time.Second)
	if err != nil {
		s.sendReply(conn, RepHostUnreach, nil)
		return err
	}
	defer targetConn.Close()

	// Track the tunnel after successful handshake
	tunnelID := s.state.trackTunnel(conn.RemoteAddr().String(), target.String())
	defer s.state.removeTunnel(tunnelID)

	// Send success reply
	localAddr := targetConn.LocalAddr().(*net.TCPAddr)
	if err := s.sendReply(conn, RepSuccess, localAddr); err != nil {
		return err
	}

	// Start proxying data
	return s.proxyData(conn, targetConn, tunnelID)
}

// readAddress reads the target address from the client request
func (s *SOCKS5Server) readAddress(conn net.Conn, addrType byte) (*net.TCPAddr, error) {
	switch addrType {
	case AddrTypeIPv4:
		addr := make([]byte, 6) // 4 for IPv4 + 2 for port
		if _, err := io.ReadFull(conn, addr); err != nil {
			return nil, err
		}
		return &net.TCPAddr{
			IP:   net.IPv4(addr[0], addr[1], addr[2], addr[3]),
			Port: int(addr[4])<<8 | int(addr[5]),
		}, nil

	case AddrTypeDomain:
		lenByte := make([]byte, 1)
		if _, err := io.ReadFull(conn, lenByte); err != nil {
			return nil, err
		}

		domain := make([]byte, lenByte[0]+2) // +2 for port
		if _, err := io.ReadFull(conn, domain); err != nil {
			return nil, err
		}

		// Resolve domain name
		host := string(domain[:len(domain)-2])
		ips, err := net.LookupIP(host)
		if err != nil {
			return nil, err
		}

		return &net.TCPAddr{
			IP:   ips[0],
			Port: int(domain[len(domain)-2])<<8 | int(domain[len(domain)-1]),
		}, nil

	case AddrTypeIPv6:
		addr := make([]byte, 18) // 16 for IPv6 + 2 for port
		if _, err := io.ReadFull(conn, addr); err != nil {
			return nil, err
		}
		return &net.TCPAddr{
			IP:   addr[:16],
			Port: int(addr[16])<<8 | int(addr[17]),
		}, nil

	default:
		return nil, fmt.Errorf("unsupported address type: %d", addrType)
	}
}

// sendReply sends a reply to the client
func (s *SOCKS5Server) sendReply(conn net.Conn, reply SOCKS5Reply, addr *net.TCPAddr) error {
	response := make([]byte, 4)
	response[0] = SOCKS5Version
	response[1] = byte(reply)
	response[2] = 0x00 // RSV

	if addr == nil {
		response[3] = AddrTypeIPv4
		response = append(response, make([]byte, 6)...) // Null address and port
	} else {
		ip := addr.IP.To4()
		if ip != nil {
			response[3] = AddrTypeIPv4
			response = append(response, ip...)
		} else {
			response[3] = AddrTypeIPv6
			response = append(response, addr.IP.To16()...)
		}
		response = append(response, byte(addr.Port>>8), byte(addr.Port))
	}

	_, err := conn.Write(response)
	return err
}

// proxyData handles bidirectional data transfer
func (s *SOCKS5Server) proxyData(client, target net.Conn, tunnelID string) error {
	errc := make(chan error, 2)

	copy := func(dst, src net.Conn, received bool) {
		written, err := io.Copy(dst, src)
		if received {
			s.state.updateTunnelStats(tunnelID, written, 0)
		} else {
			s.state.updateTunnelStats(tunnelID, 0, written)
		}
		errc <- err
	}

	go copy(client, target, false)
	go copy(target, client, true)

	// Wait for data copy to complete
	err := <-errc
	if err != nil {
		return err
	}

	return <-errc
}

// isIPAllowed checks if the client IP is allowed
func (s *SOCKS5Server) isIPAllowed(addr net.Addr) bool {
	if len(s.config.AllowedIPs) == 0 {
		return true
	}

	host, _, err := net.SplitHostPort(addr.String())
	if err != nil {
		return false
	}

	for _, allowed := range s.config.AllowedIPs {
		if allowed == host {
			return true
		}
	}

	return false
}

// isPortDisallowed checks if the target port is in the disallowed list
func (s *SOCKS5Server) isPortDisallowed(port int) bool {
	for _, p := range s.config.DisallowedPorts {
		if p == port {
			return true
		}
	}
	return false
}

// SOCKS5Protocol implements the Protocol interface for SOCKS5 communication
type SOCKS5Protocol struct {
	config   BaseProtocolConfig
	server   *SOCKS5Server
	commands struct {
		sync.Mutex
		queue []string
	}
	results struct {
		sync.Mutex
		queue []string
	}
}

// NewSOCKS5Protocol creates a new SOCKS5 protocol instance
func NewSOCKS5Protocol(config BaseProtocolConfig) *SOCKS5Protocol {
	return &SOCKS5Protocol{
		config: config,
	}
}

// Initialize sets up the SOCKS5 protocol
func (p *SOCKS5Protocol) Initialize() error {
	serverConfig := SOCKS5Config{
		ListenAddr:      "0.0.0.0",
		ListenPort:      1080, // Default SOCKS5 port
		RequireAuth:     false,
		Timeout:         300,        // 5 minutes timeout
		AllowedIPs:      []string{}, // Allow all by default
		DisallowedPorts: []int{},    // No restricted ports by default
	}

	server, err := NewSOCKS5Server(serverConfig)
	if err != nil {
		return fmt.Errorf("failed to create SOCKS5 server: %v", err)
	}

	p.server = server
	go server.Start() // Start server in background

	return os.MkdirAll(p.config.UploadDir, 0755)
}

// HandleCommand handles sending commands to agents
func (p *SOCKS5Protocol) HandleCommand(cmd string) error {
	p.commands.Lock()
	p.commands.queue = append(p.commands.queue, cmd)
	p.commands.Unlock()
	return nil
}

// HandleFileUpload handles file uploads from agents
func (p *SOCKS5Protocol) HandleFileUpload(filename string, fileData io.Reader) error {
	filepath := filepath.Join(p.config.UploadDir, filename)
	file, err := os.Create(filepath)
	if err != nil {
		return err
	}
	defer file.Close()
	_, err = io.Copy(file, fileData)
	return err
}

// HandleFileDownload handles file downloads to agents
func (p *SOCKS5Protocol) HandleFileDownload(filename string) (io.Reader, error) {
	filepath := filepath.Join(p.config.UploadDir, filename)
	return os.Open(filepath)
}

// HandleAgentHeartbeat processes agent heartbeats
func (p *SOCKS5Protocol) HandleAgentHeartbeat(agentData []byte) error {
	// SOCKS5 doesn't use heartbeats, but implement for interface compliance
	return nil
}

// GetRoutes returns the HTTP routes this protocol needs
func (p *SOCKS5Protocol) GetRoutes() map[string]http.HandlerFunc {
	// SOCKS5 doesn't use HTTP routes
	return make(map[string]http.HandlerFunc)
}

// GetServer returns the SOCKS5 server instance
func (p *SOCKS5Protocol) GetServer() *SOCKS5Server {
	return p.server
}

// UpdateConfig updates the SOCKS5 server configuration
func (p *SOCKS5Protocol) UpdateConfig(config SOCKS5Config) {
	p.server.config = config
}

// ListTunnels returns all active SOCKS5 tunnels
func (p *SOCKS5Protocol) ListTunnels(w http.ResponseWriter, r *http.Request) {
	p.server.handleListTunnels(w, r)
}

// GetTunnel returns details about a specific tunnel
func (p *SOCKS5Protocol) GetTunnel(w http.ResponseWriter, r *http.Request) {
	p.server.handleGetTunnel(w, r)
}

// CloseTunnel closes a specific tunnel
func (p *SOCKS5Protocol) CloseTunnel(w http.ResponseWriter, r *http.Request) {
	p.server.handleCloseTunnel(w, r)
}

// Add HTTP handlers for SOCKS5 management
func (s *SOCKS5Server) handleListTunnels(w http.ResponseWriter, r *http.Request) {
	tunnels := s.state.listTunnels()
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(tunnels)
}

func (s *SOCKS5Server) handleGetTunnel(w http.ResponseWriter, r *http.Request) {
	tunnelID := r.URL.Query().Get("id")
	if tunnelID == "" {
		http.Error(w, "Missing tunnel ID", http.StatusBadRequest)
		return
	}

	tunnel, exists := s.state.getTunnel(tunnelID)
	if !exists {
		http.Error(w, "Tunnel not found", http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(tunnel)
}

func (s *SOCKS5Server) handleCloseTunnel(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	tunnelID := r.URL.Query().Get("id")
	if tunnelID == "" {
		http.Error(w, "Missing tunnel ID", http.StatusBadRequest)
		return
	}

	s.state.removeTunnel(tunnelID)
	w.WriteHeader(http.StatusOK)
}

// GetConfig returns the current SOCKS5 configuration
func (s *SOCKS5Server) GetConfig() SOCKS5Config {
	return s.config
}

// SetConfig updates the SOCKS5 configuration
func (s *SOCKS5Server) SetConfig(config SOCKS5Config) {
	s.config = config
}
