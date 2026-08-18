<template>
  <div class="sidebar-wrapper" :class="{ pinned }">
    <aside class="sidebar">
      <div class="sidebar-header">
        <h1 class="app-title">词阅</h1>
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

      <div class="sidebar-footer">
        <button class="about-link" @click="showAbout = true">
          <el-icon><InfoFilled /></el-icon>
          <span>关于</span>
        </button>
        <button class="pin-btn" :class="{ active: pinned }" @click="togglePin">
          <el-icon><Fold v-if="!pinned" /><Expand v-else /></el-icon>
          <span>{{ pinned ? '收起侧边栏' : '固定侧边栏' }}</span>
        </button>
        <AboutDialog v-model="showAbout" />
      </div>
    </aside>
    <div class="hover-area" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import { HomeFilled, Document, Collection, Setting, InfoFilled, Fold, Expand } from '@element-plus/icons-vue'
import AboutDialog from '@/components/common/AboutDialog.vue'

const route = useRoute()
const activeRoute = computed(() => {
  const path = route.path
  if (path.startsWith('/novels')) return '/novels'
  if (path.startsWith('/vocabulary')) return '/vocabulary'
  return path
})

const showAbout = ref(false)
const pinned = ref(false)

function togglePin() {
  pinned.value = !pinned.value
}
</script>

<style scoped>
.sidebar-wrapper {
  position: relative;
  width: 0;
  height: 100%;
  flex-shrink: 0;
  z-index: 100;
}

.sidebar {
  position: absolute;
  top: 0;
  left: 0;
  width: 200px;
  height: 100%;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  transition: transform 0.25s ease, box-shadow 0.25s ease;
  transform: translateX(-100%);
  pointer-events: none;
}

.hover-area {
  position: absolute;
  top: 0;
  left: 0;
  width: 16px;
  height: 100%;
}

.sidebar-wrapper:hover .sidebar,
.sidebar-wrapper.pinned .sidebar {
  transform: translateX(0);
  box-shadow: 2px 0 12px rgba(0, 0, 0, 0.15);
  pointer-events: auto;
}

.sidebar-wrapper:hover,
.sidebar-wrapper.pinned {
  width: 200px;
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

.sidebar-footer {
  padding: 12px 16px;
  border-top: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.about-link,
.pin-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 12px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary, #909399);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.about-link:hover,
.pin-btn:hover {
  background: var(--bg-secondary, #f5f7fa);
  color: var(--text-regular, #303133);
}

.pin-btn.active {
  color: var(--accent-color, #409eff);
}
</style>
