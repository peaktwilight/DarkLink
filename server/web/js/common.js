// Utility Functions
function formatBytes(bytes) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function escapeHtml(unsafe) {
    return unsafe
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
}

// Status message handling
function showError(message, elementId = 'status-message') {
    const statusMessage = document.getElementById(elementId);
    if (statusMessage) {
        statusMessage.textContent = `Error: ${message}`;
        statusMessage.className = 'status-message error';
    }
}

function showSuccess(message, elementId = 'status-message', duration = 3000) {
    const statusMessage = document.getElementById(elementId);
    if (statusMessage) {
        statusMessage.textContent = message;
        statusMessage.className = 'status-message success';
        setTimeout(() => {
            statusMessage.className = 'status-message hidden';
        }, duration);
    }
}

function showLoading(message, elementId = 'status-message') {
    const statusMessage = document.getElementById(elementId);
    if (statusMessage) {
        statusMessage.textContent = message;
        statusMessage.className = 'status-message loading';
    }
}

// WebSocket connection management
function createWebSocket(url, options = {}) {
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${wsProtocol}//${window.location.host}${url}`;
    const ws = new WebSocket(wsUrl);

    if (options.onOpen) ws.onopen = options.onOpen;
    if (options.onMessage) ws.onmessage = options.onMessage;
    if (options.onClose) ws.onclose = options.onClose;
    if (options.onError) ws.onerror = options.onError;

    return ws;
}