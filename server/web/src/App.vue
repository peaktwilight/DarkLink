<template>
  <div class="microc2-container">
    <Sidebar />
    <main class="content">
      <router-view />
    </main>
  </div>
</template>

<script setup>
import Sidebar from './components/layout/Sidebar.vue'
</script>

<style>
/* Global app styles */
.microc2-container {
  display: flex;
  min-height: 100vh;
}

.content {
  flex: 1;
  padding: 20px;
  display: flex;
  flex-direction: column;
}
</style>