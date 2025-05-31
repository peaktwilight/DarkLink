<template>
  <div class="darklink-container">
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
.darklink-container {
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