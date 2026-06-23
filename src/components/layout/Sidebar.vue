<template>
  <aside class="sidebar">
    <div class="sidebar-header">
      <h1 class="app-title">词阅 PDF 工坊</h1>
    </div>
    <el-menu
      :default-active="activeRoute"
      router
      class="sidebar-menu"
      background-color="transparent"
    >
      <el-menu-item index="/">
        <el-icon><HomeFilled /></el-icon>
        <span>首页</span>
      </el-menu-item>
      <el-menu-item index="/novels">
        <el-icon><Document /></el-icon>
        <span>小说库</span>
      </el-menu-item>
      <el-menu-item index="/vocabulary">
        <el-icon><Collection /></el-icon>
        <span>词汇本</span>
      </el-menu-item>
      <el-menu-item index="/settings">
        <el-icon><Setting /></el-icon>
        <span>设置</span>
      </el-menu-item>
    </el-menu>
  </aside>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { HomeFilled, Document, Collection, Setting } from '@element-plus/icons-vue'

const route = useRoute()
const activeRoute = computed(() => {
  const path = route.path
  if (path.startsWith('/novels')) return '/novels'
  if (path.startsWith('/vocabulary')) return '/vocabulary'
  return path
})
</script>

<style scoped>
.sidebar {
  width: 200px;
  height: 100%;
  flex-shrink: 0;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
}
.sidebar-header {
  padding: 16px;
  border-bottom: 1px solid var(--border-color);
}
.app-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
}
.sidebar-menu {
  flex: 1;
  border-right: none !important;
  padding-top: 8px;
}
.sidebar-menu .el-menu-item {
  color: var(--text-primary);
}
.sidebar-menu .el-menu-item:hover {
  background: var(--bg-secondary);
}
.sidebar-menu .el-menu-item.is-active {
  color: var(--accent-color);
  background: rgba(64, 158, 255, 0.1);
}
</style>
