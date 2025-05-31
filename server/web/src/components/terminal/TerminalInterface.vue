<template>
  <div class="terminal-interface">
    <div class="terminal-output" ref="outputContainer">
      <div v-if="!connected" class="connection-status">
        <Icon name="terminal" size="48" class="terminal-icon" />
        <h3>Terminal Disconnected</h3>
        <p>Connecting to server terminal...</p>
        <Button variant="primary" @click="$emit('connect')">
          Reconnect
        </Button>
      </div>
      
      <div v-else class="output-content">
        <div 
          v-for="output in terminalOutput" 
          :key="output.id"
          :class="['output-line', { 'error': output.error }]"
        >
          <span v-if="output.cwd" class="prompt">
            <span class="user">user</span>@<span class="host">server</span>:<span class="path">{{ output.cwd }}</span>$
          </span>
          <pre v-if="output.output" class="command-output">{{ output.output }}</pre>
        </div>
      </div>
    </div>

    <div class="terminal-input-container">
      <div class="input-line">
        <span class="prompt">
          <span class="user">user</span>@<span class="host">server</span>:<span class="path">{{ currentPath }}</span>$
        </span>
        <input 
          ref="commandInput"
          v-model="currentCommand"
          type="text" 
          class="command-input"
          :disabled="!connected || commandLoading"
          @keydown="handleKeyDown"
          @keypress="handleKeyPress"
          autocomplete="off"
          spellcheck="false"
        >
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, watch, nextTick, onMounted } from 'vue'
import { useWebSocket } from '../../composables/useWebSocket'
import Button from '../ui/Button.vue'
import Icon from '../ui/Icon.vue'

const props = defineProps({
  connected: {
    type: Boolean,
    default: false
  }
})

defineEmits(['connect', 'disconnect'])

const { send } = useWebSocket()

// Terminal state
const terminalOutput = ref([])
const currentCommand = ref('')
const currentPath = ref('~')
const commandHistory = ref([])
const historyIndex = ref(-1)
const commandLoading = ref(false)
const outputContainer = ref(null)
const commandInput = ref(null)

// WebSocket setup
const { connect: wsConnect, disconnect: wsDisconnect } = useWebSocket()

onMounted(() => {
  connectWebSocket()
  focusInput()
})

// Auto-scroll when new output is added
watch(() => terminalOutput.value.length, async () => {
  await nextTick()
  scrollToBottom()
})

// Focus input when connected
watch(() => props.connected, (connected) => {
  if (connected) {
    nextTick(() => focusInput())
  }
})

function connectWebSocket() {
  wsConnect('/ws/terminal', {
    onMessage: handleWebSocketMessage,
    onConnect: () => {
      addOutput('Connected to server terminal (Bash shell).', '~')
    },
    onDisconnect: () => {
      addOutput('Disconnected from server terminal.', currentPath.value, true)
    },
    onError: (error) => {
      addOutput(`Terminal error: ${error}`, currentPath.value, true)
    }
  })
}

function handleWebSocketMessage(data) {
  try {
    const response = JSON.parse(data)
    
    if (response.type === 'tab_completion') {
      handleTabCompletion(response.completions)
      return
    }
    
    if (response.output) {
      addOutput(response.output, response.cwd, response.error)
    }
    
    if (response.cwd) {
      currentPath.value = response.cwd
    }
    
    commandLoading.value = false
  } catch (error) {
    console.error('Error parsing terminal response:', error)
    addOutput('Error: Invalid response from server', currentPath.value, true)
    commandLoading.value = false
  }
}

function handleKeyDown(event) {
  // Handle command history navigation
  if (event.key === 'ArrowUp') {
    event.preventDefault()
    navigateHistory(-1)
  } else if (event.key === 'ArrowDown') {
    event.preventDefault()
    navigateHistory(1)
  } else if (event.key === 'Tab') {
    event.preventDefault()
    handleTabCompletion()
  }
}

function handleKeyPress(event) {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault()
    executeCommand()
  }
}

function navigateHistory(direction) {
  if (commandHistory.value.length === 0) return
  
  if (direction === -1) {
    // Go back in history
    if (historyIndex.value < commandHistory.value.length - 1) {
      historyIndex.value++
      currentCommand.value = commandHistory.value[commandHistory.value.length - 1 - historyIndex.value]
    }
  } else {
    // Go forward in history
    if (historyIndex.value > 0) {
      historyIndex.value--
      currentCommand.value = commandHistory.value[commandHistory.value.length - 1 - historyIndex.value]
    } else if (historyIndex.value === 0) {
      historyIndex.value = -1
      currentCommand.value = ''
    }
  }
}

function handleTabCompletion(completions = null) {
  if (completions) {
    // Handle received completions
    if (completions.length === 1) {
      // Single completion - auto-complete
      currentCommand.value = completions[0]
    } else if (completions.length > 1) {
      // Multiple completions - show options
      addOutput(`Available completions:\n${completions.join('  ')}`, currentPath.value)
    }
  } else {
    // Request tab completion
    const request = {
      type: 'tab_completion',
      partial: currentCommand.value
    }
    send(JSON.stringify(request))
  }
}

function executeCommand() {
  if (!props.connected || commandLoading.value) return
  
  const command = currentCommand.value.trim()
  if (!command) return
  
  // Add to history
  if (command && !commandHistory.value.includes(command)) {
    commandHistory.value.push(command)
    // Keep only last 100 commands
    if (commandHistory.value.length > 100) {
      commandHistory.value = commandHistory.value.slice(-100)
    }
  }
  
  // Show command in output
  addOutput(`$ ${command}`, currentPath.value)
  
  // Send command
  commandLoading.value = true
  historyIndex.value = -1
  send(command)
  
  // Clear input
  currentCommand.value = ''
}

function addOutput(output, cwd = null, error = false) {
  terminalOutput.value.push({
    id: Date.now() + Math.random(),
    output,
    cwd,
    error,
    timestamp: new Date()
  })
  
  // Keep only last 1000 lines
  if (terminalOutput.value.length > 1000) {
    terminalOutput.value = terminalOutput.value.slice(-1000)
  }
}

function scrollToBottom() {
  if (outputContainer.value) {
    outputContainer.value.scrollTop = outputContainer.value.scrollHeight
  }
}

function focusInput() {
  if (commandInput.value && props.connected) {
    commandInput.value.focus()
  }
}
</script>

<style scoped>
.terminal-interface {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #0d1117;
  color: #c9d1d9;
  font-family: var(--font-mono);
  font-size: 14px;
  line-height: 1.4;
}

.terminal-output {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4);
  background: #0d1117;
}

.connection-status {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  text-align: center;
  color: #8b949e;
}

.terminal-icon {
  opacity: 0.5;
  margin-bottom: var(--space-4);
}

.connection-status h3 {
  margin: 0 0 var(--space-2) 0;
  color: #f0f6fc;
}

.connection-status p {
  margin: 0 0 var(--space-4) 0;
}

.output-content {
  font-family: var(--font-mono);
}

.output-line {
  margin-bottom: var(--space-1);
  word-break: break-word;
}

.output-line.error {
  color: #f85149;
}

.command-output {
  margin: 0;
  white-space: pre-wrap;
  color: #c9d1d9;
  background: transparent;
  border: none;
  padding: 0;
  font-family: inherit;
  font-size: inherit;
}

.terminal-input-container {
  border-top: 1px solid #21262d;
  background: #161b22;
  padding: var(--space-3) var(--space-4);
}

.input-line {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.prompt {
  color: #8b949e;
  white-space: nowrap;
  font-family: var(--font-mono);
  font-size: 14px;
}

.prompt .user {
  color: #3fb950;
  font-weight: 600;
}

.prompt .host {
  color: #58a6ff;
  font-weight: 600;
}

.prompt .path {
  color: #d29922;
  font-weight: 600;
}

.command-input {
  flex: 1;
  background: transparent;
  border: none;
  color: #f0f6fc;
  font-family: var(--font-mono);
  font-size: 14px;
  padding: 0;
  outline: none;
}

.command-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Custom scrollbar for terminal output */
.terminal-output::-webkit-scrollbar {
  width: 8px;
}

.terminal-output::-webkit-scrollbar-track {
  background: #0d1117;
}

.terminal-output::-webkit-scrollbar-thumb {
  background: #21262d;
  border-radius: 4px;
}

.terminal-output::-webkit-scrollbar-thumb:hover {
  background: #30363d;
}

/* Terminal cursor animation */
.command-input {
  caret-color: #58a6ff;
}

@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}

.command-input:focus {
  animation: none;
}

/* Selection styling */
::selection {
  background: #264f78;
}

::-moz-selection {
  background: #264f78;
}
</style>