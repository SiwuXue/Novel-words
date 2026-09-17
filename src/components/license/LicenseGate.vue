<template>
  <div class="license-root">
    <div v-if="opened" class="license-workspace" :inert="!license.authorized" :aria-hidden="!license.authorized ? 'true' : undefined"><slot /></div>
    <div v-if="!license.authorized" ref="lockSurface" class="license-gate" :class="{ 'license-overlay': opened }" :role="opened ? 'dialog' : undefined" :aria-modal="opened ? 'true' : undefined" :aria-label="t('license.activateTitle')">
      <header class="license-titlebar" :data-tauri-drag-region="isMobile ? undefined : ''">
        <span class="license-brand" :data-tauri-drag-region="isMobile ? undefined : ''">{{ t('about.appName') }}</span>
        <div class="license-window-actions"><ThemeToggle /><LanguageToggle /><WindowControls /></div>
      </header>
      <main class="license-gate-content">
        <div class="license-card"><p class="license-eyebrow">{{ t('license.title') }}</p><LicensePanel /></div>
      </main>
    </div>
  </div>
</template>
<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import LicensePanel from './LicensePanel.vue'
import ThemeToggle from '@/components/common/ThemeToggle.vue'
import LanguageToggle from '@/components/common/LanguageToggle.vue'
import WindowControls from '@/components/layout/WindowControls.vue'
import { isMobile } from '@/utils/platform'
import { t } from '@/i18n'
import { useLicenseStore } from '@/stores/licenseStore'
const license = useLicenseStore()
const opened = ref(false)
const lockSurface = ref<HTMLElement>()
let previousFocus: HTMLElement | null = null
function focusables() { return [...(lockSurface.value?.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), [href], [tabindex="0"]') ?? [])].filter(el => el.getAttribute('aria-hidden') !== 'true') }
function trapFocus(event: KeyboardEvent) {
  if (license.authorized || event.key !== 'Tab') return
  const items = focusables(); if (!items.length) return
  const first = items[0]!, last = items[items.length - 1]!
  if (!lockSurface.value?.contains(document.activeElement) || (event.shiftKey && document.activeElement === first) || (!event.shiftKey && document.activeElement === last)) { event.preventDefault(); (event.shiftKey ? last : first).focus() }
}
function shieldWorkspaceShortcuts(event: KeyboardEvent) { if (!license.authorized) event.stopImmediatePropagation() }
watch(() => license.authorized, async active => {
  if (active) { opened.value = true; await nextTick(); previousFocus?.focus(); previousFocus = null }
  else { previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null; await nextTick(); lockSurface.value?.querySelector<HTMLInputElement>('input:not(:disabled)')?.focus() }
}, { immediate: true })
onMounted(() => { document.addEventListener('keydown', trapFocus, true); document.addEventListener('keydown', shieldWorkspaceShortcuts); void license.startMonitoring(); void license.initialize() })
onBeforeUnmount(() => { document.removeEventListener('keydown', trapFocus, true); document.removeEventListener('keydown', shieldWorkspaceShortcuts); license.dispose() })
</script>
<style scoped>
.license-root, .license-workspace { width:100%; height:100dvh; min-width:0; }
.license-gate { display:flex; flex-direction:column; height:100dvh; overflow:hidden; background:var(--bg-secondary); }
.license-overlay { position:fixed; inset:0; z-index:10000; }
.license-titlebar { display:flex; align-items:center; min-height:48px; padding:0 16px; background:var(--bg-primary); border-bottom:1px solid var(--border-color); flex-shrink:0; gap:12px; user-select:none; }
.license-brand { flex:1; align-self:stretch; display:flex; align-items:center; color:var(--text-primary); font-size:14px; font-weight:650; }
.license-window-actions { display:flex; align-items:center; gap:8px; }
.license-gate-content { flex:1; min-height:0; overflow-y:auto; padding:clamp(18px,5vh,56px) 24px; }
.license-card { max-width:560px; width:100%; box-sizing:border-box; margin:0 auto; padding:32px; border:1px solid var(--border-color); border-radius:var(--radius-lg,12px); background:var(--bg-primary); }
.license-eyebrow { color:var(--text-secondary); font-size:12px; margin:0 0 12px; }
@media(max-width:520px) { .license-titlebar { padding:0 8px; } .license-window-actions { gap:4px; } .license-gate-content { padding:18px 12px; } .license-card { padding:22px 18px; } }
@media(max-height:560px) and (min-width:521px) { .license-gate-content { padding:18px 24px; } .license-card { padding:24px; } }
</style>
