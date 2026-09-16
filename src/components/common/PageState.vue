<template>
  <section class="page-state" :role="error ? 'alert' : 'status'" :aria-busy="loading">
    <span v-if="loading" class="state-spinner" />
    <h3>{{ title }}</h3><p v-if="description">{{ description }}</p>
    <button v-if="error" class="state-action" @click="$emit('retry')">{{ t('ui.retry') }}</button>
    <slot />
  </section>
</template>
<script setup lang="ts">
import { t } from '@/i18n'
defineProps<{ title: string; description?: string; loading?: boolean; error?: boolean }>()
defineEmits<{ retry: [] }>()
</script>
<style scoped>
.page-state { padding:40px 24px; text-align:center; background:var(--bg-primary); border:1px solid var(--border-color); border-radius:8px; display:flex; flex-direction:column; align-items:center; gap:12px; }
p { color:var(--text-secondary); max-width:480px; line-height:1.7; }
.state-action { background:var(--accent-color); color:var(--on-accent); border:0; border-radius:8px; padding:10px 20px; cursor:pointer; }
.state-spinner { width:24px; height:24px; border:2px solid var(--border-color); border-top-color:var(--accent-color); border-radius:50%; animation:spin 1s linear infinite; }
@keyframes spin { to { transform:rotate(360deg); } }
</style>
