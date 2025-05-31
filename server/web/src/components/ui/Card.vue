<template>
  <div :class="['card', { 'card-hover': hover, 'card-selected': selected }]">
    <div v-if="$slots.header" class="card-header">
      <slot name="header" />
    </div>
    <div class="card-content">
      <slot />
    </div>
    <div v-if="$slots.actions" class="card-actions">
      <slot name="actions" />
    </div>
  </div>
</template>

<script setup>
defineProps({
  hover: {
    type: Boolean,
    default: true
  },
  selected: {
    type: Boolean,
    default: false
  }
})
</script>

<style scoped>
.card {
  background: var(--bg-color);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  overflow: hidden;
  transition: all 0.2s ease;
}

.card-hover:hover {
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
  transform: translateY(-2px);
}

.card-selected {
  border-color: var(--accent-color);
  box-shadow: 0 0 0 1px var(--accent-color);
}

.card-header {
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-color);
  background: var(--secondary-bg);
}

.card-content {
  padding: 20px;
}

.card-actions {
  padding: 16px 20px;
  border-top: 1px solid var(--border-color);
  background: var(--secondary-bg);
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
</style>