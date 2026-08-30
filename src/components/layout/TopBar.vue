<template>
  <header class="topbar" data-tauri-drag-region>
    <div class="topbar-left" data-tauri-drag-region>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">{{ t('nav.home') }}</el-breadcrumb-item>
        <el-breadcrumb-item v-if="currentPage">{{ currentPage }}</el-breadcrumb-item>
      </el-breadcrumb>
    </div>
    <div class="topbar-right">
      <ThemeToggle />
      <LanguageToggle />
      <WindowControls />
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import ThemeToggle from '@/components/common/ThemeToggle.vue'
import LanguageToggle from '@/components/common/LanguageToggle.vue'
import { t } from '@/i18n'
import WindowControls from './WindowControls.vue'

const route = useRoute()

const currentPage = computed(() => {
  const path = route.path
  if (path.startsWith('/novels')) return t('nav.novels')
  if (path.startsWith('/vocabulary')) return t('nav.vocabulary')
  if (path.startsWith('/presets')) return t('nav.presets')
  if (path.startsWith('/stats')) return t('nav.stats')
  if (path.startsWith('/settings')) return t('nav.settings')
  return ''
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
  user-select: none;
  -webkit-user-select: none;
}
.topbar-left {
  flex: 1;
  min-width: 0;
  height: 100%;
  display: flex;
  align-items: center;
}
.topbar-left :deep(.el-breadcrumb) {
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.topbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

@media (max-width: 480px) {
  .topbar {
    padding-left: 8px;
  }
  .topbar-left :deep(.el-breadcrumb__item:first-child) {
    display: none;
  }
}
</style>
