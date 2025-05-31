<template>
  <div class="event-log" ref="logContainer">
    <div v-if="!events.length" class="empty-state">
      <Icon name="activity" size="48" class="empty-icon" />
      <p>No events yet</p>
      <p class="text-secondary text-sm">Server events will appear here</p>
    </div>
    
    <div v-else class="events-container">
      <div 
        v-for="event in events" 
        :key="event.id"
        :class="['log-entry', `log-${event.severity.toLowerCase()}`]"
      >
        <div class="log-meta">
          <span class="log-time">{{ formatTime(event.timestamp) }}</span>
          <span :class="['log-severity', `severity-${event.severity.toLowerCase()}`]">
            {{ event.severity }}
          </span>
          <span class="log-source">{{ event.source }}</span>
        </div>
        <div class="log-message">{{ event.message }}</div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, nextTick } from 'vue'
import Icon from '../ui/Icon.vue'

const props = defineProps({
  events: {
    type: Array,
    required: true
  },
  autoScroll: {
    type: Boolean,
    default: true
  }
})

const logContainer = ref(null)

// Auto-scroll to bottom when new events are added
watch(() => props.events.length, async () => {
  if (props.autoScroll) {
    await nextTick()
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight
    }
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
</script>

<style scoped>
.event-log {
  height: 100%;
  overflow-y: auto;
  background: var(--secondary-bg);
  border-radius: var(--radius);
  padding: var(--space-2);
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

.events-container {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.log-entry {
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-sm);
  background: var(--bg-color);
  border-left: 3px solid var(--border-color);
  animation: fadeIn 0.3s ease-out;
}

.log-entry:hover {
  background: var(--tertiary-bg);
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
  font-family: var(--font-mono);
  color: var(--text-secondary);
  min-width: 70px;
}

.log-severity {
  font-weight: 600;
  text-transform: uppercase;
  padding: 0.125rem 0.375rem;
  border-radius: var(--radius-sm);
  min-width: 60px;
  text-align: center;
}

.severity-info {
  background: rgba(88, 166, 255, 0.15);
  color: var(--info-color);
}

.severity-success {
  background: rgba(63, 185, 80, 0.15);
  color: var(--success-color);
}

.severity-warning {
  background: rgba(210, 153, 34, 0.15);
  color: var(--warning-color);
}

.severity-error {
  background: rgba(248, 81, 73, 0.15);
  color: var(--error-color);
}

.log-source {
  color: var(--text-secondary);
  font-style: italic;
}

.log-message {
  font-family: var(--font-mono);
  font-size: 0.875rem;
  line-height: 1.4;
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
</style>