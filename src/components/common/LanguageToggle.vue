<template>
  <button
    type="button"
    class="language-toggle"
    :aria-label="switchLabel"
    :title="switchLabel"
    @click="toggleLocale"
  >
    <span :class="{ active: currentLocale === 'zh' }">中</span>
    <span class="divider">/</span>
    <span :class="{ active: currentLocale === 'en' }">EN</span>
  </button>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { currentLocale, setLocale } from '@/i18n'

const switchLabel = computed(() =>
  currentLocale.value === 'zh' ? '切换为 English' : 'Switch to 中文',
)

function toggleLocale() {
  void setLocale(currentLocale.value === 'zh' ? 'en' : 'zh')
}
</script>

<style scoped>
.language-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 3px;
  height: 24px;
  min-width: 48px;
  padding: 0 6px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  color: var(--text-placeholder, #a8abb2);
  background: var(--bg-secondary, #f5f7fa);
  font: inherit;
  font-size: 11px;
  line-height: 1;
  cursor: pointer;
  transition: border-color 0.2s, background 0.2s;
}

.language-toggle:hover {
  border-color: var(--accent-color, #409eff);
  background: var(--accent-light, #ecf5ff);
}

.language-toggle:focus-visible {
  outline: 2px solid var(--accent-color, #409eff);
  outline-offset: 1px;
}

.language-toggle .active {
  color: var(--accent-color, #409eff);
  font-weight: 700;
}

.divider {
  opacity: 0.55;
}
</style>
