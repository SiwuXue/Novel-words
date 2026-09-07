<template>
  <div class="app-layout">
    <Sidebar v-if="!isReadingMode" />
    <div class="main-area">
      <TopBar v-if="!isReadingMode" />
      <main
        ref="mainContentRef"
        class="main-content"
        :class="{ 'editor-mode': isEditorRoute, 'home-mode': isHomeRoute, 'reading-mode': isReadingMode }"
      >
        <RouterView v-slot="{ Component, route: r }">
          <Transition name="page-fade" mode="out-in">
            <component :is="Component" :key="r.path" />
          </Transition>
        </RouterView>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import Sidebar from './Sidebar.vue'
import TopBar from './TopBar.vue'

const route = useRoute()
const mainContentRef = ref<HTMLElement | null>(null)
const isEditorRoute = computed(() => route.name === 'NovelEdit')
const isHomeRoute = computed(() => route.name === 'Home')
const isReadingMode = computed(() => route.name === 'NovelEdit' && route.query.mode === 'read')

watch(
  () => route.name,
  async (name) => {
    if (name !== 'Home') return
    await nextTick()
    mainContentRef.value?.scrollTo({ top: 0, left: 0 })
  },
  { immediate: true },
)
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

.main-content.editor-mode {
  padding: 0;
  overflow: hidden;
}

.main-content.reading-mode {
  background: var(--bg-primary);
}

.main-content.home-mode {
  overflow: hidden;
}

@media (max-height: 650px) {
  .main-content.home-mode {
    padding-top: 8px;
    padding-bottom: 8px;
  }
}

@media (max-width: 760px) {
  .main-content {
    padding: 10px;
  }
}

@media (max-width: 420px) {
  .main-content {
    padding: 8px;
  }
}
</style>
