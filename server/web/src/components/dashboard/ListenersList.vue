<template>
  <div class="listeners-list">
    <div v-if="!listeners.length" class="empty-state">
      <Icon name="network" size="48" class="empty-icon" />
      <p>No active listeners</p>
      <p class="text-secondary text-sm">Go to the Listeners page to create one</p>
    </div>
    
    <div v-else class="listeners-grid">
      <Card 
        v-for="listener in listeners" 
        :key="listener.config?.id || listener.id"
        class="listener-card"
        hover
      >
        <template #header>
          <div class="listener-header">
            <span class="listener-name">{{ getListenerName(listener) }}</span>
            <span class="listener-type">{{ getListenerProtocol(listener) }}</span>
          </div>
        </template>
        
        <div class="listener-details">
          <div class="detail-row">
            <span class="detail-label">ID:</span>
            <span class="detail-value">{{ getListenerId(listener) }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Host:</span>
            <span class="detail-value">{{ getListenerHost(listener) }}:{{ getListenerPort(listener) }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Status:</span>
            <span :class="['status-badge', `status-${getListenerStatus(listener).toLowerCase()}`]">
              {{ getListenerStatus(listener) }}
            </span>
          </div>
          <div v-if="getListenerError(listener)" class="error-message">
            Error: {{ getListenerError(listener) }}
          </div>
        </div>
        
        <template #actions>
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
            variant="danger" 
            size="small"
            icon="delete"
            @click="$emit('delete-listener', getListenerId(listener), getListenerName(listener))"
          >
            Delete
          </Button>
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
  }
})

defineEmits(['start-listener', 'stop-listener', 'delete-listener'])

function getListenerId(listener) {
  return listener.config?.id || listener.id || 'Unknown'
}

function getListenerName(listener) {
  return listener.config?.name || listener.name || 'Unnamed'
}

function getListenerProtocol(listener) {
  return listener.config?.protocol || listener.Protocol || listener.type || 'Unknown'
}

function getListenerHost(listener) {
  return listener.config?.host || listener.host || 'Unknown'
}

function getListenerPort(listener) {
  return listener.config?.port || listener.port || 'Unknown'
}

function getListenerStatus(listener) {
  return listener.status || 'Unknown'
}

function getListenerError(listener) {
  return listener.error || ''
}
</script>

<style scoped>
.listeners-list {
  height: 100%;
  overflow-y: auto;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 200px;
  text-align: center;
}

.empty-icon {
  opacity: 0.5;
  margin-bottom: var(--space-4);
}

.listeners-grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.listener-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.listener-name {
  font-weight: 600;
  color: var(--text-color);
}

.listener-type {
  font-size: 0.75rem;
  color: var(--text-secondary);
  background: var(--tertiary-bg);
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
  text-transform: uppercase;
}

.listener-details {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.detail-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.875rem;
}

.detail-label {
  color: var(--text-secondary);
  font-weight: 500;
}

.detail-value {
  color: var(--text-color);
  font-family: var(--font-mono);
  font-size: 0.8rem;
}

.status-badge {
  font-size: 0.75rem;
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
  font-weight: 600;
  text-transform: uppercase;
}

.status-active {
  background: rgba(63, 185, 80, 0.15);
  color: var(--success-color);
}

.status-stopped {
  background: rgba(139, 148, 158, 0.15);
  color: var(--text-secondary);
}

.status-error {
  background: rgba(248, 81, 73, 0.15);
  color: var(--error-color);
}

.error-message {
  font-size: 0.8rem;
  color: var(--error-color);
  background: rgba(248, 81, 73, 0.1);
  padding: var(--space-2);
  border-radius: var(--radius-sm);
  border-left: 2px solid var(--error-color);
}
</style>