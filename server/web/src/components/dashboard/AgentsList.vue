<template>
  <div class="agents-list">
    <div v-if="!agents.length" class="empty-state">
      <div class="empty-content">
        <div class="empty-icon-container">
          <Icon name="user" size="48" class="empty-icon" />
          <div class="icon-glow"></div>
        </div>
        <h3 class="empty-title">No Active Agents</h3>
        <p class="empty-description">Deploy payloads to establish connections with target systems</p>
        <div class="empty-actions">
          <Button 
            variant="primary" 
            @click="$router.push('/payload')"
            class="cta-button"
          >
            <Icon name="plus" />Generate Payload
          </Button>
        </div>
      </div>
    </div>
    
    <div v-else class="agents-grid">
      <Card 
        v-for="(agent, index) in agents" 
        :key="agent.id"
        :selected="selectedAgent?.id === agent.id"
        class="agent-card animate-fadeIn"
        :style="{ animationDelay: `${index * 100}ms` }"
        hover
      >
        <template #header>
          <div class="agent-header">
            <div class="agent-title">
              <div :class="['agent-status', agentStatus(agent)]">
                <div v-if="agentStatus(agent) === 'active'" class="status-pulse"></div>
              </div>
              <span class="agent-name">{{ agent.id }}</span>
              <Icon v-if="agentStatus(agent) === 'active'" name="zap" class="activity-icon" />
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

.agents-grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.agent-card {
  cursor: pointer;
  transition: all 0.3s ease;
  transform: translateY(0);
}

.agent-card:hover {
  transform: translateY(-4px);
  box-shadow: var(--shadow-lg), 0 0 20px rgba(88, 166, 255, 0.1);
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

.activity-icon {
  color: var(--warning-color);
  font-size: 0.875rem;
  animation: flash 2s infinite;
}

@keyframes flash {
  0%, 50%, 100% {
    opacity: 1;
  }
  25%, 75% {
    opacity: 0.3;
  }
}

.agent-status {
  position: relative;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  flex-shrink: 0;
  transition: all 0.3s ease;
}

.status-pulse {
  position: absolute;
  top: -2px;
  left: -2px;
  right: -2px;
  bottom: -2px;
  border-radius: 50%;
  border: 2px solid var(--success-color);
  animation: pulse-ring 2s infinite;
  opacity: 0.6;
}

@keyframes pulse-ring {
  0% {
    transform: scale(0.8);
    opacity: 0.8;
  }
  100% {
    transform: scale(1.4);
    opacity: 0;
  }
}

.agent-status.active {
  background-color: var(--success-color);
  box-shadow: 0 0 12px var(--success-color), 0 0 24px rgba(63, 185, 80, 0.3);
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