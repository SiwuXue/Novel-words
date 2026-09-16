<template>
  <header class="topbar" :data-tauri-drag-region="isMobile ? undefined : ''">
    <button v-if="showMenu" class="menu-button" :aria-label="t('ui.navigation')" @click="$emit('menu')"><el-icon><Menu /></el-icon></button>
    <div class="topbar-left" :data-tauri-drag-region="isMobile ? undefined : ''">
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">{{ t('nav.home') }}</el-breadcrumb-item>
        <el-breadcrumb-item v-for="crumb in crumbs" :key="(crumb.path || route.path) + crumb.label" :to="crumb.path ? { path: crumb.path } : undefined">{{ crumb.text || t(crumb.label) }}</el-breadcrumb-item>
      </el-breadcrumb>
    </div>
    <div class="topbar-right"><ThemeToggle /><LanguageToggle /><WindowControls /></div>
  </header>
</template>
<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { Menu } from '@element-plus/icons-vue'
import ThemeToggle from '@/components/common/ThemeToggle.vue'
import LanguageToggle from '@/components/common/LanguageToggle.vue'
import WindowControls from './WindowControls.vue'
import { t } from '@/i18n'
import { isMobile } from '@/utils/platform'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { useNovelStore } from '@/stores/novelStore'
import { navigationSection } from '@/utils/workspace'
defineProps<{ showMenu?: boolean }>()
defineEmits<{ menu: [] }>()
const route = useRoute()
const books = useVocabBookStore()
const novels = useNovelStore()
const crumbs = computed(() => {
  const section = navigationSection(route.path)
  const bookName = books.books.find(book => String(book.id) === route.params.id)?.name
  const labels: Record<string, string> = { '/novels':'nav.novels', '/vocabulary':'nav.vocabulary', '/review':'ui.review', '/presets':'nav.presets', '/stats':'nav.stats', '/settings':'nav.settings' }
  if (section === '/') return []
  if (route.path.endsWith('/review') && route.params.id) return [{label:'nav.vocabulary', path:'/vocabulary'}, {label:'ui.detail', text:bookName, path:`/vocabulary/${route.params.id}`}, {label:'ui.review'}]
  const result: { label:string; path?:string; text?:string }[] = [{ label:labels[section], path:route.path === section ? undefined : section }]
  if (route.path !== section) result.push({ label:'ui.detail', text:section === '/vocabulary' ? bookName : section === '/novels' ? novels.currentNovel?.title : undefined })
  return result
})
</script>
<style scoped>
.topbar { display:flex; align-items:center; height:48px; padding:0 16px; gap:12px; background:var(--bg-primary); border-bottom:1px solid var(--border-color); flex-shrink:0; user-select:none; }
.topbar-left { flex:1; min-width:0; height:100%; display:flex; align-items:center; }
.topbar-left :deep(.el-breadcrumb) { overflow:hidden; white-space:nowrap; line-height:1.5; }
.topbar-right { display:flex; align-items:center; gap:8px; }
.menu-button { border:0; background:none; color:var(--text-primary); padding:8px; cursor:pointer; }
@media(max-width:480px) { .topbar { padding:0 8px; gap:6px; } .topbar-left :deep(.el-breadcrumb__item:first-child) { display:none; } }
</style>
