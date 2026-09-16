<template>
  <div class="app-layout">
    <Sidebar v-if="!isReadingMode && !narrow" :collapsed="collapsed" @collapse="toggleCollapse" />
    <el-dialog v-model="navigationOpen" class="navigation-dialog" :title="t('ui.navigation')" width="min(320px, 92vw)" :show-close="true" :append-to-body="true">
      <Sidebar @navigate="navigationOpen = false" />
    </el-dialog>
    <div class="main-area">
      <TopBar v-if="!isReadingMode" :show-menu="narrow" @menu="navigationOpen = true" />
      <main ref="mainContentRef" class="main-content" :class="{ 'editor-mode': isEditorRoute, 'reading-mode': isReadingMode }">
        <RouterView v-slot="{ Component, route: r }"><Transition name="page-fade" mode="out-in"><component :is="Component" :key="r.path" /></Transition></RouterView>
      </main>
    </div>
  </div>
</template>
<script setup lang="ts">
import { computed, ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { useRoute } from 'vue-router'
import Sidebar from './Sidebar.vue'
import TopBar from './TopBar.vue'
import { t } from '@/i18n'
const route = useRoute()
const mainContentRef = ref<HTMLElement | null>(null)
const collapsed = ref(localStorage.getItem('navigation-collapsed') === 'true')
const narrow = ref(window.innerWidth <= 760)
const navigationOpen = ref(false)
const isEditorRoute = computed(() => route.name === 'NovelEdit' || route.name === 'NovelCreate')
const isReadingMode = computed(() => route.name === 'NovelEdit' && route.query.mode === 'read')
function resize() { narrow.value = window.innerWidth <= 760; if (!narrow.value) navigationOpen.value = false }
function toggleCollapse() { collapsed.value = !collapsed.value; localStorage.setItem('navigation-collapsed', String(collapsed.value)) }
onMounted(() => window.addEventListener('resize', resize))
onBeforeUnmount(() => window.removeEventListener('resize', resize))
watch(() => route.path, () => { navigationOpen.value = false; mainContentRef.value?.scrollTo?.({ top:0 }) })
</script>
<style scoped>
.app-layout { display:flex; height:100dvh; overflow:hidden; }
.main-area { flex:1; display:flex; flex-direction:column; min-width:0; }
.main-content { flex:1; min-height:0; overflow-y:auto; padding:28px; background:var(--bg-secondary); }
.main-content.editor-mode { padding:0; overflow:hidden; }
.main-content.reading-mode { background:var(--bg-primary); }
@media(max-width:1000px) { .main-content { padding:20px; } }
@media(max-width:760px) { .main-content { padding:16px; } }
</style>
