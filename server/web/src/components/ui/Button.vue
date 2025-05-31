<template>
  <button 
    :class="[
      'btn',
      `btn-${variant}`,
      `btn-${size}`,
      { 'btn-loading': loading, 'btn-disabled': disabled }
    ]"
    :disabled="disabled || loading"
    @click="$emit('click', $event)"
  >
    <Icon v-if="loading" name="refresh" class="btn-icon animate-spin" />
    <Icon v-else-if="icon" :name="icon" class="btn-icon" />
    <span v-if="$slots.default" class="btn-text">
      <slot />
    </span>
  </button>
</template>

<script setup>
import Icon from './Icon.vue'

defineProps({
  variant: {
    type: String,
    default: 'primary',
    validator: (value) => ['primary', 'secondary', 'danger', 'success', 'warning'].includes(value)
  },
  size: {
    type: String,
    default: 'medium',
    validator: (value) => ['small', 'medium', 'large'].includes(value)
  },
  icon: {
    type: String,
    default: null
  },
  loading: {
    type: Boolean,
    default: false
  },
  disabled: {
    type: Boolean,
    default: false
  }
})

defineEmits(['click'])
</script>

<style scoped>
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 0 16px;
  border: none;
  border-radius: 6px;
  font-weight: 500;
  text-decoration: none;
  cursor: pointer;
  transition: all 0.2s ease;
  position: relative;
  white-space: nowrap;
}

.btn:focus {
  outline: none;
  box-shadow: 0 0 0 2px rgba(90, 200, 250, 0.3);
}

/* Sizes */
.btn-small {
  height: 32px;
  font-size: 14px;
  padding: 0 12px;
}

.btn-medium {
  height: 40px;
  font-size: 14px;
}

.btn-large {
  height: 48px;
  font-size: 16px;
  padding: 0 24px;
}

/* Variants */
.btn-primary {
  background-color: var(--button-primary);
  color: white;
}

.btn-primary:hover:not(.btn-disabled):not(.btn-loading) {
  background-color: var(--button-primary-hover);
  transform: translateY(-1px);
}

.btn-secondary {
  background-color: var(--button-secondary);
  color: var(--text-color);
  border: 1px solid var(--border-color);
}

.btn-secondary:hover:not(.btn-disabled):not(.btn-loading) {
  background-color: var(--button-secondary-hover);
  border-color: var(--accent-color);
}

.btn-danger {
  background-color: var(--button-danger);
  color: white;
}

.btn-danger:hover:not(.btn-disabled):not(.btn-loading) {
  background-color: var(--button-danger-hover);
  transform: translateY(-1px);
}

.btn-success {
  background-color: var(--success-color);
  color: white;
}

.btn-success:hover:not(.btn-disabled):not(.btn-loading) {
  background-color: #45c75a;
  transform: translateY(-1px);
}

.btn-warning {
  background-color: var(--warning-color);
  color: #333;
}

.btn-warning:hover:not(.btn-disabled):not(.btn-loading) {
  background-color: #e6b800;
  transform: translateY(-1px);
}

/* States */
.btn-loading,
.btn-disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none !important;
}

.btn-icon {
  width: 16px;
  height: 16px;
}

.animate-spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>