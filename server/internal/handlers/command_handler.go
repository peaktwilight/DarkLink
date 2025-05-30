package handlers

import (
	"errors"
	"fmt"
	"sync"
	"time"

	"github.com/google/uuid"
)

// CommandStatus represents the current state of a command
type CommandStatus string

const (
	StatusQueued    CommandStatus = "QUEUED"
	StatusSent      CommandStatus = "SENT"
	StatusCompleted CommandStatus = "COMPLETED"
	StatusFailed    CommandStatus = "FAILED"
)

// Command represents a command to be executed by an agent
type Command struct {
	ID        string        `json:"id"`
	Command   string        `json:"command"`
	AgentID   string        `json:"agent_id"`
	Status    CommandStatus `json:"status"`
	Output    string        `json:"output,omitempty"`
	Error     string        `json:"error,omitempty"`
	QueueTime time.Time     `json:"queue_time"`
	SentTime  time.Time     `json:"sent_time,omitempty"`
	DoneTime  time.Time     `json:"done_time,omitempty"`
}

// CommandQueue manages command execution for a listener
type CommandQueue struct {
	commands map[string]*Command
	queue    []string // IDs of queued commands
	mu       sync.RWMutex
}

// NewCommandQueue creates a new command queue
func NewCommandQueue() *CommandQueue {
	return &CommandQueue{
		commands: make(map[string]*Command),
		queue:    make([]string, 0),
	}
}

// QueueCommand adds a new command to the queue
func (q *CommandQueue) QueueCommand(AgentID, cmdStr string) (*Command, error) {
	if cmdStr == "" {
		return nil, errors.New("empty command")
	}

	cmd := &Command{
		ID:        uuid.New().String(),
		Command:   cmdStr,
		AgentID:   AgentID,
		Status:    StatusQueued,
		QueueTime: time.Now(),
	}

	q.mu.Lock()
	defer q.mu.Unlock()

	q.commands[cmd.ID] = cmd
	q.queue = append(q.queue, cmd.ID)

	return cmd, nil
}

// GetNextCommand retrieves the next command for an agent
func (q *CommandQueue) GetNextCommand(AgentID string) (*Command, error) {
	q.mu.Lock()
	defer q.mu.Unlock()

	// Find first command for this agent
	for i, id := range q.queue {
		cmd := q.commands[id]
		if cmd.AgentID == AgentID && cmd.Status == StatusQueued {
			// Remove from queue and mark as sent
			q.queue = append(q.queue[:i], q.queue[i+1:]...)
			cmd.Status = StatusSent
			cmd.SentTime = time.Now()
			return cmd, nil
		}
	}

	return nil, nil // No commands available
}

// UpdateCommandStatus updates the status of a command
func (q *CommandQueue) UpdateCommandStatus(cmdID string, status CommandStatus, output string, err error) error {
	q.mu.Lock()
	defer q.mu.Unlock()

	cmd, exists := q.commands[cmdID]
	if !exists {
		return fmt.Errorf("command %s not found", cmdID)
	}

	cmd.Status = status
	if output != "" {
		cmd.Output = output
	}
	if err != nil {
		cmd.Error = err.Error()
	}
	if status == StatusCompleted || status == StatusFailed {
		cmd.DoneTime = time.Now()
	}

	return nil
}

// GetCommand retrieves a command by ID
func (q *CommandQueue) GetCommand(cmdID string) (*Command, error) {
	q.mu.RLock()
	defer q.mu.RUnlock()

	cmd, exists := q.commands[cmdID]
	if !exists {
		return nil, fmt.Errorf("command %s not found", cmdID)
	}

	return cmd, nil
}

// ListCommands returns all commands, optionally filtered by status
func (q *CommandQueue) ListCommands(status CommandStatus) []*Command {
	q.mu.RLock()
	defer q.mu.RUnlock()

	var commands []*Command
	for _, cmd := range q.commands {
		if status == "" || cmd.Status == status {
			commands = append(commands, cmd)
		}
	}

	return commands
}

// CleanupOldCommands removes completed commands older than the specified duration
func (q *CommandQueue) CleanupOldCommands(age time.Duration) {
	q.mu.Lock()
	defer q.mu.Unlock()

	now := time.Now()
	for id, cmd := range q.commands {
		if (cmd.Status == StatusCompleted || cmd.Status == StatusFailed) &&
			now.Sub(cmd.DoneTime) > age {
			delete(q.commands, id)
		}
	}

	// Clean up queue
	newQueue := make([]string, 0, len(q.queue))
	for _, id := range q.queue {
		if _, exists := q.commands[id]; exists {
			newQueue = append(newQueue, id)
		}
	}
	q.queue = newQueue
}
