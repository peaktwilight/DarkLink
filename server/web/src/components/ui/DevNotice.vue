<template>
  <div v-if="isDevelopment" class="dev-notice">
    <Icon name="info" size="16" />
    <span>Development Mode - Using mock data</span>
    <small>Start the Go server on port 8080 for live data</small>
  </div>
</template>

<script setup>
import Icon from './Icon.vue'

const isDevelopment = import.meta.env.DEV && !window.location.port.includes('8080')
</script>

<style scoped>
.dev-notice {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: rgba(210, 153, 34, 0.1);
  border: 1px solid rgba(210, 153, 34, 0.2);
  border-radius: var(--radius);
  color: var(--warning-color);
  font-size: 0.875rem;
  margin-bottom: var(--space-4);
}

.dev-notice small {
  margin-left: auto;
  opacity: 0.8;
  font-size: 0.75rem;
}
</style>