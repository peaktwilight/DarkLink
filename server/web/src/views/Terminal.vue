<template>
  <div class="terminal-page">
    <div class="page-header">
      <h1>Server Terminal</h1>
      <div class="terminal-status">
        <Icon 
          :name="isConnected ? 'check' : 'x'" 
          :size="16"
          :class="['status-icon', isConnected ? 'connected' : 'disconnected']"
        />
        <span>{{ isConnected ? 'Connected' : 'Disconnected' }}</span>
      </div>
    </div>

    <Card class="terminal-container">
      <TerminalInterface 
        :connected="isConnected"
        @connect="connectTerminal"
        @disconnect="disconnectTerminal"
      />
    </Card>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useWebSocket } from '../composables/useWebSocket'
import Card from '../components/ui/Card.vue'
import Icon from '../components/ui/Icon.vue'
import TerminalInterface from '../components/terminal/TerminalInterface.vue'

const { connect, disconnect, isConnected } = useWebSocket()

onMounted(() => {
  connectTerminal()
})

onUnmounted(() => {
  disconnectTerminal()
})

function connectTerminal() {
  connect('/ws/terminal')
}

function disconnectTerminal() {
  disconnect()
}
</script>

<style scoped>
.terminal-page {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
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

.terminal-status {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: 0.875rem;
  color: var(--text-secondary);
}

.status-icon.connected {
  color: var(--success-color);
}

.status-icon.disconnected {
  color: var(--error-color);
}

.terminal-container {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
</style>