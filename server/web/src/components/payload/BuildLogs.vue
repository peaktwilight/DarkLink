<template>
  <div class="build-logs">
    <div v-if="building" class="building-state">
      <Icon name="refresh" size="20" class="animate-spin" />
      <span>Building payload...</span>
    </div>
    
    <div v-else-if="!logs.length" class="empty-state">
      <Icon name="terminal" size="32" class="empty-icon" />
      <p>No build logs yet</p>
      <p class="text-secondary text-sm">Generate a payload to see build output</p>
    </div>
    
    <div v-else class="logs-container" ref="logsContainer">
      <div 
        v-for="log in logs" 
        :key="log.id"
        :class="['log-entry', `log-${log.type}`]"
      >
        <div class="log-meta">
          <span class="log-time">{{ formatTime(log.timestamp) }}</span>
          <span :class="['log-type', `type-${log.type}`]">
            <Icon :name="getLogIcon(log.type)" size="12" />
            {{ log.type.toUpperCase() }}
          </span>
        </div>
        <div class="log-message">{{ log.message }}</div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, nextTick } from 'vue'
import Icon from '../ui/Icon.vue'

const props = defineProps({
  logs: {
    type: Array,
    required: true
  },
  building: {
    type: Boolean,
    default: false
  }
})

const logsContainer = ref(null)

// Auto-scroll to bottom when new logs are added
watch(() => props.logs.length, async () => {
  await nextTick()
  if (logsContainer.value) {
    logsContainer.value.scrollTop = logsContainer.value.scrollHeight
  }
})

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

function getLogIcon(type) {
  const icons = {
    info: 'activity',
    success: 'check',
    warning: 'alert-triangle',
    error: 'x'
  }
  return icons[type] || 'activity'
}
</script>

<style scoped>
.build-logs {
  height: 100%;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.building-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-3);
  height: 200px;
  color: var(--accent-color);
  font-weight: 500;
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

.logs-container {
  flex: 1;
  overflow-y: auto;
  background: var(--bg-color);
  border-radius: var(--radius);
  padding: var(--space-2);
  font-family: var(--font-mono);
  font-size: 0.875rem;
  line-height: 1.4;
}

.log-entry {
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-sm);
  margin-bottom: var(--space-2);
  border-left: 3px solid var(--border-color);
  background: var(--secondary-bg);
  animation: fadeIn 0.3s ease-out;
}

.log-entry:hover {
  background: var(--tertiary-bg);
}

.log-entry:last-child {
  margin-bottom: 0;
}

.log-info {
  border-left-color: var(--info-color);
}

.log-success {
  border-left-color: var(--success-color);
}

.log-warning {
  border-left-color: var(--warning-color);
}

.log-error {
  border-left-color: var(--error-color);
}

.log-meta {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  margin-bottom: var(--space-1);
  font-size: 0.75rem;
}

.log-time {
  color: var(--text-secondary);
  min-width: 70px;
}

.log-type {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  font-weight: 600;
  text-transform: uppercase;
  padding: 0.125rem 0.375rem;
  border-radius: var(--radius-sm);
  min-width: 80px;
  justify-content: center;
}

.type-info {
  background: rgba(88, 166, 255, 0.15);
  color: var(--info-color);
}

.type-success {
  background: rgba(63, 185, 80, 0.15);
  color: var(--success-color);
}

.type-warning {
  background: rgba(210, 153, 34, 0.15);
  color: var(--warning-color);
}

.type-error {
  background: rgba(248, 81, 73, 0.15);
  color: var(--error-color);
}

.log-message {
  color: var(--text-color);
  white-space: pre-wrap;
  word-break: break-word;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Scrollbar styling for logs */
.logs-container::-webkit-scrollbar {
  width: 6px;
}

.logs-container::-webkit-scrollbar-track {
  background: var(--tertiary-bg);
  border-radius: 3px;
}

.logs-container::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 3px;
}

.logs-container::-webkit-scrollbar-thumb:hover {
  background: #484f58;
}
</style>