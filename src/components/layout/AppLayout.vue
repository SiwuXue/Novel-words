<template>
  <div class="app-layout">
    <Sidebar />
    <div class="main-area">
      <TopBar />
      <main class="main-content">
        <RouterView />
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import Sidebar from './Sidebar.vue'
import TopBar from './TopBar.vue'
</script>

<style scoped>
.app-layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
}
.main-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
  background: var(--bg-secondary);
}
</style>
