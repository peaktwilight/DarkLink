<template>
  <aside class="sidebar">
    <div class="nav-header">MICROC2 CONSOLE</div>
    <nav class="nav-items">
      <router-link 
        v-for="item in navItems" 
        :key="item.name"
        :to="item.path"
        class="nav-item"
        :class="{ active: $route.name === item.name }"
      >
        <Icon :name="item.icon" class="nav-icon" />
        {{ item.title }}
      </router-link>
    </nav>
  </aside>
</template>

<script setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import Icon from '../ui/Icon.vue'

const route = useRoute()

const navItems = [
  { name: 'Dashboard', title: 'Mission Control', path: '/', icon: 'dashboard' },
  { name: 'Listeners', title: 'Listeners', path: '/listeners', icon: 'network' },
  { name: 'Payload', title: 'Payload', path: '/payload', icon: 'code' },
  { name: 'FileDrop', title: 'File Drop', path: '/file-drop', icon: 'upload' },
  { name: 'Terminal', title: 'Server Terminal', path: '/terminal', icon: 'terminal' }
]
</script>

<style scoped>
.sidebar {
  width: 220px;
  background-color: var(--secondary-bg);
  border-right: 1px solid var(--border-color);
  padding: 20px 0;
  display: flex;
  flex-direction: column;
}

.nav-header {
  padding: 0 20px;
  margin-bottom: 30px;
  font-weight: bold;
  color: var(--accent-color);
  font-size: 14px;
  letter-spacing: 0.5px;
}

.nav-items {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 20px;
  color: var(--text-color);
  text-decoration: none;
  transition: all 0.2s ease;
  border-left: 3px solid transparent;
  font-weight: 500;
}

.nav-item:hover {
  background-color: rgba(90, 200, 250, 0.1);
  color: var(--accent-color);
}

.nav-item.active {
  background-color: rgba(90, 200, 250, 0.15);
  color: var(--accent-color);
  border-left-color: var(--accent-color);
}

.nav-icon {
  width: 18px;
  height: 18px;
  opacity: 0.8;
}

.nav-item.active .nav-icon {
  opacity: 1;
}
</style>