<template>
  <el-switch
    :aria-label="t('settings.theme')"
    :model-value="settingsStore.theme === 'dark'"
    :active-icon="Moon"
    :inactive-icon="Sunny"
    @change="toggleTheme"
    inline-prompt
    size="default"
  />
</template>

<script setup lang="ts">
import { Sunny, Moon } from '@element-plus/icons-vue'
import { useSettingsStore } from '@/stores/settingsStore'
import { t } from '@/i18n'

const settingsStore = useSettingsStore()

function toggleTheme() {
  const next = settingsStore.theme === 'dark' ? 'light' : 'dark'
  settingsStore.setTheme(next)
}
</script>
