package protocols

import (
	"fmt"
	"log"
	"sync"
	"time"

	"github.com/google/uuid"
)

// ListenerManager handles the lifecycle and management of all C2 listeners
type ListenerManager struct {
	listeners map[string]*Listener
	mu        sync.RWMutex
}

// NewListenerManager creates a new listener manager instance
func NewListenerManager() *ListenerManager {
	return &ListenerManager{
		listeners: make(map[string]*Listener),
	}
}

// CreateListener creates and starts a new listener with the given configuration
func (m *ListenerManager) CreateListener(config ListenerConfig) (*Listener, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	// Generate ID if not provided
	if config.ID == "" {
		config.ID = uuid.New().String()
	}

	// Validate configuration
	if err := m.validateListenerConfig(config); err != nil {
		return nil, fmt.Errorf("invalid listener configuration: %v", err)
	}

	// Check for port conflicts
	if m.hasPortConflict(config) {
		return nil, fmt.Errorf("port %d is already in use by another listener", config.Port)
	}

	listener, err := NewListener(config)
	if err != nil {
		return nil, err
	}

	if err := listener.Start(); err != nil {
		return nil, err
	}

	m.listeners[config.ID] = listener
	return listener, nil
}

// GetListener retrieves a listener by its ID
func (m *ListenerManager) GetListener(id string) (*Listener, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	listener, exists := m.listeners[id]
	if !exists {
		return nil, fmt.Errorf("listener %s not found", id)
	}
	return listener, nil
}

// ListListeners returns a list of all active listeners
func (m *ListenerManager) ListListeners() []*Listener {
	m.mu.RLock()
	defer m.mu.RUnlock()

	list := make([]*Listener, 0, len(m.listeners))
	for _, listener := range m.listeners {
		list = append(list, listener)
	}
	return list
}

// StopListener stops a listener but keeps it in the manager
func (m *ListenerManager) StopListener(id string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	listener, exists := m.listeners[id]
	if !exists {
		return fmt.Errorf("listener %s not found", id)
	}

	if err := listener.Stop(); err != nil {
		return err
	}

	// No longer delete from map - just keep it with stopped status
	return nil
}

// DeleteListener stops (if running) and removes a listener from the manager
func (m *ListenerManager) DeleteListener(id string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	listener, exists := m.listeners[id]
	if !exists {
		return fmt.Errorf("listener %s not found", id)
	}

	// If listener is active, stop it first
	if listener.Status == StatusActive {
		if err := listener.Stop(); err != nil {
			return fmt.Errorf("failed to stop listener before deletion: %v", err)
		}
	}

	// Remove from listeners map
	delete(m.listeners, id)
	return nil
}

// StopAll stops all active listeners but keeps them in the manager
func (m *ListenerManager) StopAll() []error {
	m.mu.Lock()
	defer m.mu.Unlock()

	var errors []error
	for id, listener := range m.listeners {
		if listener.Status == StatusActive {
			if err := listener.Stop(); err != nil {
				errors = append(errors, fmt.Errorf("failed to stop listener %s: %v", id, err))
			}
		}
	}
	return errors
}

// DeleteAll stops and removes all listeners
func (m *ListenerManager) DeleteAll() []error {
	m.mu.Lock()
	defer m.mu.Unlock()

	var errors []error
	for id, listener := range m.listeners {
		if listener.Status == StatusActive {
			if err := listener.Stop(); err != nil {
				errors = append(errors, fmt.Errorf("failed to stop listener %s: %v", id, err))
				continue // Skip deletion if stopping fails
			}
		}
		delete(m.listeners, id)
	}
	return errors
}

// validateListenerConfig checks if the listener configuration is valid
func (m *ListenerManager) validateListenerConfig(config ListenerConfig) error {
	if config.Name == "" {
		log.Printf("[ERROR] Listener validation failed: name is required")
		return fmt.Errorf("listener name is required")
	}

	if config.Protocol == "" {
		log.Printf("[ERROR] Listener validation failed: protocol is required")
		return fmt.Errorf("protocol is required")
	}

	if config.Port < 1 || config.Port > 65535 {
		log.Printf("[ERROR] Listener validation failed: invalid port number %d", config.Port)
		return fmt.Errorf("invalid port number: %d", config.Port)
	}

	if config.BindHost == "" {
		log.Printf("[INFO] BindHost not provided, defaulting to 0.0.0.0")
		config.BindHost = "0.0.0.0" // Set default bind address
	}

	// Validate TLS configuration if provided
	if config.TLSConfig != nil {
		if config.TLSConfig.CertFile == "" || config.TLSConfig.KeyFile == "" {
			log.Printf("[ERROR] Listener validation failed: both certificate and key files are required for TLS")
			return fmt.Errorf("both certificate and key files are required for TLS")
		}
	}

	log.Printf("[INFO] Listener configuration validated successfully: %+v", config)
	return nil
}

// hasPortConflict checks if the given port is already in use by another listener
func (m *ListenerManager) hasPortConflict(config ListenerConfig) bool {
	for _, l := range m.listeners {
		if l.Config.Port == config.Port && l.Status == StatusActive {
			return true
		}
	}
	return false
}

// CleanupInactive removes listeners that have been stopped for longer than the specified duration
func (m *ListenerManager) CleanupInactive(threshold time.Duration) {
	m.mu.Lock()
	defer m.mu.Unlock()

	now := time.Now()
	for id, listener := range m.listeners {
		if listener.Status == StatusStopped && !listener.StopTime.IsZero() {
			if now.Sub(listener.StopTime) > threshold {
				delete(m.listeners, id)
			}
		}
	}
}
