<template>
  <div class="agents-list">
    <div v-if="!agents.length" class="empty-state">
      <Icon name="user" size="48" class="empty-icon" />
      <p>No active agents</p>
      <p class="text-secondary text-sm">Generate a payload to get started</p>
    </div>
    
    <div v-else class="agents-grid">
      <Card 
        v-for="agent in agents" 
        :key="agent.id"
        :selected="selectedAgent?.id === agent.id"
        class="agent-card"
        hover
      >
        <template #header>
          <div class="agent-header">
            <div class="agent-title">
              <div :class="['agent-status', agentStatus(agent)]"></div>
              <span class="agent-name">{{ agent.id }}</span>
            </div>
            <span class="agent-type">{{ agent.type || 'Standard' }}</span>
          </div>
        </template>
        
        <div class="agent-details">
          <div class="detail-row">
            <span class="detail-label">Last Seen:</span>
            <span class="detail-value">{{ formatDate(agent.last_seen) }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">IP:</span>
            <span class="detail-value">{{ agent.ip || 'Unknown' }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Hostname:</span>
            <span class="detail-value">{{ agent.hostname || 'Unknown' }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">OS:</span>
            <span class="detail-value">{{ agent.os || 'Unknown' }}</span>
          </div>
        </div>
        
        <template #actions>
          <Button 
            variant="primary" 
            size="small" 
            @click="$emit('select-agent', agent)"
          >
            Interact
          </Button>
          <Button 
            variant="danger" 
            size="small" 
            @click="$emit('remove-agent', agent.id)"
          >
            Remove
          </Button>
        </template>
      </Card>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import Card from '../ui/Card.vue'
import Button from '../ui/Button.vue'
import Icon from '../ui/Icon.vue'

const props = defineProps({
  agents: {
    type: Array,
    required: true
  },
  selectedAgent: {
    type: Object,
    default: null
  }
})

defineEmits(['select-agent', 'remove-agent'])

function agentStatus(agent) {
  return agent.connected ? 'active' : 'disconnected'
}

function formatDate(dateString) {
  if (!dateString) return 'Never'
  return new Date(dateString).toLocaleString()
}
</script>

<style scoped>
.agents-list {
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

.agents-grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.agent-card {
  cursor: pointer;
}

.agent-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.agent-title {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.agent-status {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.agent-status.active {
  background-color: var(--success-color);
  box-shadow: 0 0 6px var(--success-color);
}

.agent-status.disconnected {
  background-color: var(--text-secondary);
}

.agent-name {
  font-weight: 600;
  color: var(--text-color);
  font-family: var(--font-mono);
  font-size: 0.875rem;
}

.agent-type {
  font-size: 0.75rem;
  color: var(--text-secondary);
  background: var(--tertiary-bg);
  padding: 0.25rem 0.5rem;
  border-radius: var(--radius-sm);
}

.agent-details {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.detail-row {
  display: flex;
  justify-content: space-between;
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
</style>