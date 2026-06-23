<template>
  <el-switch
    :model-value="isDark"
    :active-icon="Moon"
    :inactive-icon="Sunny"
    @change="toggleTheme"
    inline-prompt
    size="default"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Sunny, Moon } from '@element-plus/icons-vue'

const isDark = computed(() => document.documentElement.classList.contains('dark'))

function toggleTheme() {
  const html = document.documentElement
  if (html.classList.contains('dark')) {
    html.classList.remove('dark')
    localStorage.setItem('theme', 'light')
  } else {
    html.classList.add('dark')
    localStorage.setItem('theme', 'dark')
  }
}
</script>
