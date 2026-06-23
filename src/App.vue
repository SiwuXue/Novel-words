<template>
  <AppLayout />
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import AppLayout from '@/components/layout/AppLayout.vue'

onMounted(() => {
  // Restore theme from localStorage
  const saved = localStorage.getItem('theme')
  if (saved === 'dark') {
    document.documentElement.classList.add('dark')
  } else if (!saved) {
    // Default to light
    localStorage.setItem('theme', 'light')
  }
})
</script>
