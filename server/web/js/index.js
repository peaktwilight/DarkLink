class DashboardManager {
    constructor() {
        this.autoScroll = true;
        this.previousListenerStates = new Map();
        this.logWebSocket = null;
        this.wsReconnectAttempts = 0;
        this.MAX_RECONNECT_ATTEMPTS = 10;
        this.RECONNECT_DELAY = 2000;
        this.reconnectTimer = null;
        
        this.initializeWebSocket();
        this.setupEventListeners();
        
        // Add periodic refresh for listeners
        this.loadActiveListeners();
        setInterval(() => this.loadActiveListeners(), 10000);
    }

    initializeWebSocket() {
        if (this.wsReconnectAttempts >= this.MAX_RECONNECT_ATTEMPTS) {
            this.appendLogEntry({
                timestamp: new Date().toISOString(),
                severity: 'ERROR',
                message: 'Maximum WebSocket reconnection attempts reached. Please refresh the page.',
                source: 'system'
            });
            return;
        }

        const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${wsProtocol}//${window.location.host}/ws/logs`;
        
        if (this.logWebSocket) {
            this.logWebSocket.close();
        }

        this.logWebSocket = new WebSocket(wsUrl);
        
        // Set a timeout for the initial connection
        const connectionTimeout = setTimeout(() => {
            if (this.logWebSocket.readyState !== WebSocket.OPEN) {
                this.logWebSocket.close();
                this.handleReconnect();
            }
        }, 5000);

        this.logWebSocket.onopen = () => {
            clearTimeout(connectionTimeout);
            this.wsReconnectAttempts = 0;
            this.appendLogEntry({
                timestamp: new Date().toISOString(),
                severity: 'INFO',
                message: 'Connected to server log stream',
                source: 'system'
            });

            // Set up ping interval
            const pingInterval = setInterval(() => {
                if (this.logWebSocket.readyState === WebSocket.OPEN) {
                    this.logWebSocket.send('ping');
                } else {
                    clearInterval(pingInterval);
                }
            }, 30000);

            // Clean up ping interval when socket closes
            this.logWebSocket.addEventListener('close', () => clearInterval(pingInterval));
        };

        this.logWebSocket.onmessage = (event) => {
            if (event.data === 'pong') return;
            
            try {
                const log = JSON.parse(event.data);
                this.appendLogEntry({
                    timestamp: log.timestamp,
                    severity: log.level.toUpperCase(),
                    message: log.message.trim(),
                    source: 'server'
                });
            } catch (error) {
                console.error('Error parsing log message:', error);
            }
        };

        this.logWebSocket.onclose = (event) => {
            clearTimeout(connectionTimeout);
            if (!event.wasClean) {
                this.handleReconnect();
            }
        };

        this.logWebSocket.onerror = (error) => {
            this.appendLogEntry({
                timestamp: new Date().toISOString(),
                severity: 'ERROR',
                message: 'WebSocket error occurred',
                source: 'system'
            });
        };
    }

    handleReconnect() {
        this.wsReconnectAttempts++;
        this.appendLogEntry({
            timestamp: new Date().toISOString(),
            severity: 'WARNING',
            message: `WebSocket connection closed. Attempt ${this.wsReconnectAttempts}/${this.MAX_RECONNECT_ATTEMPTS}. Retrying in ${this.RECONNECT_DELAY/1000}s...`,
            source: 'system'
        });

        // Clear any existing reconnect timer
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
        }

        // Set new reconnect timer
        this.reconnectTimer = setTimeout(() => {
            if (document.visibilityState === 'visible') {
                this.initializeWebSocket();
            }
        }, this.RECONNECT_DELAY);
    }

    setupEventListeners() {
        // Event viewer auto-scroll toggle
        document.getElementById('autoScrollBtn').addEventListener('click', () => this.toggleAutoScroll());

        // Command input handling
        document.getElementById('command-input').addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                this.sendCommand();
            }
        });

        // Visibility change handler
        document.addEventListener('visibilitychange', () => {
            if (document.visibilityState === 'visible' && 
                (!this.logWebSocket || this.logWebSocket.readyState === WebSocket.CLOSED)) {
                this.initializeWebSocket();
            }
        });
    }

    appendLogEntry(entry) {
        const eventLog = document.getElementById('event-log');
        const logEntry = document.createElement('div');
        logEntry.className = 'log-entry';
        
        logEntry.innerHTML = `
            <span class="message">${entry.message}</span>
        `;
        
        eventLog.appendChild(logEntry);
        
        if (this.autoScroll) {
            eventLog.scrollTop = eventLog.scrollHeight;
        }
    }

    clearEventLog() {
        document.getElementById('event-log').innerHTML = '';
    }

    toggleAutoScroll() {
        this.autoScroll = !this.autoScroll;
        const autoScrollBtn = document.getElementById('autoScrollBtn');
        autoScrollBtn.textContent = `Auto-scroll: ${this.autoScroll ? 'On' : 'Off'}`;
        
        if (this.autoScroll) {
            const eventLog = document.getElementById('event-log');
            eventLog.scrollTop = eventLog.scrollHeight;
        }
    }

    sendCommand() {
        const commandInput = document.getElementById('command-input');
        const command = commandInput.value.trim();
        
        if (command) {
            this.appendLogEntry({
                timestamp: new Date().toISOString(),
                severity: 'INFO',
                message: `> ${command}`,
                source: 'user'
            });

            if (this.logWebSocket && this.logWebSocket.readyState === WebSocket.OPEN) {
                this.logWebSocket.send(JSON.stringify({
                    type: 'command',
                    command: command
                }));
            } else {
                this.appendLogEntry({
                    timestamp: new Date().toISOString(),
                    severity: 'ERROR',
                    message: 'Not connected to server',
                    source: 'system'
                });
            }

            commandInput.value = '';
        }
    }
    
    // Fixed listener display based on actual data structure
    async loadActiveListeners() {
        try {
            const response = await fetch('/api/listeners/list');
            if (!response.ok) {
                throw new Error(`Server returned ${response.status}`);
            }
            
            const listeners = await response.json();
            const listenersContainer = document.getElementById('active-listeners');
            
            if (!Array.isArray(listeners) || listeners.length === 0) {
                listenersContainer.innerHTML = `
                    <div class="empty-state">
                        <p>No active listeners</p>
                        <p>Go to the Listeners page to create one</p>
                    </div>
                `;
                return;
            }
            
            let html = '';
            listeners.forEach(listener => {
                const config = listener.config || {};
                const listenerId = config.id || listener.id;
                const listenerName = config.name || listener.name || 'Unnamed';
                const listenerProtocol = config.protocol || listener.protocol || listener.type || 'Unknown';
                const listenerHost = config.host || listener.host || 'Unknown';
                const listenerPort = config.port || listener.port || 'Unknown';
                const listenerStatus = listener.status || 'Unknown';
                const listenerError = listener.error || '';
                
                if (!listenerId) {
                    console.warn('Listener missing ID:', listener);
                    return;
                }

                html += `
                    <div class="listener-card" data-id="${listenerId}">
                        <div class="listener-header">
                            <span class="listener-name">${listenerName}</span>
                            <span class="listener-type">${listenerProtocol}</span>
                        </div>
                        <div class="listener-details">
                            <div class="listener-id">ID: ${listenerId}</div>
                            <div>Host: ${listenerHost}:${listenerPort}</div>
                            <div>Status: <span class="status-${listenerStatus.toLowerCase()}">${listenerStatus}</span></div>
                            ${listenerError ? `<div class="error-message">Error: ${listenerError}</div>` : ''}
                        </div>
                        <div class="listener-actions">
                            ${listenerStatus.toLowerCase() === 'stopped' ? 
                              `<button class="action-button success" onclick="dashboardManager.startListener('${listenerId}')">Start</button>` : 
                              `<button class="action-button" onclick="dashboardManager.stopListener('${listenerId}')">Stop</button>`}
                            <button class="action-button delete" onclick="dashboardManager.deleteListener('${listenerId}', '${listenerName}')">Delete</button>
                        </div>
                    </div>
                `;
                
                // Track listener state changes for notifications
                if (listenerId && listenerName) {
                    const key = `${listenerId}-${listenerName}`;
                    const previousStatus = this.previousListenerStates.get(key);
                    
                    if (previousStatus && previousStatus !== listenerStatus) {
                        this.appendLogEntry({
                            timestamp: new Date().toISOString(),
                            severity: 'INFO',
                            message: `Listener "${listenerName}" changed status from ${previousStatus} to ${listenerStatus}`,
                            source: 'system'
                        });
                    }
                    
                    this.previousListenerStates.set(key, listenerStatus);
                }
            });
            
            listenersContainer.innerHTML = html;
            
        } catch (error) {
            console.error('Error loading listeners:', error);
            document.getElementById('active-listeners').innerHTML = `
                <div class="empty-state">
                    <p>Error loading listeners</p>
                    <p>${error.message}</p>
                </div>
            `;
        }
    }

    async startListener(id) {
        try {
            const response = await fetch(`/api/listeners/${id}/start`, { 
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                }
            });
            
            if (!response.ok) {
                if (response.status === 405) {
                    return await this.handleStartListenerFallback(id);
                }
                const responseText = await response.text();
                let errorMsg;
                try {
                    const result = JSON.parse(responseText);
                    errorMsg = result.error || `Failed to start listener (${response.status})`;
                } catch {
                    errorMsg = responseText || `Failed to start listener (${response.status})`;
                }
                throw new Error(errorMsg);
            }
            
            await this.loadActiveListeners();
        } catch (error) {
            console.error('Error starting listener:', error);
            this.appendLogEntry({
                timestamp: new Date().toISOString(),
                severity: 'ERROR',
                message: `Failed to start listener: ${error.message}`,
                source: 'system'
            });
        }
    }

    async stopListener(id) {
        try {
            const response = await fetch(`/api/listeners/${id}/stop`, { 
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                }
            });
            
            if (!response.ok) {
                const responseText = await response.text();
                let errorMsg;
                try {
                    const result = JSON.parse(responseText);
                    errorMsg = result.error || `Failed to stop listener (${response.status})`;
                } catch {
                    errorMsg = responseText || `Failed to stop listener (${response.status})`;
                }
                throw new Error(errorMsg);
            }
            
            await this.loadActiveListeners();
        } catch (error) {
            console.error('Error stopping listener:', error);
            this.appendLogEntry({
                timestamp: new Date().toISOString(),
                severity: 'ERROR',
                message: `Failed to stop listener: ${error.message}`,
                source: 'system'
            });
        }
    }

    async deleteListener(id, name) {
        if (!confirm(`Are you sure you want to delete ${name}?`)) return;
        
        try {
            const response = await fetch(`/api/listeners/${id}`, { 
                method: 'DELETE',
                headers: {
                    'Content-Type': 'application/json'
                }
            });
            
            if (!response.ok) {
                const responseText = await response.text();
                let errorMsg;
                try {
                    const result = JSON.parse(responseText);
                    errorMsg = result.error || `Failed to delete listener (${response.status})`;
                } catch {
                    errorMsg = responseText || `Failed to delete listener (${response.status})`;
                }
                throw new Error(errorMsg);
            }
            
            await this.loadActiveListeners();
        } catch (error) {
            console.error('Error deleting listener:', error);
            this.appendLogEntry({
                timestamp: new Date().toISOString(),
                severity: 'ERROR',
                message: `Failed to delete listener: ${error.message}`,
                source: 'system'
            });
        }
    }

    async handleStartListenerFallback(id) {
        try {
            const response = await fetch(`/api/listeners/${id}`);
            if (!response.ok) {
                throw new Error(`Failed to get listener details (${response.status})`);
            }
            
            const listener = await response.json();
            const config = listener.config || listener;
            const newConfig = {...config};
            delete newConfig.id;
            
            // Delete the old listener
            const deleteResponse = await fetch(`/api/listeners/${id}`, {
                method: 'DELETE'
            });
            
            if (!deleteResponse.ok) {
                throw new Error(`Failed to delete old listener (${deleteResponse.status})`);
            }
            
            // Create a new listener with the same config
            const createResponse = await fetch('/api/listeners/create', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(newConfig)
            });
            
            if (!createResponse.ok) {
                throw new Error(`Failed to recreate listener (${createResponse.status})`);
            }
            
            await this.loadActiveListeners();
        } catch (error) {
            console.error("Error in fallback listener start:", error);
            this.appendLogEntry({
                timestamp: new Date().toISOString(),
                severity: 'ERROR',
                message: `Failed to start listener (fallback method): ${error.message}`,
                source: 'system'
            });
        }
    }
}

// Initialize the dashboard manager when the page loads
let dashboardManager;
document.addEventListener('DOMContentLoaded', () => {
    dashboardManager = new DashboardManager();
});