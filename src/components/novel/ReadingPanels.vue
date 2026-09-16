<template>
  <div class="reading-panel-triggers">
    <button ref="directoryButton" data-panel="directory" :aria-expanded="active === 'directory'" aria-controls="reading-side-panel" @click="toggle('directory')">{{ t('ui.directory') }}</button>
    <button ref="toolsButton" data-panel="tools" :aria-expanded="active === 'tools'" aria-controls="reading-side-panel" @click="toggle('tools')">{{ t('ui.learningTools') }}</button>
  </div>
  <Teleport to="body" :disabled="!teleport">
    <div v-if="active" class="reading-panel-shade" @click.self="close" @keydown.esc.stop="close">
      <section id="reading-side-panel" ref="panel" class="reading-side-panel" role="dialog" aria-modal="true" :aria-label="t(active === 'directory' ? 'ui.directory' : 'ui.learningTools')" tabindex="-1" @keydown.tab="trapFocus">
        <header><h3>{{ t(active === 'directory' ? 'ui.directory' : 'ui.learningTools') }}</h3><button :aria-label="t('ui.close')" @click="close">×</button></header>
        <div class="reading-panel-content"><slot :name="active" :close="close" /></div>
      </section>
    </div>
  </Teleport>
</template>
<script setup lang="ts">
import { ref, nextTick } from 'vue'
import { t } from '@/i18n'
defineProps<{ teleport?: boolean }>()
const active = ref<'directory' | 'tools' | null>(null)
const panel = ref<HTMLElement | null>(null)
const directoryButton = ref<HTMLButtonElement | null>(null)
const toolsButton = ref<HTMLButtonElement | null>(null)
let lastTrigger: HTMLButtonElement | null = null
async function toggle(name: 'directory' | 'tools') {
  if (active.value === name) return close()
  lastTrigger = name === 'directory' ? directoryButton.value : toolsButton.value
  active.value = name
  await nextTick()
  panel.value?.focus()
}
function close() { active.value = null; lastTrigger?.focus() }
function trapFocus(event: KeyboardEvent) {
  const elements = Array.from(panel.value?.querySelectorAll<HTMLElement>('button, input, select, textarea, a[href], [tabindex="0"]') || []).filter(el => !el.hasAttribute('disabled'))
  const first = elements[0], last = elements[elements.length - 1]
  if (!first) { event.preventDefault(); return }
  if (event.shiftKey && (document.activeElement === first || document.activeElement === panel.value)) { event.preventDefault(); last.focus() }
  else if (!event.shiftKey && (document.activeElement === last || document.activeElement === panel.value)) { event.preventDefault(); first.focus() }
}
</script>
<style>
.reading-panel-triggers { display:flex; gap:6px; }
.reading-panel-triggers button { background:var(--bg-primary); color:var(--text-primary); border:1px solid var(--border-color); border-radius:8px; padding:6px 10px; cursor:pointer; white-space:nowrap; }
.reading-panel-triggers button[aria-expanded="true"] { background:var(--accent-light); color:var(--accent-color); }
.reading-panel-shade { position:fixed; inset:0; background:rgba(16,26,22,.24); z-index:1100; }
.reading-side-panel { position:absolute; right:0; top:0; height:100%; width:min(360px, 92vw); background:var(--bg-primary); color:var(--text-primary); display:flex; flex-direction:column; border-left:1px solid var(--border-color); }
.reading-side-panel header { display:flex; align-items:center; justify-content:space-between; padding:16px; border-bottom:1px solid var(--border-color); }
.reading-side-panel header button { border:0; background:none; color:var(--text-primary); cursor:pointer; font-size:24px; padding:4px 8px; }
.reading-panel-content { flex:1; min-height:0; overflow:auto; }
.reading-tools-form { padding:20px; display:flex; flex-direction:column; gap:16px; }
.reading-tools-form p { color:var(--text-secondary); font-size:14px; line-height:1.7; }
</style>
