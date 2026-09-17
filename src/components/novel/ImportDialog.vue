<template>
  <el-dialog
    v-model="visible"
    :title="t('import.title')"
    width="min(680px, calc(100vw - 32px))"
    :close-on-click-modal="false"
    destroy-on-close
    @closed="emit('close')"
  >
    <!-- Step 1: choose file -->
    <div
      v-show="step === 1"
      class="import-step drop-step"
      @dragenter.prevent="onDragEnter"
      @dragover.prevent="onDragOver"
      @drop.prevent="onDrop"
    >
      <button
        type="button"
        class="drop-zone"
        :aria-label="t('import.clickSelect')"
        :disabled="analyzing"
        :class="{ 'is-drag-over': isDragOver }"
        @click="selectFile"
      >
        <el-icon :size="48" color="var(--text-secondary)"><FolderOpened /></el-icon>
        <span class="file-select-label">{{ t('import.clickSelect') }}</span>
        <span class="hint">{{ t('import.dragHere') }}</span>
      </button>
      <div v-if="filePath" class="selected-file">
        <el-tag type="info" size="small">{{ fileName }}</el-tag>
      </div>
      <div class="custom-rule-row">
        <el-input
          v-model="customPattern"
          size="small"
          clearable
          :placeholder="t('import.customRulePlaceholder')"
        >
          <template #prepend>{{ t('import.customRule') }}</template>
        </el-input>
        <div class="custom-rule-hint">{{ t('import.customRuleHint') }}</div>
      </div>
      <div v-if="analyzing" class="analyze-progress">
        <el-progress :percentage="importPercent" :stroke-width="8" :status="importPercent >= 100 ? 'success' : undefined" />
        <div class="analyze-msg">{{ importMessage }}</div>
      </div>
      <div class="step-footer">
        <el-button @click="visible = false" :disabled="analyzing">{{ t('import.cancel') }}</el-button>
        <el-button type="primary" :disabled="!filePath" :loading="analyzing" @click="analyzeFile">
          {{ t('import.analyze') }}
        </el-button>
      </div>
    </div>

    <!-- Step 2: preview chapters & confirm -->
    <div v-show="step === 2" class="import-step">
      <el-descriptions :column="2" border size="small">
        <el-descriptions-item :label="t('import.detectedTitle')">
          {{ result?.detectedTitle || '—' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('import.chapters')">
          {{ result?.chapters.length || 0 }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('import.chars')">
          {{ (result?.rawText.length || 0).toLocaleString() }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('import.cleanedChars')">
          {{ (result?.cleanedText.length || 0).toLocaleString() }}
        </el-descriptions-item>
      </el-descriptions>

      <div class="chapter-preview">
        <h4>{{ t('import.previewTitle') }}</h4>
        <ul>
          <li v-for="(ch, i) in result?.chapters.slice(0, 10)" :key="i">
            <span class="ch-title">{{ ch.title }}</span>
            <span class="ch-len">{{ ch.content.length.toLocaleString() }}</span>
          </li>
        </ul>
      </div>

      <div class="step-footer">
        <el-button @click="step = 1">{{ t('import.back') }}</el-button>
        <el-button @click="visible = false">{{ t('import.cancel') }}</el-button>
        <el-button type="primary" :loading="importing" @click="handleImport">
          {{ t('import.confirmImport') }}
        </el-button>
      </div>
    </div>

    <!-- Error state -->
    <div v-show="step === 3" class="import-step">
      <el-result icon="error" :title="t('import.importFailed')" :sub-title="errorMsg">
        <template #extra>
          <el-button type="primary" @click="step = 1">{{ t('import.retry') }}</el-button>
        </template>
      </el-result>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { FolderOpened } from '@element-plus/icons-vue'
import type { ImportResult } from '@/types/novel'
import { t } from '@/i18n'

const props = defineProps<{
  /** If provided, jump straight to analyzing this file (used by drag-drop). */
  initialPath?: string
}>()

const emit = defineEmits<{
  (e: 'confirm', result: ImportResult, filePath: string): void
  (e: 'close'): void
}>()

const visible = ref(true)
const step = ref(1)
const filePath = ref('')
const analyzing = ref(false)
const importing = ref(false)
const result = ref<ImportResult | null>(null)
const errorMsg = ref('')
const importPercent = ref(0)
const importMessage = ref('')
/** 自定义章节规则（正则，可选），localStorage 记住上次输入 */
const customPattern = ref(localStorage.getItem('custom-chapter-pattern') ?? '')

watch(customPattern, (v) => {
  localStorage.setItem('custom-chapter-pattern', v)
})

let unlistenProgress: UnlistenFn | null = null
onBeforeUnmount(() => {
  unlistenProgress?.()
})

const fileName = computed(() => {
  if (!filePath.value) return ''
  const parts = filePath.value.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1]
})

watch(
  () => props.initialPath,
  (p) => {
    if (p && p !== filePath.value) {
      filePath.value = p
      // Auto-analyze so the user lands on the preview step.
      void analyzeFile()
    }
  },
  { immediate: true },
)

const isDragOver = ref(false)

function onDragEnter() {
  isDragOver.value = true
}
function onDragOver() {
  isDragOver.value = true
}
function onDrop(e: DragEvent) {
  isDragOver.value = false
  const file = e.dataTransfer?.files?.[0] as (File & { path?: string }) | undefined
  const path = file?.path
  if (path) {
    filePath.value = path
    void analyzeFile()
  }
}

async function selectFile() {
  const selected = await open({
    multiple: false,
    filters: [
      { name: '小说文件', extensions: ['txt', 'md', 'text', 'epub', 'fb2', 'pdf'] },
      { name: '文本文件', extensions: ['txt', 'md', 'text'] },
      { name: '电子书', extensions: ['epub', 'fb2', 'pdf'] },
      { name: '所有文件', extensions: ['*'] },
    ],
  })
  if (selected && typeof selected === 'string') {
    filePath.value = selected
  }
}

async function analyzeFile() {
  if (!filePath.value) return
  const pattern = customPattern.value.trim()
  if (pattern) {
    try {
      // eslint-disable-next-line no-new
      new RegExp(pattern)
    } catch {
      ElMessage.warning(t('import.customRuleInvalid'))
      return
    }
  }
  analyzing.value = true
  importPercent.value = 0
  importMessage.value = '正在准备…'
  try {
    unlistenProgress = await listen<{ percent: number; message: string }>(
      'import-progress',
      (event) => {
        importPercent.value = event.payload.percent
        importMessage.value = event.payload.message
      },
    )
    const r = await invoke<ImportResult>('import_file', {
      path: filePath.value,
      customPattern: pattern || null,
    })
    result.value = r
    importPercent.value = 100
    step.value = 2
  } catch (e: any) {
    console.error('[import_file] failed:', e)
    errorMsg.value = String(e?.message || e || '未知错误')
    step.value = 3
  } finally {
    analyzing.value = false
  }
}

async function handleImport() {
  if (!result.value) return
  importing.value = true
  try {
    emit('confirm', result.value, filePath.value)
    ElMessage.success(`已导入 ${result.value.detectedTitle || '小说'}，${result.value.chapters.length} 章`)
  } finally {
    importing.value = false
  }
}

</script>

<style scoped>
.import-step {
  min-height: 320px;
  padding: 20px 0;
}
.drop-zone {
  width: 100%;
  background: transparent;
  color: inherit;
  border: 2px dashed var(--border-color, #dcdfe6);
  border-radius: 8px;
  padding: 48px;
  text-align: center;
  cursor: pointer;
  transition: border-color 0.2s;
}
.drop-zone:hover {
  border-color: var(--accent-color, #409eff);
}
.drop-zone.is-drag-over {
  border-color: var(--accent-color, #409eff);
  background: var(--accent-light, #ecf5ff);
  transform: scale(1.01);
  transition: border-color 0.15s, background 0.15s, transform 0.15s;
}
.drop-step {
  position: relative;
}
.drop-zone .file-select-label, .drop-zone .hint {
  display: block;
  margin: 8px 0 0;
  color: var(--text-regular);
}
.drop-zone .hint {
  color: var(--text-secondary);
  font-size: 13px;
}
.selected-file {
  margin-top: 12px;
  text-align: center;
}
.custom-rule-row {
  margin-top: 14px;
}
.custom-rule-hint {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}
.analyze-progress {
  margin-top: 16px;
  padding: 0 8px;
}
.analyze-msg {
  margin-top: 6px;
  font-size: 13px;
  color: var(--text-secondary);
  text-align: center;
}
.chapter-preview {
  margin-top: 16px;
  max-height: 280px;
  overflow-y: auto;
}
.chapter-preview h4 {
  margin: 0 0 8px;
  font-size: 14px;
  color: var(--text-secondary);
}
.chapter-preview ul {
  list-style: none;
  padding: 0;
  margin: 0;
}
.chapter-preview li {
  display: flex;
  justify-content: space-between;
  padding: 6px 8px;
  border-radius: 4px;
  font-size: 13px;
}
.chapter-preview li:nth-child(odd) {
  background: var(--bg-secondary);
}
.ch-title {
  color: var(--text-regular);
}
.ch-len {
  color: var(--text-secondary);
  flex-shrink: 0;
}
.import-error {
  text-align: center;
  padding: 32px;
}
.import-error p {
  margin: 16px 0;
  color: var(--danger-color, #f56c6c);
}
.step-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--border-color, #ebeef5);
}

@media (max-width: 560px) {
  .import-step {
    min-height: 0;
    padding: 8px 0;
  }
  .drop-zone {
    padding: 32px 12px;
  }
  :deep(.el-descriptions__body .el-descriptions__table) {
    table-layout: fixed;
  }
  :deep(.el-descriptions__cell) {
    word-break: break-word;
  }
  .step-footer {
    flex-wrap: wrap;
  }
  .step-footer .el-button {
    flex: 1 1 100px;
    margin-left: 0;
  }
}
</style>
