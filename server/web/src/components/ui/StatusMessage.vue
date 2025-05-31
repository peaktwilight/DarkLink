<template>
  <transition name="fade">
    <div 
      v-if="visible"
      :class="['status-message', `status-${type}`]"
    >
      <Icon :name="iconName" class="status-icon" />
      <span class="status-text">{{ message }}</span>
      <button 
        v-if="dismissible" 
        @click="$emit('dismiss')"
        class="status-dismiss"
      >
        <Icon name="x" size="16" />
      </button>
    </div>
  </transition>
</template>

<script setup>
import { computed } from 'vue'
import Icon from './Icon.vue'

const props = defineProps({
  type: {
    type: String,
    default: 'info',
    validator: (value) => ['success', 'error', 'warning', 'info', 'loading'].includes(value)
  },
  message: {
    type: String,
    required: true
  },
  visible: {
    type: Boolean,
    default: true
  },
  dismissible: {
    type: Boolean,
    default: true
  }
})

defineEmits(['dismiss'])

const iconName = computed(() => {
  const icons = {
    success: 'check',
    error: 'x',
    warning: 'alert-triangle',
    info: 'info',
    loading: 'refresh'
  }
  return icons[props.type] || 'info'
})
</script>

<style scoped>
.status-message {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-radius: 6px;
  font-weight: 500;
  margin: 16px 0;
}

.status-success {
  background: rgba(76, 217, 100, 0.15);
  color: var(--success-color);
  border-left: 4px solid var(--success-color);
}

.status-error {
  background: rgba(255, 59, 48, 0.15);
  color: var(--error-color);
  border-left: 4px solid var(--error-color);
}

.status-warning {
  background: rgba(255, 204, 0, 0.15);
  color: var(--warning-color);
  border-left: 4px solid var(--warning-color);
}

.status-info {
  background: rgba(90, 200, 250, 0.15);
  color: var(--accent-color);
  border-left: 4px solid var(--accent-color);
}

.status-loading {
  background: rgba(90, 200, 250, 0.15);
  color: var(--accent-color);
  border-left: 4px solid var(--accent-color);
}

.status-loading .status-icon {
  animation: spin 1s linear infinite;
}

.status-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}

.status-text {
  flex: 1;
}

.status-dismiss {
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.7;
  transition: opacity 0.2s ease;
}

.status-dismiss:hover {
  opacity: 1;
  background: rgba(255, 255, 255, 0.1);
}

.fade-enter-active,
.fade-leave-active {
  transition: all 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>