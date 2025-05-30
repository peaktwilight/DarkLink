function safeParseJson(text) {
    try { return JSON.parse(text); }
    catch (e) { console.error('JSON parse error:', e); return null; }
}

function parseHeaderLines(raw) {
    return raw.split('\n')
      .map(l => l.trim())
      .filter(l => l)
      .reduce((obj, line) => {
          const [key, ...vals] = line.split(':');
          obj[key.trim()] = vals.join(':').trim();
          return obj;
      }, {});
}

class ListenerManager {
    constructor() {
        this.setupEventListeners();
        this.startAutoRefresh();
        this.fetchListenerList();
    }

    setupEventListeners() {
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
        
        
        const listenerConfig = {
            name: formValues.listenerName,
            protocol: formValues.payloadType,
            host: formValues.bindHost,
            port: parseInt(formValues.port, 10)
        };

        // Basic validation
        if (!formValues.listenerName) {
            this.showError('Listener name is required');
            return;
        }

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
            const parsed = parseHeaderLines(formValues.headers);
            if (Object.keys(parsed).length) listenerConfig.headers = parsed;
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
                    const errorJson = safeParseJson(responseText);
                    if (errorJson && errorJson.error) errorMessage = errorJson.error;
                } catch {
                    errorMessage = responseText || errorMessage;
                }
                throw new Error(errorMessage);
            }

            this.showSuccess('Listener created successfully');
            e.target.reset();
            this.clearAllLists();
            await this.fetchListenerList();
        } catch (error) {
            console.error('Error creating listener:', error);
            this.showError(error.message);
        }
    }

    async fetchListenerList() {
        const listenersList = document.getElementById('active-listeners');
        if (!listenersList) {
            console.warn("Element 'active-listeners' not found in DOM");
            return;
        }
        // show loading state
        listenersList.textContent = '';
        const spinner = document.createElement('div'); spinner.className = 'loading-spinner'; spinner.textContent = 'Loading...';
        listenersList.appendChild(spinner);
        try {
            const response = await fetch('/api/listeners/list');
            if (!response.ok) throw new Error(`HTTP ${response.status}`);
            const listeners = await response.json();
            // if empty
            if (!Array.isArray(listeners) || listeners.length === 0) {
                listenersList.textContent = '';
                const empty = document.createElement('div'); empty.className='empty-state'; empty.textContent='No active listeners';
                listenersList.appendChild(empty);
                return;
            }
            // rebuild list via fragment
            const frag = document.createDocumentFragment();
            listeners.forEach(listener => {
                const config = listener.config||{};
                const id = config.id||listener.id; if(!id) return;
                const card = document.createElement('div'); card.className='listener-card'; card.dataset.id=id;
                // header
                const header = document.createElement('div'); header.className='listener-header';
                header.innerHTML = `<span class="listener-name">${config.name||listener.name||'Unnamed'}</span>`+
                    `<span class="listener-type">${config.protocol||listener.Protocol||listener.type||'?'}</span>`;
                card.appendChild(header);
                // details
                const details = document.createElement('div'); details.className='listener-details';
                details.innerHTML = `<div class="listener-id">ID: ${id}</div>`+
                    `<div>Host: ${config.host||listener.host||'?'}:${config.port||listener.port||'?'}</div>`+
                    `<div>Status: <span class="status-${(listener.status||'').toLowerCase()}">${listener.status||'?'}</span></div>`+
                    `${listener.error?`<div class="error-message">Error: ${listener.error}</div>`:''}`;
                card.appendChild(details);
                // actions
                const actions = document.createElement('div'); actions.className='listener-actions';
                const btn = document.createElement('button');
                if((listener.status||'').toLowerCase()==='stopped'){ btn.className='action-button success'; btn.textContent='Start'; btn.onclick=()=>listenerManager.startListener(id);
                } else { btn.className='action-button'; btn.textContent='Stop'; btn.onclick=()=>listenerManager.stopListener(id);}                
                const del = document.createElement('button'); del.className='action-button delete'; del.textContent='Delete'; del.onclick=()=>listenerManager.deleteListener(id, config.name||listener.name);
                actions.append(btn, del);
                card.appendChild(actions);
                frag.appendChild(card);
            });
            listenersList.textContent = '';
            listenersList.appendChild(frag);
        } catch (error) {
            console.error('Error fetching listeners:', error);
            listenersList.textContent = '';
            const err = document.createElement('div'); err.className='error-state'; err.textContent=`Error loading listeners: ${error.message}`;
            listenersList.appendChild(err);
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
                    const result = safeParseJson(responseText);
                    errorMsg = result.error || `Failed to stop listener (${response.status})`;
                } catch {
                    errorMsg = responseText || `Failed to stop listener (${response.status})`;
                }
                throw new Error(errorMsg);
            }
            
            statusMessage.textContent = "Listener stopped successfully";
            statusMessage.className = "status-message success";
            await this.fetchListenerList();
            
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
                    const result = safeParseJson(responseText);
                    errorMsg = result.error || `Failed to delete listener (${response.status})`;
                } catch {
                    errorMsg = responseText || `Failed to delete listener (${response.status})`;
                }
                throw new Error(errorMsg);
            }
            
            statusMessage.textContent = `Listener "${name}" deleted successfully`;
            statusMessage.className = "status-message success";
            await this.fetchListenerList();
            
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
                    const result = safeParseJson(responseText);
                    errorMsg = result.error || `Failed to start listener (${response.status})`;
                } catch {
                    errorMsg = responseText || `Failed to start listener (${response.status})`;
                }
                throw new Error(errorMsg);
            }
            
            statusMessage.textContent = "Listener started successfully";
            statusMessage.className = "status-message success";
            await this.fetchListenerList();
            
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
            await this.fetchListenerList();
            
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
        // Server-Sent Events for live updates
        if (window.EventSource) {
            const es = new EventSource('/api/listeners/stream');
            es.onmessage = () => this.fetchListenerList();
            es.onerror = () => console.warn('Listeners SSE error, falling back to polling');
        } else {
            // fallback polling
            const interval = 30000;
            let timer = setInterval(() => this.refreshListenersList(), interval);
            document.addEventListener('visibilitychange', () => {
                if (document.hidden) clearInterval(timer);
                else { this.refreshListenersList(); timer = setInterval(() => this.refreshListenersList(), interval); }
            });
        }
    }

    refreshListenersList() {
        const listenersList = document.getElementById('active-listeners');
        if (!listenersList) {
            console.warn("Cannot refresh listeners: Element 'active-listeners' not found");
            return false;
        }
        
        return this.fetchListenerList().catch(error => {
            console.error("Error refreshing listeners list:", error);
            return false;
        });
    }
}

let listenerManager;
document.addEventListener('DOMContentLoaded', () => {
    listenerManager = new ListenerManager();
});