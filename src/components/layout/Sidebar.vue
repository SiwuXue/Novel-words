<template>
  <aside class="sidebar" :class="{ collapsed }" :aria-label="t('ui.navigation')">
    <RouterLink to="/" class="sidebar-brand" @click="$emit('navigate')"><span class="brand-mark">词</span><strong v-if="!collapsed">词阅</strong></RouterLink>
    <nav class="sidebar-nav">
      <RouterLink v-for="item in items" :key="item.path" :to="item.path" :class="{ active: activeRoute === item.path }" :aria-current="activeRoute === item.path ? 'page' : undefined" :title="t(item.label)" @click="$emit('navigate')">
        <el-icon><component :is="item.icon" /></el-icon><span v-if="!collapsed">{{ t(item.label) }}</span>
        <span v-if="item.path === '/review' && dueCount > 0" class="due-badge">{{ dueCount > 99 ? '99+' : dueCount }}</span>
      </RouterLink>
    </nav>
    <footer>
      <RouterLink to="/settings" :class="{ active: activeRoute === '/settings' }" :title="t('nav.settings')" @click="$emit('navigate')"><el-icon><Setting /></el-icon><span v-if="!collapsed">{{ t('nav.settings') }}</span></RouterLink>
      <button @click="showAbout = true" :title="t('nav.about')"><el-icon><InfoFilled /></el-icon><span v-if="!collapsed">{{ t('nav.about') }}</span></button>
      <button class="sidebar-collapse" :aria-label="t(collapsed ? 'ui.expandNav' : 'ui.collapseNav')" @click="$emit('collapse')"><el-icon><Expand v-if="collapsed" /><Fold v-else /></el-icon><span v-if="!collapsed">{{ t('ui.collapseNav') }}</span></button>
    </footer>
    <AboutDialog v-model="showAbout" />
  </aside>
</template>
<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { useRoute } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { HomeFilled, Document, Collection, Setting, InfoFilled, Fold, Expand, DataLine, Files, Reading } from '@element-plus/icons-vue'
import AboutDialog from '@/components/common/AboutDialog.vue'
import { t } from '@/i18n'
import { navigationSection } from '@/utils/workspace'
defineProps<{ collapsed?: boolean }>()
defineEmits<{ collapse: []; navigate: [] }>()
const route = useRoute()
const activeRoute = computed(() => navigationSection(route.path))
const items = [
  { path:'/', label:'nav.home', icon:HomeFilled }, { path:'/novels', label:'nav.novels', icon:Document },
  { path:'/vocabulary', label:'nav.vocabulary', icon:Collection }, { path:'/review', label:'ui.review', icon:Reading },
  { path:'/presets', label:'nav.presets', icon:Files }, { path:'/stats', label:'nav.stats', icon:DataLine },
]
const showAbout = ref(false), dueCount = ref(0)
let timer: number | undefined
async function refresh() { try { dueCount.value = await invoke<number>('get_due_words_count') } catch { /* optional badge */ } }
onMounted(() => { void refresh(); timer = window.setInterval(refresh, 60000) })
onBeforeUnmount(() => window.clearInterval(timer))
watch(() => route.fullPath, refresh)
</script>
<style scoped>
.sidebar { width:192px; flex-shrink:0; background:var(--bg-sidebar); border-right:1px solid var(--border-color); display:flex; flex-direction:column; height:100%; padding:16px 10px; min-height:0; }
.sidebar.collapsed { width:64px; }
.sidebar-brand { display:flex; align-items:center; gap:10px; padding:8px 10px 24px; }
.brand-mark { display:grid; place-items:center; width:28px; height:28px; flex-shrink:0; border-radius:8px; background:var(--accent-color); color:var(--on-accent); font-size:16px; }
.sidebar-brand strong { font-size:18px; font-weight:650; }
.sidebar-nav { display:flex; flex-direction:column; gap:6px; flex:1; min-height:0; overflow-y:auto; }
nav a, footer a, footer button { display:flex; align-items:center; gap:12px; min-height:40px; width:100%; padding:10px 12px; border:0; border-radius:8px; background:transparent; color:var(--text-secondary); font:inherit; font-size:14px; text-align:left; cursor:pointer; }
nav a:hover, footer a:hover, footer button:hover { color:var(--text-primary); background:var(--hover-bg); }
nav a.active, footer a.active { background:var(--accent-light); color:var(--accent-color); font-weight:600; }
.el-icon { flex-shrink:0; font-size:18px; }
footer { flex-shrink:0; padding-top:12px; border-top:1px solid var(--border-color); }
.due-badge { margin-left:auto; font-size:11px; background:var(--accent-color); color:var(--on-accent); border-radius:10px; padding:1px 5px; }
.collapsed .due-badge { position:absolute; transform:translate(12px,-12px); }
@media(max-width:760px) { .sidebar-collapse { display:none; } }
</style>
