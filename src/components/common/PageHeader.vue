<template>
  <header class="page-heading">
    <div class="page-heading-copy"><h2>{{ title }}</h2><p v-if="description">{{ description }}</p><slot name="meta" /></div>
    <div v-if="$slots.default" class="page-heading-actions"><slot /></div>
  </header>
</template>
<script setup lang="ts">
defineProps<{ title: string; description?: string }>()
</script>
<style scoped>
.page-heading { display:flex; justify-content:space-between; align-items:center; gap:16px; margin-bottom:24px; }
h2 { font-size:24px; line-height:1.4; font-weight:650; overflow-wrap:anywhere; }
p { color:var(--text-secondary); font-size:14px; margin-top:6px; }
.page-heading-copy { min-width:0; }
.page-heading-actions { display:flex; flex-wrap:wrap; align-items:center; gap:8px; flex-shrink:0; }
@media(max-width:600px) { .page-heading { align-items:flex-start; flex-direction:column; margin-bottom:16px; } h2 { font-size:21px; } }
</style>
