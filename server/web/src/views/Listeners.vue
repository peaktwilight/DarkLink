<template>
  <div class="listeners-page">
    <div class="page-header">
      <h1>Listener Configuration</h1>
      <Button variant="primary" icon="refresh" @click="loadListeners">
        Refresh
      </Button>
      <Button variant="secondary" @click="testMinimalListener">
        Test Minimal
      </Button>
    </div>

    <StatusMessage 
      v-if="statusMessage.visible"
      :type="statusMessage.type"
      :message="statusMessage.text"
      @dismiss="clearStatusMessage"
    />

    <div class="listeners-layout">
      <!-- Configuration Panel -->
      <Card class="config-panel">
        <template #header>
          <h2>Create New Listener</h2>
        </template>

        <ListenerForm 
          :loading="formLoading"
          @submit="createListener"
        />
      </Card>

      <!-- Active Listeners Panel -->
      <Card class="listeners-panel">
        <template #header>
          <div class="panel-header">
            <h3>Active Listeners</h3>
            <span class="listener-count">{{ listeners.length }} active</span>
          </div>
        </template>

        <ActiveListeners 
          :listeners="listeners"
          :loading="listenersLoading"
          @start-listener="startListener"
          @stop-listener="stopListener"
          @delete-listener="deleteListener"
          @edit-listener="editListener"
        />
      </Card>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useApi } from '../composables/useApi'
import Card from '../components/ui/Card.vue'
import Button from '../components/ui/Button.vue'
import StatusMessage from '../components/ui/StatusMessage.vue'
import ListenerForm from '../components/listeners/ListenerForm.vue'
import ActiveListeners from '../components/listeners/ActiveListeners.vue'

const { apiGet, apiPost, apiDelete } = useApi()

// Reactive state
const listeners = ref([])
const formLoading = ref(false)
const listenersLoading = ref(false)
const statusMessage = ref({
  visible: false,
  type: 'info',
  text: ''
})

onMounted(() => {
  loadListeners()
})

async function loadListeners() {
  listenersLoading.value = true
  try {
    const response = await apiGet('/api/listeners/list')
    listeners.value = response || []
  } catch (error) {
    showStatusMessage(`Failed to load listeners: ${error.message}`, 'error')
  } finally {
    listenersLoading.value = false
  }
}

async function createListener(config) {
  formLoading.value = true
  try {
    await apiPost('/api/listeners/create', config)
    showStatusMessage('Listener created successfully', 'success')
    await loadListeners()
  } catch (error) {
    showStatusMessage(`Failed to create listener: ${error.message}`, 'error')
  } finally {
    formLoading.value = false
  }
}

async function startListener(listenerId) {
  try {
    await apiPost(`/api/listeners/${listenerId}/start`)
    showStatusMessage('Listener started successfully', 'success')
    await loadListeners()
  } catch (error) {
    showStatusMessage(`Failed to start listener: ${error.message}`, 'error')
  }
}

async function stopListener(listenerId) {
  try {
    await apiPost(`/api/listeners/${listenerId}/stop`)
    showStatusMessage('Listener stopped successfully', 'success')
    await loadListeners()
  } catch (error) {
    showStatusMessage(`Failed to stop listener: ${error.message}`, 'error')
  }
}

async function deleteListener(listenerId, listenerName) {
  if (!confirm(`Are you sure you want to delete "${listenerName}"?`)) return
  
  try {
    await apiDelete(`/api/listeners/${listenerId}`)
    showStatusMessage(`Listener "${listenerName}" deleted successfully`, 'success')
    await loadListeners()
  } catch (error) {
    showStatusMessage(`Failed to delete listener: ${error.message}`, 'error')
  }
}

function editListener(listener) {
  // TODO: Implement edit functionality
  showStatusMessage('Edit functionality coming soon', 'info')
}

async function testMinimalListener() {
  const minimalConfig = {
    Name: "test-listener",
    Protocol: "http",
    Port: 9999
  }
  
  console.log('🧪 Testing minimal listener config:', JSON.stringify(minimalConfig, null, 2))
  
  try {
    await apiPost('/api/listeners/create', minimalConfig)
    showStatusMessage('Minimal listener created successfully!', 'success')
    await loadListeners()
  } catch (error) {
    showStatusMessage(`Minimal test failed: ${error.message}`, 'error')
  }
}

function showStatusMessage(text, type = 'info') {
  statusMessage.value = {
    visible: true,
    type,
    text
  }
  
  // Auto-hide after 5 seconds
  setTimeout(() => {
    clearStatusMessage()
  }, 5000)
}

function clearStatusMessage() {
  statusMessage.value.visible = false
}
</script>

<style scoped>
.listeners-page {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  height: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.page-header h1 {
  margin: 0;
  color: var(--text-color);
}

.listeners-layout {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-6);
  flex: 1;
  min-height: 0;
}

.config-panel {
  height: fit-content;
}

.listeners-panel {
  height: 100%;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.panel-header h3 {
  margin: 0;
}

.listener-count {
  font-size: 0.875rem;
  color: var(--text-secondary);
  background: var(--tertiary-bg);
  padding: 0.25rem 0.75rem;
  border-radius: var(--radius);
}

@media (max-width: 1024px) {
  .listeners-layout {
    grid-template-columns: 1fr;
  }
}
</style>