class TerminalManager {
    constructor() {
        this.commandHistory = [];
        this.historyIndex = -1;
        this.currentWorkingDirectory = '~';
        this.terminalOutput = document.getElementById('terminal-output');
        this.terminalInput = document.getElementById('terminal-input');
        this.setupWebSocket();
        this.setupEventListeners();
        this.focusInput();
    }

    setupWebSocket() {
        const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        this.ws = new WebSocket(`${wsProtocol}//${window.location.host}/ws/terminal`);

        this.ws.onmessage = (event) => {
            let response;
            try {
                response = JSON.parse(event.data);
            } catch (e) {
                console.error('Invalid JSON from terminal websocket:', e);
                return;
            }
            if (response.cwd) {
                this.currentWorkingDirectory = response.cwd;
                this.updatePrompt();
            }
            if (response.output) {
                this.appendToTerminal(response.output, response.error);
            }
        };

        this.ws.onclose = () => {
            this.appendToTerminal('Connection to server closed. Attempting to reconnect...', true);
            setTimeout(() => this.setupWebSocket(), 5000);
        };
    }

    setupEventListeners() {
        this.terminalInput.addEventListener('keydown', (e) => this.handleKeyDown(e));

        // Focus input when clicking anywhere in terminal
        document.querySelector('.terminal-container').addEventListener('click', (e) => {
            if (window.getSelection().toString() === '') {
                this.terminalInput.focus();
            }
        });
    }

    handleKeyDown(e) {
        switch (e.key) {
            case 'Enter':
                this.handleEnterKey();
                break;
            case 'ArrowUp':
                this.handleArrowUp(e);
                break;
            case 'ArrowDown':
                this.handleArrowDown(e);
                break;
            case 'Tab':
                this.handleTab(e);
                break;
        }
    }

    handleEnterKey() {
        const command = this.terminalInput.value.trim();
        if (command) {
            this.appendCommand(command);
            this.ws.send(command);
            this.commandHistory.push(command);
            this.historyIndex = this.commandHistory.length;
            this.terminalInput.value = '';
        }
    }

    handleArrowUp(e) {
        e.preventDefault();
        if (this.historyIndex > 0) {
            this.historyIndex--;
            this.terminalInput.value = this.commandHistory[this.historyIndex];
            // Move cursor to end of input
            setTimeout(() => this.terminalInput.selectionStart = this.terminalInput.selectionEnd = this.terminalInput.value.length, 0);
        }
    }

    handleArrowDown(e) {
        e.preventDefault();
        if (this.historyIndex < this.commandHistory.length - 1) {
            this.historyIndex++;
            this.terminalInput.value = this.commandHistory[this.historyIndex];
        } else {
            this.historyIndex = this.commandHistory.length;
            this.terminalInput.value = '';
        }
    }

    handleTab(e) {
        e.preventDefault();
        // Send tab completion request
        this.ws.send(JSON.stringify({
            type: 'tab_completion',
            partial: this.terminalInput.value
        }));
    }

    updatePrompt() {
        const pathElement = document.querySelector('.prompt .path');
        if (pathElement) {
            pathElement.textContent = this.currentWorkingDirectory;
        }
    }

    appendToTerminal(text, isError = false) {
        if (text.trim()) {
            const outputDiv = document.createElement('div');
            outputDiv.className = isError ? 'command-output error-output' : 'command-output';
            outputDiv.textContent = text;
            this.terminalOutput.appendChild(outputDiv);
        }
        this.scrollToBottom();
    }

    appendCommand(command) {
        const cmdLine = document.createElement('div');
        cmdLine.className = 'command-line';
        cmdLine.innerHTML = `<span class="prompt"><span class="user">user@server</span>:<span class="path">${this.currentWorkingDirectory}</span>$</span> ${command}`;
        this.terminalOutput.appendChild(cmdLine);
        this.scrollToBottom();
    }

    scrollToBottom() {
        this.terminalOutput.scrollTop = this.terminalOutput.scrollHeight;
    }

    focusInput() {
        this.terminalInput.focus();
    }

    clear() {
        this.terminalOutput.innerHTML = '';
    }
}

// Initialize the terminal manager when the page loads
let terminalManager;
document.addEventListener('DOMContentLoaded', () => {
    terminalManager = new TerminalManager();
});