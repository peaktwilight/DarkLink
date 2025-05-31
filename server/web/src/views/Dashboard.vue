<template>
  <div class="dashboard">
    <DevNotice />
    <div class="dashboard-grid">
      <!-- Agents Section -->
      <Card class="agents-section">
        <template #header>
          <div class="section-header">
            <h3>AGENTS</h3>
            <Button 
              variant="secondary" 
              size="small" 
              icon="refresh" 
              @click="refreshAgents"
              :loading="agentsLoading"
            />
          </div>
        </template>
        <AgentsList 
          :agents="agents" 
          :selected-agent="selectedAgent"
          @select-agent="selectAgent"
          @remove-agent="removeAgent"
        />
      </Card>

      <!-- Active Listeners Section -->
      <Card class="listeners-section">
        <template #header>
          <div class="section-header">
            <h3>ACTIVE LISTENERS</h3>
            <Button 
              variant="secondary" 
              size="small" 
              icon="refresh" 
              @click="refreshListeners"
              :loading="listenersLoading"
            />
          </div>
        </template>
        <ListenersList 
          :listeners="listeners"
          @start-listener="startListener"
          @stop-listener="stopListener"
          @delete-listener="deleteListener"
        />
      </Card>

      <!-- Event Viewer Section -->
      <Card class="events-section">
        <template #header>
          <div class="section-header">
            <h3>EVENT VIEWER</h3>
            <div class="event-controls">
              <Button 
                variant="secondary" 
                size="small" 
                @click="clearEventLog"
              >
                Clear
              </Button>
              <Button 
                variant="secondary" 
                size="small" 
                @click="toggleAutoScroll"
              >
                Auto-scroll: {{ autoScroll ? 'On' : 'Off' }}
              </Button>
            </div>
          </div>
        </template>
        <EventLog 
          :events="events" 
          :auto-scroll="autoScroll"
        />
      </Card>
    </div>

    <!-- Command Shell -->
    <Card class="command-shell">
      <template #header>
        <h3 v-if="selectedAgent">
          Command Shell - Agent {{ selectedAgent.id }}
        </h3>
        <h3 v-else>Command Shell</h3>
      </template>
      <CommandShell 
        :selected-agent="selectedAgent"
        :command-results="commandResults"
        @send-command="sendCommand"
      />
    </Card>

    <!-- Status Messages -->
    <StatusMessage 
      v-for="message in statusMessages" 
      :key="message.id"
      :type="message.type"
      :message="message.text"
      :visible="message.visible"
      @dismiss="dismissMessage(message.id)"
    />
  </div>
</template>

<script setup>
import { ref, reactive, onMounted, onUnmounted } from 'vue'
import { useWebSocket } from '../composables/useWebSocket'
import { useApi } from '../composables/useApi'
import Card from '../components/ui/Card.vue'
import Button from '../components/ui/Button.vue'
import StatusMessage from '../components/ui/StatusMessage.vue'
import DevNotice from '../components/ui/DevNotice.vue'
import AgentsList from '../components/dashboard/AgentsList.vue'
import ListenersList from '../components/dashboard/ListenersList.vue'
import EventLog from '../components/dashboard/EventLog.vue'
import CommandShell from '../components/dashboard/CommandShell.vue'

// Composables
const { apiGet, apiPost, apiDelete } = useApi()
const { connect: connectWebSocket, disconnect: disconnectWebSocket, isConnected } = useWebSocket()

// Reactive state
const agents = ref([])
const listeners = ref([])
const events = ref([])
const selectedAgent = ref(null)
const commandResults = ref([])
const statusMessages = ref([])
const autoScroll = ref(true)
const agentsLoading = ref(false)
const listenersLoading = ref(false)

// Auto-refresh intervals
let agentsInterval = null
let listenersInterval = null

onMounted(async () => {
  // Connect to WebSocket for real-time logs
  connectWebSocket('/ws/logs', {
    onMessage: handleLogMessage,
    onConnect: () => addStatusMessage('Connected to server log stream', 'success'),
    onDisconnect: () => addStatusMessage('Disconnected from log stream', 'warning'),
    onError: () => addStatusMessage('WebSocket connection error', 'error')
  })

  // Load initial data
  await Promise.all([
    loadAgents(),
    loadListeners()
  ])

  // Set up auto-refresh
  agentsInterval = setInterval(loadAgents, 5000)
  listenersInterval = setInterval(loadListeners, 10000)
})

onUnmounted(() => {
  disconnectWebSocket()
  if (agentsInterval) clearInterval(agentsInterval)
  if (listenersInterval) clearInterval(listenersInterval)
})

// Data loading functions
async function loadAgents() {
  agentsLoading.value = true
  try {
    const response = await apiGet('/api/agents/list')
    agents.value = Object.values(response || {})
  } catch (error) {
    addStatusMessage(`Failed to load agents: ${error.message}`, 'error')
  } finally {
    agentsLoading.value = false
  }
}

async function loadListeners() {
  listenersLoading.value = true
  try {
    const response = await apiGet('/api/listeners/list')
    listeners.value = response || []
  } catch (error) {
    addStatusMessage(`Failed to load listeners: ${error.message}`, 'error')
  } finally {
    listenersLoading.value = false
  }
}

async function loadAgentResults(agentId) {
  try {
    const results = await apiGet(`/api/agents/${agentId}/results`)
    commandResults.value = results || []
  } catch (error) {
    addStatusMessage(`Failed to load agent results: ${error.message}`, 'error')
  }
}

// WebSocket message handler
function handleLogMessage(data) {
  try {
    const log = JSON.parse(data)
    addEvent({
      timestamp: log.timestamp || new Date().toISOString(),
      severity: log.level?.toUpperCase() || 'INFO',
      message: log.message?.trim() || '',
      source: 'server'
    })
  } catch (error) {
    console.error('Error parsing log message:', error)
  }
}

// Agent management
async function selectAgent(agent) {
  selectedAgent.value = agent
  addEvent({
    timestamp: new Date().toISOString(),
    severity: 'INFO',
    message: `Selected agent ${agent.id} for interaction`,
    source: 'system'
  })
  await loadAgentResults(agent.id)
}

async function removeAgent(agentId) {
  try {
    await apiDelete(`/api/agents/${agentId}`)
    agents.value = agents.value.filter(a => a.id !== agentId)
    if (selectedAgent.value?.id === agentId) {
      selectedAgent.value = null
      commandResults.value = []
    }
    addStatusMessage(`Agent ${agentId} removed successfully`, 'success')
  } catch (error) {
    addStatusMessage(`Failed to remove agent: ${error.message}`, 'error')
  }
}

// Listener management
async function startListener(listenerId) {
  try {
    await apiPost(`/api/listeners/${listenerId}/start`)
    await loadListeners()
    addStatusMessage('Listener started successfully', 'success')
  } catch (error) {
    addStatusMessage(`Failed to start listener: ${error.message}`, 'error')
  }
}

async function stopListener(listenerId) {
  try {
    await apiPost(`/api/listeners/${listenerId}/stop`)
    await loadListeners()
    addStatusMessage('Listener stopped successfully', 'success')
  } catch (error) {
    addStatusMessage(`Failed to stop listener: ${error.message}`, 'error')
  }
}

async function deleteListener(listenerId, listenerName) {
  if (!confirm(`Are you sure you want to delete ${listenerName}?`)) return
  
  try {
    await apiDelete(`/api/listeners/${listenerId}`)
    await loadListeners()
    addStatusMessage(`Listener ${listenerName} deleted successfully`, 'success')
  } catch (error) {
    addStatusMessage(`Failed to delete listener: ${error.message}`, 'error')
  }
}

// Command handling
async function sendCommand(command) {
  if (!selectedAgent.value) {
    addStatusMessage('No agent selected. Click "Interact" on an agent first.', 'warning')
    return
  }

  try {
    await apiPost(`/api/agents/${selectedAgent.value.id}/command`, { command })
    addEvent({
      timestamp: new Date().toISOString(),
      severity: 'INFO',
      message: `Command sent to agent ${selectedAgent.value.id}: ${command}`,
      source: 'user'
    })
    // Refresh results after sending command
    setTimeout(() => loadAgentResults(selectedAgent.value.id), 1000)
  } catch (error) {
    addStatusMessage(`Failed to send command: ${error.message}`, 'error')
  }
}

// Event log management
function addEvent(event) {
  events.value.push({
    ...event,
    id: Date.now() + Math.random()
  })
  
  // Keep only last 1000 events
  if (events.value.length > 1000) {
    events.value = events.value.slice(-1000)
  }
}

function clearEventLog() {
  events.value = []
}

function toggleAutoScroll() {
  autoScroll.value = !autoScroll.value
}

// Status message management
function addStatusMessage(text, type = 'info') {
  const id = Date.now() + Math.random()
  statusMessages.value.push({
    id,
    text,
    type,
    visible: true
  })
  
  // Auto-dismiss after 5 seconds
  setTimeout(() => {
    dismissMessage(id)
  }, 5000)
}

function dismissMessage(id) {
  const message = statusMessages.value.find(m => m.id === id)
  if (message) {
    message.visible = false
    setTimeout(() => {
      statusMessages.value = statusMessages.value.filter(m => m.id !== id)
    }, 300)
  }
}

// Refresh functions
async function refreshAgents() {
  await loadAgents()
  addStatusMessage('Agents refreshed', 'success')
}

async function refreshListeners() {
  await loadListeners()
  addStatusMessage('Listeners refreshed', 'success')
}
</script>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  height: 100%;
}

.dashboard-grid {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: var(--space-6);
  flex: 1;
  min-height: 0;
}

.agents-section,
.listeners-section,
.events-section {
  min-height: 400px;
}

.command-shell {
  height: 250px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.section-header h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--accent-color);
  letter-spacing: 0.5px;
}

.event-controls {
  display: flex;
  gap: var(--space-2);
}

@media (max-width: 1200px) {
  .dashboard-grid {
    grid-template-columns: 1fr 1fr;
  }
  
  .events-section {
    grid-column: 1 / -1;
  }
}

@media (max-width: 768px) {
  .dashboard-grid {
    grid-template-columns: 1fr;
  }
}
</style>