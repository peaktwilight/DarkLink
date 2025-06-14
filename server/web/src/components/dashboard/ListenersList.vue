<template>
  <div class="listeners-list">
    <div v-if="!listeners.length" class="empty-state">
      <div class="empty-content">
        <div class="empty-icon-container">
          <Icon name="network" size="48" class="empty-icon" />
          <div class="icon-glow"></div>
        </div>
        <h3 class="empty-title">No Active Listeners</h3>
        <p class="empty-description">Configure listeners to receive agent connections</p>
        <div class="empty-actions">
          <Button 
            variant="primary" 
            @click="$router.push('/listeners')"
            class="cta-button"
          >
            <Icon name="plus" />Create Listener
          </Button>
        </div>
      </div>
    </div>
    
    <div v-else class="listeners-grid">
      <Card 
        v-for="listener in listeners" 
        :key="listener.config?.ID || listener.config?.id || listener.id"
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
  return listener.config?.ID || listener.config?.id || listener.id || 'Unknown'
}

function getListenerName(listener) {
  return listener.config?.Name || listener.config?.name || listener.name || 'Unnamed'
}

function getListenerProtocol(listener) {
  const protocol = listener.config?.Protocol || listener.config?.protocol || listener.type || 'Unknown'
  return (typeof protocol === 'string' ? protocol : 'Unknown').toUpperCase()
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
  height: 280px;
  text-align: center;
  padding: var(--space-6);
}

.empty-content {
  max-width: 300px;
}

.empty-icon-container {
  position: relative;
  display: inline-block;
  margin-bottom: var(--space-6);
}

.empty-icon {
  color: var(--text-secondary);
  opacity: 0.6;
  filter: drop-shadow(0 0 10px rgba(139, 148, 158, 0.3));
}

.icon-glow {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 80px;
  height: 80px;
  background: radial-gradient(circle, rgba(88, 166, 255, 0.1), transparent 70%);
  border-radius: 50%;
  animation: gentle-pulse 4s infinite;
}

@keyframes gentle-pulse {
  0%, 100% {
    opacity: 0.3;
    transform: translate(-50%, -50%) scale(0.8);
  }
  50% {
    opacity: 0.6;
    transform: translate(-50%, -50%) scale(1.2);
  }
}

.empty-title {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text-color);
  margin: 0 0 var(--space-2) 0;
}

.empty-description {
  color: var(--text-secondary);
  font-size: 0.875rem;
  line-height: 1.5;
  margin: 0 0 var(--space-6) 0;
}

.empty-actions {
  display: flex;
  justify-content: center;
}

.cta-button {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-weight: 600;
  transform: translateY(0);
  transition: all 0.3s ease;
}

.cta-button:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 25px rgba(35, 134, 54, 0.4);
}

.listeners-grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.listener-card {
  transition: all 0.3s ease;
  transform: translateY(0);
}

.listener-card:hover {
  transform: translateY(-3px);
  box-shadow: var(--shadow-lg), 0 0 20px rgba(88, 166, 255, 0.1);
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