<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('preset.previewTitle')"
    width="760px"
    :close-on-click-modal="false"
    :close-on-press-escape="!busy"
    :show-close="!busy"
    destroy-on-close
    @update:model-value="handleDialogVisibility"
  >
    <!-- Step 1: configure -->
    <div v-if="!preview">
      <el-form label-width="110px">
        <el-form-item :label="t('preset.chooseNovel')">
          <el-select
            v-model="novelId"
            :placeholder="t('preset.chooseNovelPlaceholder')"
            :disabled="busy"
            style="width: 100%"
          >
            <el-option
              v-for="n in zhNovels"
              :key="n.id"
              :label="n.title || t('home.unnamed')"
              :value="n.id"
            />
          </el-select>
          <div v-if="zhNovels.length === 0" class="hint">{{ t('preset.noNovels') }}</div>
        </el-form-item>
      </el-form>

      <div v-if="computing" class="calculation-progress">
        <el-progress :percentage="progressPercent" :stroke-width="8" />
        <div class="progress-label">
          {{ progressTotal > 0
            ? t('preset.computingProgress', { processed: progressProcessed, total: progressTotal })
            : t('preset.preparing') }}
        </div>
      </div>
    </div>

    <!-- Step 2: preview -->
    <div v-else>
      <div class="summary">
        {{ t('preset.matchedSummary', { total: preview.totalPresetWords, matched: preview.matchedCount }) }}
        <span class="hint" v-if="preview.items.length === 0">{{ t('preset.noMatch') }}</span>
      </div>

      <el-table :data="preview.items" max-height="340" size="small" style="margin-top: 8px">
        <el-table-column :label="t('preset.word')" width="120" prop="word" />
        <el-table-column :label="t('preset.definition')" min-width="140" prop="definition" show-overflow-tooltip />
        <el-table-column :label="t('preset.example')" min-width="220" prop="exampleSentence" show-overflow-tooltip />
      </el-table>

      <el-form label-width="110px" style="margin-top: 12px">
        <el-form-item :label="t('preset.newBookName')">
          <el-input v-model="newBookName" placeholder="CET4 · {小说}精选" />
        </el-form-item>
      </el-form>
    </div>

    <template #footer>
      <el-button v-if="preview" :disabled="busy" @click="preview = null">{{ t('preset.recalc') }}</el-button>
      <el-button :disabled="busy" @click="closeDialog">{{ t('preset.cancel') }}</el-button>
      <el-button v-if="!preview" type="primary" :disabled="!novelId || busy" :loading="computing" @click="computePreview">
        {{ t('preset.recalc') }}
      </el-button>
      <el-button v-else type="primary" :disabled="preview.items.length === 0 || busy" :loading="importing" @click="doImport">
        {{ t('preset.confirmImport') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { ElMessage } from 'element-plus'
import { t } from '@/i18n'
import type { PresetClonePreview } from '@/types/vocabBook'
import type { Novel } from '@/types/novel'

const props = defineProps<{
  modelValue: boolean
  presetKey: string
  presetName: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'imported', bookId: number): void
}>()

const novels = ref<Novel[]>([])
const zhNovels = computed(() => novels.value.filter((n) => n.language !== 'en'))
const novelId = ref<number | null>(null)
const preview = ref<PresetClonePreview | null>(null)
const computing = ref(false)
const importing = ref(false)
const newBookName = ref('')
const progressPercent = ref(0)
const progressProcessed = ref(0)
const progressTotal = ref(0)
const activeRequestId = ref<string | null>(null)
const busy = computed(() => computing.value || importing.value)

interface PresetCloneProgress {
  requestId: string
  processed: number
  total: number
  percent: number
}

onMounted(async () => {
  try {
    novels.value = await invoke<Novel[]>('get_all_novels')
  } catch {
    /* ignore */
  }
})

watch(
  () => props.modelValue,
  (v) => {
    if (v) {
      preview.value = null
      newBookName.value = ''
      progressPercent.value = 0
      progressProcessed.value = 0
      progressTotal.value = 0
    }
  },
)

function handleDialogVisibility(v: boolean) {
  if (!v && busy.value) return
  emit('update:modelValue', v)
}

function closeDialog() {
  if (busy.value) return
  emit('update:modelValue', false)
}

function createRequestId() {
  return globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random()}`
}

async function computePreview() {
  if (!novelId.value || busy.value) return

  const requestedNovelId = novelId.value
  const requestId = createRequestId()
  let unlistenProgress: UnlistenFn | null = null

  activeRequestId.value = requestId
  computing.value = true
  progressPercent.value = 0
  progressProcessed.value = 0
  progressTotal.value = 0

  try {
    try {
      unlistenProgress = await listen<PresetCloneProgress>('preset-clone-progress', (event) => {
        if (event.payload.requestId !== activeRequestId.value) return
        progressPercent.value = event.payload.percent
        progressProcessed.value = event.payload.processed
        progressTotal.value = event.payload.total
      })
    } catch (e) {
      console.warn('[preset-clone-progress] listen failed:', e)
    }

    const result = await invoke<PresetClonePreview>('preview_preset_clone', {
      presetKey: props.presetKey,
      novelId: requestedNovelId,
      requestId,
    })
    if (activeRequestId.value !== requestId) return
    preview.value = result
    progressPercent.value = 100
    newBookName.value = `${props.presetName} · ${novels.value.find((n) => n.id === requestedNovelId)?.title || ''}精选`
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '计算失败'))
  } finally {
    unlistenProgress?.()
    if (activeRequestId.value === requestId) {
      activeRequestId.value = null
      computing.value = false
    }
  }
}

async function doImport() {
  if (!preview.value || !novelId.value || busy.value) return
  importing.value = true
  try {
    const bookId = await invoke<number>('commit_preset_clone', {
      presetKey: props.presetKey,
      novelId: novelId.value,
      newBookName: newBookName.value,
    })
    ElMessage.success(t('preset.imported', { n: preview.value.items.length }))
    emit('imported', bookId)
    emit('update:modelValue', false)
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '导入失败'))
  } finally {
    importing.value = false
  }
}
</script>

<style scoped>
.hint {
  font-size: 12px;
  color: var(--text-secondary, #909399);
  margin-left: 8px;
}
.summary {
  font-size: 13px;
  color: var(--text-regular, #303133);
}
.calculation-progress {
  margin: 8px 16px 0 110px;
}
.progress-label {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-secondary, #909399);
  text-align: center;
}
</style>
