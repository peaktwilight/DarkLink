<template>
  <div class="dashboard">
    <DevNotice />
    
    <!-- Enhanced Header Section -->
    <div class="dashboard-header">
      <div class="header-content">
        <div class="title-section">
          <h1 class="dashboard-title">
            <Icon name="activity" class="title-icon" />
            Mission Control
          </h1>
          <p class="dashboard-subtitle">
            DarkLink Command & Control Center
            <span v-if="lastUpdateTime" class="last-update">
              • Last updated {{ formatLastUpdate() }}
            </span>
          </p>
        </div>
        
        <div class="stats-overview">
          <div class="stat-card agents-stat">
            <div class="stat-header">
              <div class="stat-value" :class="{ 'loading': agentsLoading }">{{ connectedAgentsCount }}</div>
              <div v-if="agentsLoading" class="loading-spinner"></div>
            </div>
            <div class="stat-label">Connected Agents</div>
            <div class="stat-detail">{{ agents.length }} total</div>
          </div>
          <div class="stat-card listeners-stat">
            <div class="stat-header">
              <div class="stat-value" :class="{ 'loading': listenersLoading }">{{ activeListenersCount }}</div>
              <div v-if="listenersLoading" class="loading-spinner"></div>
            </div>
            <div class="stat-label">Active Listeners</div>
            <div class="stat-detail">{{ listeners.length }} total</div>
          </div>
          <div class="stat-card events-stat">
            <div class="stat-value">{{ recentEventsCount }}</div>
            <div class="stat-label">Events (24h)</div>
            <div class="stat-detail">{{ events.length }} total</div>
          </div>
          <div class="stat-card uptime-stat">
            <div class="stat-value">{{ serverUptime }}</div>
            <div class="stat-label">Uptime</div>
            <div class="stat-detail">{{ formatUptime() }}</div>
          </div>
          <div class="stat-card connection-status">
            <div :class="['connection-indicator', wsConnectionStatus]">
              <div class="connection-dot"></div>
            </div>
            <div class="stat-label">Server Status</div>
            <div class="stat-detail">{{ isConnected ? 'Online' : 'Offline' }}</div>
          </div>
        </div>
      </div>
    </div>
    
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
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue'
import { useWebSocket } from '../composables/useWebSocket'
import { useApi } from '../composables/useApi'
import Card from '../components/ui/Card.vue'
import Button from '../components/ui/Button.vue'
import Icon from '../components/ui/Icon.vue'
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
const serverStartTime = ref(Date.now())
const lastUpdateTime = ref(null)
const serverStats = ref({
  totalRequests: 0,
  totalConnections: 0,
  errorRate: 0
})

// Auto-refresh intervals
let agentsInterval = null
let listenersInterval = null

onMounted(async () => {
  // Connect to WebSocket for real-time logs
  connectWebSocket('/ws/logs', {
    onMessage: handleLogMessage,
    onConnect: () => {
      addStatusMessage('Connected to server log stream', 'success')
      addEvent({
        timestamp: new Date().toISOString(),
        severity: 'SUCCESS',
        message: 'WebSocket connection established',
        source: 'system'
      })
    },
    onDisconnect: () => {
      addStatusMessage('Disconnected from log stream', 'warning')
      addEvent({
        timestamp: new Date().toISOString(),
        severity: 'WARNING',
        message: 'WebSocket connection lost',
        source: 'system'
      })
    },
    onError: () => {
      addStatusMessage('WebSocket connection error', 'error')
      addEvent({
        timestamp: new Date().toISOString(),
        severity: 'ERROR',
        message: 'WebSocket connection failed',
        source: 'system'
      })
    }
  })

  // Add initial system event
  addEvent({
    timestamp: new Date().toISOString(),
    severity: 'INFO',
    message: 'Dashboard initialized',
    source: 'system'
  })

  // Load initial data
  await Promise.all([
    loadAgents(),
    loadListeners()
  ])

  // Set up auto-refresh with staggered intervals to reduce server load
  agentsInterval = setInterval(loadAgents, 3000)  // More frequent for agents
  listenersInterval = setInterval(loadListeners, 8000)  // Less frequent for listeners
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
    const newAgents = Object.values(response || {})
    
    // Check for new agents
    const currentIds = new Set(agents.value.map(a => a.id))
    const newIds = new Set(newAgents.map(a => a.id))
    
    newIds.forEach(id => {
      if (!currentIds.has(id)) {
        const agent = newAgents.find(a => a.id === id)
        addEvent({
          timestamp: new Date().toISOString(),
          severity: 'SUCCESS',
          message: `New agent connected: ${agent.id} from ${agent.ip || 'unknown'}`,
          source: 'system'
        })
      }
    })
    
    // Check for disconnected agents
    currentIds.forEach(id => {
      if (!newIds.has(id)) {
        addEvent({
          timestamp: new Date().toISOString(),
          severity: 'WARNING',
          message: `Agent disconnected: ${id}`,
          source: 'system'
        })
      }
    })
    
    agents.value = newAgents
    lastUpdateTime.value = new Date().toISOString()
  } catch (error) {
    addStatusMessage(`Failed to load agents: ${error.message}`, 'error')
    addEvent({
      timestamp: new Date().toISOString(),
      severity: 'ERROR',
      message: `Failed to refresh agents: ${error.message}`,
      source: 'system'
    })
  } finally {
    agentsLoading.value = false
  }
}

async function loadListeners() {
  listenersLoading.value = true
  try {
    const response = await apiGet('/api/listeners/list')
    const newListeners = response || []
    
    // Check for listener status changes
    if (listeners.value.length > 0) {
      newListeners.forEach(newListener => {
        const oldListener = listeners.value.find(l => 
          (l.config?.ID || l.config?.id || l.id) === 
          (newListener.config?.ID || newListener.config?.id || newListener.id)
        )
        
        if (oldListener && oldListener.status !== newListener.status) {
          addEvent({
            timestamp: new Date().toISOString(),
            severity: newListener.status === 'Active' ? 'SUCCESS' : 'INFO',
            message: `Listener ${newListener.config?.name || 'unknown'} is now ${newListener.status}`,
            source: 'system'
          })
        }
      })
    }
    
    listeners.value = newListeners
    lastUpdateTime.value = new Date().toISOString()
  } catch (error) {
    addStatusMessage(`Failed to load listeners: ${error.message}`, 'error')
    addEvent({
      timestamp: new Date().toISOString(),
      severity: 'ERROR',
      message: `Failed to refresh listeners: ${error.message}`,
      source: 'system'
    })
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

// Computed properties for stats
const connectedAgentsCount = computed(() => {
  return agents.value.filter(agent => agent.connected === true).length
})

const activeListenersCount = computed(() => {
  return listeners.value.filter(l => 
    (l.status || '').toLowerCase() === 'active' || 
    (l.status || '').toLowerCase() === 'running'
  ).length
})

const recentEventsCount = computed(() => {
  const yesterday = new Date(Date.now() - 24 * 60 * 60 * 1000)
  return events.value.filter(e => 
    new Date(e.timestamp) > yesterday
  ).length
})

const serverUptime = computed(() => {
  const uptimeMs = Date.now() - serverStartTime.value
  const hours = Math.floor(uptimeMs / (1000 * 60 * 60))
  return hours > 0 ? `${hours}h` : '<1h'
})

const wsConnectionStatus = computed(() => {
  return isConnected.value ? 'connected' : 'disconnected'
})

function formatUptime() {
  const uptimeMs = Date.now() - serverStartTime.value
  const days = Math.floor(uptimeMs / (1000 * 60 * 60 * 24))
  const hours = Math.floor((uptimeMs % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60))
  const minutes = Math.floor((uptimeMs % (1000 * 60 * 60)) / (1000 * 60))
  
  if (days > 0) return `${days}d ${hours}h ${minutes}m`
  if (hours > 0) return `${hours}h ${minutes}m`
  return `${minutes}m`
}

function formatLastUpdate() {
  if (!lastUpdateTime.value) return 'never'
  const diff = Date.now() - new Date(lastUpdateTime.value).getTime()
  const seconds = Math.floor(diff / 1000)
  
  if (seconds < 60) return `${seconds}s ago`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}m ago`
  const hours = Math.floor(minutes / 60)
  return `${hours}h ago`
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
  height: 100vh;
  padding: var(--space-4);
  box-sizing: border-box;
  background: linear-gradient(135deg, var(--bg-color) 0%, #0a0e13 100%);
}

/* Enhanced Header Styles */
.dashboard-header {
  background: linear-gradient(135deg, var(--secondary-bg) 0%, var(--tertiary-bg) 100%);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  padding: var(--space-6);
  margin-bottom: var(--space-4);
  box-shadow: var(--shadow-lg);
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-6);
}

.title-section {
  flex: 1;
}

.dashboard-title {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  font-size: 2rem;
  font-weight: 700;
  margin: 0 0 var(--space-2) 0;
  background: linear-gradient(135deg, var(--accent-color), #79c0ff);
  background-clip: text;
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  letter-spacing: -0.02em;
}

.title-icon {
  color: var(--accent-color);
  filter: drop-shadow(0 0 8px rgba(88, 166, 255, 0.3));
}

.dashboard-subtitle {
  font-size: 1rem;
  color: var(--text-secondary);
  margin: 0;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.last-update {
  font-size: 0.875rem;
  color: var(--text-secondary);
  opacity: 0.7;
  font-family: var(--font-mono);
}

.stats-overview {
  display: flex;
  gap: var(--space-3);
  align-items: center;
  flex-wrap: wrap;
}

.stat-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--space-3) var(--space-4);
  background: rgba(88, 166, 255, 0.1);
  border: 1px solid rgba(88, 166, 255, 0.2);
  border-radius: var(--radius-md);
  min-width: 90px;
  transition: all 0.3s ease;
  position: relative;
}

.stat-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  position: relative;
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 25px rgba(88, 166, 255, 0.15);
  border-color: rgba(88, 166, 255, 0.4);
}

.stat-value {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--accent-color);
  font-family: var(--font-mono);
  line-height: 1;
  transition: opacity 0.3s ease;
}

.stat-value.loading {
  opacity: 0.6;
}

.loading-spinner {
  width: 12px;
  height: 12px;
  border: 2px solid rgba(88, 166, 255, 0.3);
  border-top: 2px solid var(--accent-color);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.stat-label {
  font-size: 0.75rem;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-top: var(--space-1);
  font-weight: 600;
}

.stat-detail {
  font-size: 0.65rem;
  color: var(--text-secondary);
  opacity: 0.8;
  margin-top: 2px;
  font-family: var(--font-mono);
}

.agents-stat {
  background: rgba(63, 185, 80, 0.1);
  border-color: rgba(63, 185, 80, 0.2);
}

.agents-stat:hover {
  border-color: rgba(63, 185, 80, 0.4);
  box-shadow: 0 8px 25px rgba(63, 185, 80, 0.15);
}

.listeners-stat {
  background: rgba(255, 193, 7, 0.1);
  border-color: rgba(255, 193, 7, 0.2);
}

.listeners-stat:hover {
  border-color: rgba(255, 193, 7, 0.4);
  box-shadow: 0 8px 25px rgba(255, 193, 7, 0.15);
}

.listeners-stat .stat-value {
  color: var(--warning-color);
}

.events-stat {
  background: rgba(139, 69, 255, 0.1);
  border-color: rgba(139, 69, 255, 0.2);
}

.events-stat:hover {
  border-color: rgba(139, 69, 255, 0.4);
  box-shadow: 0 8px 25px rgba(139, 69, 255, 0.15);
}

.events-stat .stat-value {
  color: #8b45ff;
}

.uptime-stat {
  background: rgba(0, 123, 255, 0.1);
  border-color: rgba(0, 123, 255, 0.2);
}

.uptime-stat:hover {
  border-color: rgba(0, 123, 255, 0.4);
  box-shadow: 0 8px 25px rgba(0, 123, 255, 0.15);
}

.uptime-stat .stat-value {
  color: #007bff;
}

.connection-status {
  background: rgba(63, 185, 80, 0.1);
  border-color: rgba(63, 185, 80, 0.2);
}

.connection-status:hover {
  border-color: rgba(63, 185, 80, 0.4);
  box-shadow: 0 8px 25px rgba(63, 185, 80, 0.15);
}

.connection-indicator {
  position: relative;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.connection-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  transition: all 0.3s ease;
}

.connection-indicator.connected .connection-dot {
  background: var(--success-color);
  box-shadow: 0 0 10px var(--success-color), 0 0 20px rgba(63, 185, 80, 0.5);
  animation: pulse-success 2s infinite;
}

.connection-indicator.disconnected .connection-dot {
  background: var(--error-color);
  box-shadow: 0 0 10px var(--error-color);
}

@keyframes pulse-success {
  0%, 100% {
    box-shadow: 0 0 10px var(--success-color), 0 0 20px rgba(63, 185, 80, 0.5);
  }
  50% {
    box-shadow: 0 0 15px var(--success-color), 0 0 30px rgba(63, 185, 80, 0.8);
  }
}

.dashboard-grid {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: var(--space-6);
  flex: 1;
  min-height: 0;
  max-height: calc(100vh - 450px); /* Reserve space for header and command shell */
}

.agents-section,
.listeners-section {
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.events-section {
  min-height: 0;
  display: flex;
  flex-direction: column;
  max-height: 100%;
}

.command-shell {
  height: 280px;
  flex-shrink: 0;
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
  
  .header-content {
    flex-direction: column;
    gap: var(--space-4);
    text-align: center;
  }
  
  .stats-overview {
    justify-content: center;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
}

@media (max-width: 768px) {
  .dashboard-grid {
    grid-template-columns: 1fr;
  }
  
  .dashboard {
    padding: var(--space-3);
  }
  
  .dashboard-header {
    padding: var(--space-4);
  }
  
  .dashboard-title {
    font-size: 1.5rem;
  }
  
  .stats-overview {
    gap: var(--space-2);
  }
  
  .stat-card {
    min-width: 70px;
    padding: var(--space-2) var(--space-3);
  }
  
  .stat-value {
    font-size: 1.25rem;
  }
}
</style>