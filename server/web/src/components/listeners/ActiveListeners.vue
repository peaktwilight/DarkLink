<template>
  <div class="active-listeners">
    <div v-if="loading" class="loading-state">
      <Icon name="refresh" size="32" class="animate-spin" />
      <p>Loading listeners...</p>
    </div>
    
    <div v-else-if="!listeners.length" class="empty-state">
      <Icon name="network" size="48" class="empty-icon" />
      <p>No active listeners</p>
      <p class="text-secondary text-sm">Create a listener to get started</p>
    </div>
    
    <div v-else class="listeners-grid">
      <Card 
        v-for="listener in listeners" 
        :key="getListenerId(listener)"
        class="listener-card"
        hover
      >
        <template #header>
          <div class="listener-header">
            <div class="listener-title">
              <h4>{{ getListenerName(listener) }}</h4>
              <span class="listener-protocol">{{ getListenerProtocol(listener) }}</span>
            </div>
            <div :class="['status-indicator', `status-${getListenerStatus(listener).toLowerCase()}`]">
              <Icon :name="getStatusIcon(listener)" size="16" />
            </div>
          </div>
        </template>
        
        <div class="listener-details">
          <div class="detail-grid">
            <div class="detail-item">
              <span class="detail-label">ID</span>
              <span class="detail-value">{{ getListenerId(listener) }}</span>
            </div>
            
            <div class="detail-item">
              <span class="detail-label">Address</span>
              <span class="detail-value">
                {{ getListenerHost(listener) }}:{{ getListenerPort(listener) }}
              </span>
            </div>
            
            <div class="detail-item">
              <span class="detail-label">Status</span>
              <span :class="['status-badge', `status-${getListenerStatus(listener).toLowerCase()}`]">
                {{ getListenerStatus(listener) }}
              </span>
            </div>
            
            <div v-if="getStartTime(listener)" class="detail-item">
              <span class="detail-label">Started</span>
              <span class="detail-value">{{ formatTime(getStartTime(listener)) }}</span>
            </div>
          </div>
          
          <div v-if="getListenerError(listener)" class="error-message">
            <Icon name="x" size="16" />
            <span>{{ getListenerError(listener) }}</span>
          </div>
        </div>
        
        <template #actions>
          <div class="action-buttons">
            <Button 
              v-if="getListenerStatus(listener).toLowerCase() === 'stopped'"
              variant="success" 
              size="small"
              icon="play"
              @click="$emit('start-listener', getListenerId(listener))"
            >
              Start
            </Button>
            <Button 
              v-else
              variant="secondary" 
              size="small"
              icon="stop"
              @click="$emit('stop-listener', getListenerId(listener))"
            >
              Stop
            </Button>
            
            <Button 
              variant="secondary" 
              size="small"
              icon="settings"
              @click="$emit('edit-listener', listener)"
            >
              Edit
            </Button>
            
            <Button 
              variant="danger" 
              size="small"
              icon="delete"
              @click="$emit('delete-listener', getListenerId(listener), getListenerName(listener))"
            >
              Delete
            </Button>
          </div>
        </template>
      </Card>
    </div>
  </div>
</template>

<script setup>
import Card from '../ui/Card.vue'
import Button from '../ui/Button.vue'
import Icon from '../ui/Icon.vue'

const props = defineProps({
  listeners: {
    type: Array,
    required: true
  },
  loading: {
    type: Boolean,
    default: false
  }
})

defineEmits(['start-listener', 'stop-listener', 'delete-listener', 'edit-listener'])

function getListenerId(listener) {
  return listener.config?.ID || listener.config?.id || listener.id || 'Unknown'
}

function getListenerName(listener) {
  return listener.config?.Name || listener.config?.name || listener.name || 'Unnamed'
}

function getListenerProtocol(listener) {
  return (listener.config?.Protocol || listener.config?.protocol || listener.Protocol || listener.type || 'Unknown').toUpperCase()
}

function getListenerHost(listener) {
  return listener.config?.BindHost || listener.config?.host || listener.config?.bindHost || listener.host || '0.0.0.0'
}

function getListenerPort(listener) {
  return listener.config?.Port || listener.config?.port || listener.port || 'Unknown'
}

function getListenerStatus(listener) {
  return listener.status || 'Unknown'
}

function getListenerError(listener) {
  return listener.error || ''
}

function getStartTime(listener) {
  return listener.startTime || listener.start_time
}

function getStatusIcon(listener) {
  const status = getListenerStatus(listener).toLowerCase()
  switch (status) {
    case 'active':
    case 'running':
      return 'check'
    case 'stopped':
      return 'stop'
    case 'error':
      return 'x'
    default:
      return 'activity'
  }
}

function formatTime(timestamp) {
  if (!timestamp) return 'Unknown'
  try {
    return new Date(timestamp).toLocaleString()
  } catch (error) {
    return 'Invalid date'
  }
}
</script>

<style scoped>
.active-listeners {
  height: 100%;
  overflow-y: auto;
}

.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 300px;
  text-align: center;
}

.empty-icon {
  opacity: 0.5;
  margin-bottom: var(--space-4);
}

.listeners-grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.listener-card {
  transition: all 0.2s ease;
}

.listener-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.listener-title h4 {
  margin: 0 0 var(--space-1) 0;
  color: var(--text-color);
  font-size: 1.125rem;
}

.listener-protocol {
  font-size: 0.75rem;
  color: var(--text-secondary);
  background: var(--tertiary-bg);
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
  font-weight: 600;
}

.status-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--secondary-bg);
}

.status-indicator.status-active,
.status-indicator.status-running {
  background: rgba(63, 185, 80, 0.15);
  color: var(--success-color);
}

.status-indicator.status-stopped {
  background: rgba(139, 148, 158, 0.15);
  color: var(--text-secondary);
}

.status-indicator.status-error {
  background: rgba(248, 81, 73, 0.15);
  color: var(--error-color);
}

.listener-details {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-3);
}

.detail-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.detail-label {
  font-size: 0.75rem;
  color: var(--text-secondary);
  text-transform: uppercase;
  font-weight: 600;
  letter-spacing: 0.5px;
}

.detail-value {
  font-family: var(--font-mono);
  font-size: 0.875rem;
  color: var(--text-color);
}

.status-badge {
  font-size: 0.75rem;
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
  font-weight: 600;
  text-transform: uppercase;
  width: fit-content;
}

.status-badge.status-active,
.status-badge.status-running {
  background: rgba(63, 185, 80, 0.15);
  color: var(--success-color);
}

.status-badge.status-stopped {
  background: rgba(139, 148, 158, 0.15);
  color: var(--text-secondary);
}

.status-badge.status-error {
  background: rgba(248, 81, 73, 0.15);
  color: var(--error-color);
}

.error-message {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: rgba(248, 81, 73, 0.1);
  border: 1px solid rgba(248, 81, 73, 0.2);
  border-radius: var(--radius-sm);
  color: var(--error-color);
  font-size: 0.875rem;
}

.action-buttons {
  display: flex;
  gap: var(--space-2);
  flex-wrap: wrap;
}

@media (max-width: 768px) {
  .detail-grid {
    grid-template-columns: 1fr;
  }
  
  .action-buttons {
    justify-content: stretch;
  }
  
  .action-buttons > * {
    flex: 1;
    min-width: 0;
  }
}
</style>