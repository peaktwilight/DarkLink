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
                // The data we need is nested in the config object
                const config = listener.config || {};
                
                const name = config.name || 'Unnamed';
                const protocol = config.protocol || 'Unknown';
                const hostInfo = (config.host && config.port) ? 
                    `Host: ${config.host}:${config.port}` : '';
                const status = listener.status || 'Unknown';
                const id = config.id || '';
                
                const statusClass = status === 'ACTIVE' ? 'status-active' : 'status-inactive';
                
                html += `
                    <div class="listener-card">
                        <div class="listener-header">
                            <div class="listener-name">${name}</div>
                            <div class="listener-protocol">${protocol}</div>
                        </div>
                        <div class="listener-details">
                            ${hostInfo ? `<div>${hostInfo}</div>` : ''}
                            <div>Status: <span class="${statusClass}">${status}</span></div>
                            ${id ? `<div>ID: ${id.substring(0, 8)}...</div>` : ''}
                        </div>
                    </div>
                `;
                
                // Track listener state changes for notifications
                if (id && name) {
                    const key = `${id}-${name}`;
                    const previousStatus = this.previousListenerStates.get(key);
                    
                    if (previousStatus && previousStatus !== status) {
                        this.appendLogEntry({
                            timestamp: new Date().toISOString(),
                            severity: 'INFO',
                            message: `Listener "${name}" changed status from ${previousStatus} to ${status}`,
                            source: 'system'
                        });
                    }
                    
                    this.previousListenerStates.set(key, status);
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
}

// Initialize the dashboard manager when the page loads
let dashboardManager;
document.addEventListener('DOMContentLoaded', () => {
    dashboardManager = new DashboardManager();
});