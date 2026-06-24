<template>
  <AppLayout />
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import AppLayout from '@/components/layout/AppLayout.vue'
import { useSettingsStore } from '@/stores/settingsStore'

onMounted(async () => {
  // Instant theme from localStorage (no flash)
  const saved = localStorage.getItem('theme')
  if (saved === 'dark') {
    document.documentElement.classList.add('dark')
  } else if (!saved) {
    localStorage.setItem('theme', 'light')
  }

  // Load persisted settings from DB (reconcile theme if DB disagrees)
  const settingsStore = useSettingsStore()
  await settingsStore.load()

  // If localStorage and store disagree, DB value wins
  const localTheme = localStorage.getItem('theme')
  if (settingsStore.theme && settingsStore.theme !== localTheme) {
    if (settingsStore.theme === 'dark') {
      document.documentElement.classList.add('dark')
    } else {
      document.documentElement.classList.remove('dark')
    }
    localStorage.setItem('theme', settingsStore.theme)
  }
})
</script>
