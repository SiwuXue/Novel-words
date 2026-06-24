<template>
  <header class="topbar">
    <div class="topbar-left" data-tauri-drag-region>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
        <el-breadcrumb-item v-if="currentPage">{{ currentPage }}</el-breadcrumb-item>
      </el-breadcrumb>
    </div>
    <div class="topbar-right">
      <ThemeToggle />
      <WindowControls />
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import ThemeToggle from '@/components/common/ThemeToggle.vue'
import WindowControls from './WindowControls.vue'

const route = useRoute()

const pageNameMap: Record<string, string> = {
  '/': '',
  '/novels': '小说库',
  '/vocabulary': '词汇本',
  '/settings': '设置',
}

const currentPage = computed(() => {
  const path = route.path
  if (path.startsWith('/novels')) return '小说库'
  if (path.startsWith('/vocabulary')) return '词汇本'
  return pageNameMap[path] || ''
})
</script>

<style scoped>
.topbar {
  display: flex;
  align-items: center;
  height: 40px;
  padding: 0 12px;
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}
.topbar-left {
  flex: 1;
}
.topbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
