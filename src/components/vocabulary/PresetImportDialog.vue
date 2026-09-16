<template>
  <el-dialog :model-value="modelValue" :title="t('preset.fullImportTitle')" width="min(520px, calc(100vw - 32px))" :close-on-click-modal="false" :close-on-press-escape="!busy" :show-close="!busy" @update:model-value="handleVisibility">
    <p class="import-name">{{ presetName }}</p>
    <p class="import-hint">{{ t('preset.fullImportHint') }}</p>
    <el-form v-if="!result" label-position="top">
      <el-form-item :label="t('preset.newBookName')">
        <el-input v-model="newBookName" :placeholder="presetName" :aria-label="t('preset.newBookName')" :disabled="busy" maxlength="200" />
      </el-form-item>
    </el-form>
    <div v-if="busy" class="import-progress" role="status" aria-live="polite">
      <el-progress :percentage="percent" />
      <p>{{ total ? t('preset.importProgress', { processed, total }) : t('preset.loadingList') }}</p>
    </div>
    <el-alert v-if="error" :title="error" type="error" :closable="false" show-icon />
    <el-alert v-if="result" :title="t('vocabImport.summary', { newWords: result.newWords, inherited: result.inherited, skipped: result.skipped })" type="success" :closable="false" show-icon />
    <template #footer>
      <el-button :disabled="busy" @click="handleVisibility(false)">{{ result ? t('ui.close') : t('preset.cancel') }}</el-button>
      <el-button v-if="result" type="primary" @click="openBook">{{ t('preset.openBook') }}</el-button>
      <el-button v-else type="primary" :loading="busy" :disabled="busy" @click="importBook">{{ error ? t('ui.retry') : t('preset.fullImport') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { t } from '@/i18n'
import type { VocabImportResult } from '@/types/vocabBook'

const props = defineProps<{ modelValue: boolean; presetKey: string; presetName: string }>()
const emit = defineEmits<{ (e: 'update:modelValue', value: boolean): void; (e: 'imported', bookId: number): void }>()
const newBookName = ref('')
const busy = ref(false)
const result = ref<VocabImportResult | null>(null)
const error = ref('')
const percent = ref(0)
const processed = ref(0)
const total = ref(0)
let activeRequestId: string | null = null
let unlisten: UnlistenFn | null = null
let disposed = false
interface ImportProgress { requestId: string; processed: number; total: number; percent: number; stage: 'loading' | 'importing' }

watch(() => props.modelValue, value => {
  if (value && !busy.value) { newBookName.value = ''; result.value = null; error.value = ''; percent.value = 0; processed.value = 0; total.value = 0 }
})
function handleVisibility(value: boolean) { if (!busy.value) emit('update:modelValue', value) }
function openBook() {
  if (!result.value) return
  emit('imported', result.value.bookId)
  emit('update:modelValue', false)
}
async function importBook() {
  if (busy.value || !props.presetKey) return
  const requestId = globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random()}`
  activeRequestId = requestId; busy.value = true; error.value = ''; percent.value = 0; processed.value = 0; total.value = 0
  try {
    const stop = await listen<ImportProgress>('preset-import-progress', event => {
      if (disposed || event.payload.requestId !== activeRequestId) return
      percent.value = Math.max(0, Math.min(100, event.payload.percent))
      processed.value = event.payload.processed; total.value = event.payload.total
    })
    if (disposed || activeRequestId !== requestId) { stop(); return }
    unlisten = stop
    const imported = await invoke<VocabImportResult>('import_preset_vocab_book', { presetKey: props.presetKey, newBookName: newBookName.value.trim() || null, requestId })
    if (disposed || activeRequestId !== requestId) return
    result.value = imported; percent.value = 100
  } catch (e) {
    if (!disposed && activeRequestId === requestId) error.value = e instanceof Error ? e.message : String(e)
  } finally {
    unlisten?.(); unlisten = null
    if (activeRequestId === requestId) { activeRequestId = null; busy.value = false }
  }
}
onBeforeUnmount(() => { disposed = true; activeRequestId = null; unlisten?.(); unlisten = null })
</script>

<style scoped>
.import-name { font-weight: 600; color: var(--text-primary); margin: 0 0 8px; overflow-wrap: anywhere; }
.import-hint, .import-progress p { font-size: 13px; color: var(--text-secondary); line-height: 1.6; }
.import-progress { margin-bottom: 16px; }
:deep(.el-dialog__footer) { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 8px; }
</style>
