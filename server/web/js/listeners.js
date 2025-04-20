class ListenerManager {
    constructor() {
        this.setupEventListeners();
        this.startAutoRefresh();
        this.updateListenersList();
    }

    setupEventListeners() {
        // Enable/disable proxy settings
        document.getElementById('enableProxy').addEventListener('change', function() {
            const proxySettings = document.getElementById('proxySettings');
            proxySettings.classList.toggle('hidden', !this.checked);
        });

        // Input events to allow pressing Enter to add items
        document.getElementById('hostInput').addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                this.addListItem('hostInput', 'hostsList', 'hosts');
            }
        });

        document.getElementById('headerInput').addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                this.addListItem('headerInput', 'headersList', 'headers');
            }
        });
        
        document.getElementById('uriInput').addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                this.addListItem('uriInput', 'urisList', 'uris');
            }
        });

        // Form submission
        document.getElementById('listenerForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            await this.handleFormSubmit(e);
        });
    }

    addListItem(inputId, listId, hiddenFieldId) {
        const input = document.getElementById(inputId);
        const list = document.getElementById(listId);
        const hiddenField = document.getElementById(hiddenFieldId);
        if (!input.value.trim()) return;
        
        // Create list item element
        const item = document.createElement('div');
        item.className = 'list-item';
        const itemText = document.createElement('span');
        itemText.className = 'item-text';
        itemText.textContent = input.value.trim();
        item.appendChild(itemText);
        
        const removeBtn = document.createElement('button');
        removeBtn.className = 'remove-item';
        removeBtn.innerHTML = '×';
        removeBtn.type = 'button';
        removeBtn.onclick = () => {
            list.removeChild(item);
            this.updateHiddenField(listId, hiddenFieldId);
        };
        item.appendChild(removeBtn);
        
        list.appendChild(item);
        input.value = '';
        
        this.updateHiddenField(listId, hiddenFieldId);
    }

    updateHiddenField(listId, hiddenFieldId) {
        const list = document.getElementById(listId);
        const hiddenField = document.getElementById(hiddenFieldId);
        const items = Array.from(list.querySelectorAll('.item-text')).map(span => span.textContent);
        hiddenField.value = items.join('\n');
    }
    
    clearList(listId, hiddenFieldId) {
        const list = document.getElementById(listId);
        const hiddenField = document.getElementById(hiddenFieldId);
        list.innerHTML = '';
        hiddenField.value = '';
    }

    clearAllLists() {
        this.clearList('hostsList', 'hosts');
        this.clearList('headersList', 'headers');
        this.clearList('urisList', 'uris');
    }

    async handleFormSubmit(e) {
        const formData = new FormData(e.target);
        const formValues = Object.fromEntries(formData);
        
        // Basic validation
        if (!formValues.listenerName) {
            this.showError('Listener name is required');
            return;
        }
        
        const listenerConfig = {
            name: formValues.listenerName,
            protocol: formValues.payloadType,
            host: formValues.bindHost,
            port: parseInt(formValues.port, 10)
        };

        // Add arrays if they have non-empty values
        if (formValues.hosts) {
            const hosts = formValues.hosts.split('\n')
                .map(h => h.trim())
                .filter(h => h.length > 0);
            if (hosts.length > 0) {
                listenerConfig.hosts = hosts;
            }
        }

        if (formValues.hostRotation) {
            listenerConfig.host_rotation = formValues.hostRotation;
        }

        if (formValues.userAgent) {
            listenerConfig.user_agent = formValues.userAgent.trim();
        }

        // Parse and add headers if they exist
        if (formValues.headers) {
            try {
                const headerPairs = formValues.headers.split('\n')
                    .map(h => h.trim())
                    .filter(h => h.length > 0)
                    .map(h => {
                        const [key, ...values] = h.split(':');
                        return [key.trim(), values.join(':').trim()];
                    });
                
                if (headerPairs.length > 0) {
                    listenerConfig.headers = Object.fromEntries(headerPairs);
                }
            } catch (error) {
                this.showError('Invalid header format');
                return;
            }
        }

        // Parse and add URIs if they exist
        if (formValues.uris) {
            const uris = formValues.uris.split('\n')
                .map(u => u.trim())
                .filter(u => u.length > 0);
            if (uris.length > 0) {
                listenerConfig.uris = uris;
            }
        }

        if (formValues.hostHeader) {
            listenerConfig.host_header = formValues.hostHeader.trim();
        }

        // Add proxy configuration if enabled
        if (formValues.enableProxy === "on") {
            if (!formValues.proxyHost || !formValues.proxyPort) {
                this.showError('Proxy host and port are required when proxy is enabled');
                return;
            }
            
            listenerConfig.proxy = {
                type: formValues.proxyType,
                host: formValues.proxyHost.trim(),
                port: parseInt(formValues.proxyPort, 10)
            };

            if (formValues.proxyUsername) {
                listenerConfig.proxy.username = formValues.proxyUsername.trim();
            }
            if (formValues.proxyPassword) {
                listenerConfig.proxy.password = formValues.proxyPassword.trim();
            }
        }

        console.log('Sending listener config:', listenerConfig);
        this.showLoading('Creating listener...');

        try {
            const response = await fetch('/api/listeners/create', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(listenerConfig)
            });

            const responseText = await response.text();
            let errorMessage = 'Failed to create listener';
            
            if (!response.ok) {
                try {
                    const errorJson = JSON.parse(responseText);
                    errorMessage = errorJson.error || errorMessage;
                } catch {
                    errorMessage = responseText || errorMessage;
                }
                throw new Error(errorMessage);
            }

            this.showSuccess('Listener created successfully');
            e.target.reset();
            this.clearAllLists();
            await this.updateListenersList();
        } catch (error) {
            console.error('Error creating listener:', error);
            this.showError(error.message);
        }
    }

    async updateListenersList() {
        const listenersList = document.getElementById('active-listeners');
        
        if (!listenersList) {
            console.warn("Element 'active-listeners' not found in DOM");
            return;
        }
        
        listenersList.innerHTML = '<div class="loading-spinner">Loading...</div>';
        try {
            const response = await fetch('/api/listeners/list');
            
            if (!response.ok) {
                throw new Error(`HTTP error ${response.status}`);
            }
            
            const listeners = await response.json();
            console.log("Received listeners:", listeners);
            
            if (listeners.length === 0) {
                listenersList.innerHTML = '<div class="empty-state">No active listeners</div>';
                return;
            }
            
            let listenersHTML = '';
            for (const listener of listeners) {
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
                    continue;
                }

                listenersHTML += `
                    <div class="listener-card" data-id="${listenerId}">
                        <div class="listener-header">
                            <span class="listener-name">${listenerName}</span>
                            <span class="listener-type">${listenerProtocol}</span>
                        </div>
                        <div class="listener-details">
                            <div>Host: ${listenerHost}:${listenerPort}</div>
                            <div>Status: <span class="status-${listenerStatus.toLowerCase()}">${listenerStatus}</span></div>
                            ${listenerError ? `<div class="error-message">Error: ${listenerError}</div>` : ''}
                        </div>
                        <div class="listener-actions">
                            ${listenerStatus.toLowerCase() === 'stopped' ? 
                              `<button class="action-button success" onclick="listenerManager.startListener('${listenerId}')">Start</button>` : 
                              `<button class="action-button" onclick="listenerManager.stopListener('${listenerId}')">Stop</button>`}
                            <button class="action-button delete" onclick="listenerManager.deleteListener('${listenerId}', '${listenerName}')">Delete</button>
                        </div>
                    </div>
                `;
            }
            
            listenersList.innerHTML = listenersHTML || '<div class="empty-state">No active listeners</div>';
        } catch (error) {
            console.error('Error fetching listeners:', error);
            listenersList.innerHTML = `<div class="error-state">Error loading listeners: ${error.message}</div>`;
        }
    }

    async stopListener(id) {
        if (!id || id === 'undefined') {
            this.showError('Invalid listener ID');
            console.error('Attempted to stop a listener with invalid ID:', id);
            return;
        }

        console.log(`Stopping listener with ID: ${id}`);
        const statusMessage = document.getElementById('status-message');
        statusMessage.textContent = "Stopping listener...";
        statusMessage.className = "status-message loading";
        
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
            
            statusMessage.textContent = "Listener stopped successfully";
            statusMessage.className = "status-message success";
            await this.updateListenersList();
            
            setTimeout(() => {
                statusMessage.className = "status-message hidden";
            }, 3000);
        } catch (error) {
            console.error('Error stopping listener:', error);
            statusMessage.textContent = `Error: ${error.message}`;
            statusMessage.className = "status-message error";
        }
    }

    async deleteListener(id, name) {
        if (!id || id === 'undefined') {
            this.showError('Invalid listener ID');
            console.error('Attempted to delete a listener with invalid ID:', id);
            return;
        }

        if (!confirm(`Are you sure you want to delete ${name}?`)) return;
        
        const statusMessage = document.getElementById('status-message');
        statusMessage.textContent = "Deleting listener...";
        statusMessage.className = "status-message loading";
        
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
            
            statusMessage.textContent = `Listener "${name}" deleted successfully`;
            statusMessage.className = "status-message success";
            await this.updateListenersList();
            
            setTimeout(() => {
                statusMessage.className = "status-message hidden";
            }, 3000);
        } catch (error) {
            console.error('Error deleting listener:', error);
            statusMessage.textContent = `Error: ${error.message}`;
            statusMessage.className = "status-message error";
        }
    }

    async startListener(id) {
        if (!id || id === 'undefined') {
            this.showError('Invalid listener ID');
            console.error('Attempted to start a listener with invalid ID:', id);
            return;
        }

        console.log(`Starting listener with ID: ${id}`);
        const statusMessage = document.getElementById('status-message');
        statusMessage.textContent = "Starting listener...";
        statusMessage.className = "status-message loading";
        
        try {
            const response = await fetch(`/api/listeners/${id}/start`, { 
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                }
            });
            
            if (!response.ok) {
                // Special handling for Method Not Allowed - try the fallback method
                if (response.status === 405) {
                    console.log("Start endpoint not available, trying fallback method");
                    await this.handleStartListenerFallback(id);
                    return;
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
            
            statusMessage.textContent = "Listener started successfully";
            statusMessage.className = "status-message success";
            await this.updateListenersList();
            
            setTimeout(() => {
                statusMessage.className = "status-message hidden";
            }, 3000);
        } catch (error) {
            console.error('Error starting listener:', error);
            statusMessage.textContent = `Error: ${error.message}`;
            statusMessage.className = "status-message error";
        }
    }

    async handleStartListenerFallback(id) {
        const statusMessage = document.getElementById('status-message');
        
        try {
            // First get the listener configuration
            const response = await fetch(`/api/listeners/${id}`);
            if (!response.ok) {
                throw new Error(`Failed to get listener details (${response.status})`);
            }
            
            const listener = await response.json();
            console.log("Retrieved listener for recreation:", listener);
            
            // Extract the config from the listener
            const config = listener.config || listener;
            
            // Remove the ID as we're creating a new one
            const newConfig = {...config};
            delete newConfig.id;
            
            statusMessage.textContent = "Recreating listener...";
            
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
            
            statusMessage.textContent = "Listener started successfully (recreated)";
            statusMessage.className = "status-message success";
            await this.updateListenersList();
            
            setTimeout(() => {
                statusMessage.className = "status-message hidden";
            }, 3000);
            
        } catch (error) {
            console.error("Error in fallback listener start:", error);
            statusMessage.textContent = `Error: ${error.message}`;
            statusMessage.className = "status-message error";
        }
    }

    showError(message) {
        const statusMessage = document.getElementById('status-message');
        statusMessage.textContent = `Error: ${message}`;
        statusMessage.className = 'status-message error';
    }

    showSuccess(message) {
        const statusMessage = document.getElementById('status-message');
        statusMessage.textContent = message;
        statusMessage.className = 'status-message success';
        setTimeout(() => {
            statusMessage.className = 'status-message hidden';
        }, 3000);
    }

    showLoading(message) {
        const statusMessage = document.getElementById('status-message');
        statusMessage.textContent = message;
        statusMessage.className = 'status-message loading';
    }

    startAutoRefresh() {
        const AUTO_REFRESH_INTERVAL = 30000; // 30 seconds
        let refreshTimer = setInterval(() => this.refreshListenersList(), AUTO_REFRESH_INTERVAL);
        
        // Clear interval when page is hidden to save resources
        document.addEventListener('visibilitychange', () => {
            if (document.hidden) {
                clearInterval(refreshTimer);
            } else {
                // Refresh immediately when page becomes visible
                this.refreshListenersList();
                // Restart the timer
                refreshTimer = setInterval(() => this.refreshListenersList(), AUTO_REFRESH_INTERVAL);
            }
        });
    }

    refreshListenersList() {
        const listenersList = document.getElementById('active-listeners');
        if (!listenersList) {
            console.warn("Cannot refresh listeners: Element 'active-listeners' not found");
            return false;
        }
        
        return this.updateListenersList().catch(error => {
            console.error("Error refreshing listeners list:", error);
            return false;
        });
    }
}

// Initialize the listener manager when the page loads
let listenerManager;
document.addEventListener('DOMContentLoaded', () => {
    listenerManager = new ListenerManager();
});