package payload

import (
	"fmt"
	"microc2/server/internal/protocols"
)

// ListenerManagerAdapter adapts the listener manager to the ListenerGetter interface
type ListenerManagerAdapter struct {
	manager *protocols.ListenerManager
}

// NewListenerManagerAdapter creates a new adapter for the listener manager
func NewListenerManagerAdapter(manager *protocols.ListenerManager) *ListenerManagerAdapter {
	return &ListenerManagerAdapter{
		manager: manager,
	}
}

// GetListener retrieves a listener by its ID and converts it to the simplified Listener type
func (a *ListenerManagerAdapter) GetListener(id string) (Listener, error) {
	protocolListener, err := a.manager.GetListener(id)
	if err != nil {
		return Listener{}, fmt.Errorf("listener not found: %w", err)
	}

	return Listener{
		ID:       protocolListener.Config.ID,
		Name:     protocolListener.Config.Name,
		Protocol: protocolListener.Config.Protocol,
		Host:     protocolListener.Config.BindHost,
		Port:     protocolListener.Config.Port,
	}, nil
}
