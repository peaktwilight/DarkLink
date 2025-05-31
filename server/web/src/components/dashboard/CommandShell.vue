<template>
  <div class="command-shell">
    <div class="shell-output" ref="outputContainer">
      <div v-if="!selectedAgent" class="no-agent-message">
        <Icon name="terminal" size="32" class="terminal-icon" />
        <p>No agent selected</p>
        <p class="text-secondary text-sm">Select an agent to start sending commands</p>
      </div>
      
      <div v-else-if="!commandResults.length" class="loading-message">
        <p>Loading command results...</p>
      </div>
      
      <div v-else class="results-container">
        <div 
          v-for="result in commandResults" 
          :key="result.id || result.timestamp"
          class="command-result"
        >
          <div class="result-header">
            <span class="result-timestamp">{{ formatTime(result.timestamp) }}</span>
            <span class="result-command">{{ result.command }}</span>
          </div>
          <pre v-if="result.output" class="result-output">{{ result.output }}</pre>
        </div>
      </div>
    </div>
    
    <div class="shell-input">
      <div class="input-wrapper">
        <span class="prompt">
          <span class="user">{{ selectedAgent ? selectedAgent.id : 'no-agent' }}</span>
          <span class="separator">@</span>
          <span class="host">server</span>
          <span class="separator">:</span>
          <span class="path">~</span>
          <span class="prompt-symbol">$</span>
        </span>
        <input 
          ref="commandInput"
          v-model="command"
          type="text" 
          class="command-input"
          :placeholder="selectedAgent ? 'Enter command...' : 'Select an agent first'"
          :disabled="!selectedAgent || loading"
          @keypress="handleKeyPress"
          @keydown="handleKeyDown"
        >
      </div>
      <Button 
        variant="primary" 
        @click="sendCommand"
        :disabled="!selectedAgent || !command.trim() || loading"
        :loading="loading"
      >
        Send
      </Button>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, nextTick } from 'vue'
import Button from '../ui/Button.vue'
import Icon from '../ui/Icon.vue'

const props = defineProps({
  selectedAgent: {
    type: Object,
    default: null
  },
  commandResults: {
    type: Array,
    required: true
  }
})

const emit = defineEmits(['send-command'])

const command = ref('')
const loading = ref(false)
const commandHistory = ref([])
const historyIndex = ref(-1)
const outputContainer = ref(null)
const commandInput = ref(null)

// Auto-scroll to bottom when new results are added
watch(() => props.commandResults.length, async () => {
  await nextTick()
  if (outputContainer.value) {
    outputContainer.value.scrollTop = outputContainer.value.scrollHeight
  }
})

// Focus input when agent is selected
watch(() => props.selectedAgent, () => {
  if (props.selectedAgent) {
    nextTick(() => {
      commandInput.value?.focus()
    })
  }
})

function handleKeyPress(event) {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault()
    sendCommand()
  }
}

function handleKeyDown(event) {
  // Handle command history navigation
  if (event.key === 'ArrowUp') {
    event.preventDefault()
    if (historyIndex.value < commandHistory.value.length - 1) {
      historyIndex.value++
      command.value = commandHistory.value[commandHistory.value.length - 1 - historyIndex.value] || ''
    }
  } else if (event.key === 'ArrowDown') {
    event.preventDefault()
    if (historyIndex.value > 0) {
      historyIndex.value--
      command.value = commandHistory.value[commandHistory.value.length - 1 - historyIndex.value] || ''
    } else if (historyIndex.value === 0) {
      historyIndex.value = -1
      command.value = ''
    }
  }
}

async function sendCommand() {
  if (!props.selectedAgent || !command.value.trim()) return
  
  const cmd = command.value.trim()
  
  // Add to history
  if (cmd && !commandHistory.value.includes(cmd)) {
    commandHistory.value.push(cmd)
    // Keep only last 50 commands
    if (commandHistory.value.length > 50) {
      commandHistory.value = commandHistory.value.slice(-50)
    }
  }
  
  loading.value = true
  historyIndex.value = -1
  
  try {
    await emit('send-command', cmd)
    command.value = ''
  } finally {
    loading.value = false
    // Focus back on input
    await nextTick()
    commandInput.value?.focus()
  }
}

function formatTime(timestamp) {
  if (!timestamp) return ''
  const date = new Date(timestamp)
  return date.toLocaleTimeString('en-US', { 
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  })
}
</script>

<style scoped>
.command-shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-color);
  border-radius: var(--radius);
  overflow: hidden;
}

.shell-output {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4);
  background: var(--secondary-bg);
  font-family: var(--font-mono);
  font-size: 0.875rem;
  line-height: 1.4;
}

.no-agent-message,
.loading-message {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  text-align: center;
  color: var(--text-secondary);
}

.terminal-icon {
  opacity: 0.5;
  margin-bottom: var(--space-4);
}

.results-container {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.command-result {
  border-left: 3px solid var(--accent-color);
  padding-left: var(--space-3);
}

.result-header {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin-bottom: var(--space-2);
  font-size: 0.8rem;
}

.result-timestamp {
  color: var(--text-secondary);
}

.result-command {
  color: var(--accent-color);
  font-weight: 600;
}

.result-output {
  margin: 0;
  background: var(--bg-color);
  padding: var(--space-3);
  border-radius: var(--radius-sm);
  color: var(--text-color);
  white-space: pre-wrap;
  word-break: break-word;
  overflow-x: auto;
}

.shell-input {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3);
  background: var(--tertiary-bg);
  border-top: 1px solid var(--border-color);
}

.input-wrapper {
  flex: 1;
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.prompt {
  font-family: var(--font-mono);
  font-size: 0.875rem;
  color: var(--text-secondary);
  white-space: nowrap;
  display: flex;
  align-items: center;
  gap: 0.125rem;
}

.user {
  color: var(--success-color);
  font-weight: 600;
}

.host {
  color: var(--accent-color);
  font-weight: 600;
}

.path {
  color: var(--warning-color);
  font-weight: 600;
}

.prompt-symbol {
  color: var(--text-color);
  margin-left: 0.25rem;
  margin-right: 0.5rem;
}

.separator {
  color: var(--text-secondary);
}

.command-input {
  flex: 1;
  background: var(--bg-color);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  padding: var(--space-2) var(--space-3);
  font-family: var(--font-mono);
  font-size: 0.875rem;
  color: var(--text-color);
  transition: all 0.2s ease;
}

.command-input:focus {
  outline: none;
  border-color: var(--accent-color);
  box-shadow: 0 0 0 2px rgba(88, 166, 255, 0.3);
}

.command-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.command-input::placeholder {
  color: var(--text-secondary);
}
</style>