<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('preset.previewTitle')"
    width="760px"
    :close-on-click-modal="false"
    destroy-on-close
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <!-- Step 1: configure -->
    <div v-if="!preview">
      <el-form label-width="110px">
        <el-form-item :label="t('preset.chooseNovel')">
          <el-select
            v-model="novelId"
            :placeholder="t('preset.chooseNovelPlaceholder')"
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
        <el-form-item :label="t('preset.scale')">
          <el-slider v-model="limit" :min="50" :max="500" :step="50" :marks="{300: '300'}" style="width: 320px" />
        </el-form-item>
      </el-form>
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
      <el-button v-if="preview" @click="preview = null">{{ t('preset.recalc') }}</el-button>
      <el-button @click="emit('update:modelValue', false)">{{ t('preset.cancel') }}</el-button>
      <el-button v-if="!preview" type="primary" :disabled="!novelId" :loading="computing" @click="computePreview">
        {{ t('preset.recalc') }}
      </el-button>
      <el-button v-else type="primary" :disabled="preview.items.length === 0" :loading="importing" @click="doImport">
        {{ t('preset.confirmImport') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
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
const limit = ref(300)
const preview = ref<PresetClonePreview | null>(null)
const computing = ref(false)
const importing = ref(false)
const newBookName = ref('')

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
    }
  },
)

async function computePreview() {
  if (!novelId.value) return
  computing.value = true
  try {
    preview.value = await invoke<PresetClonePreview>('preview_preset_clone', {
      presetKey: props.presetKey,
      novelId: novelId.value,
      limit: limit.value,
    })
    newBookName.value = `${props.presetName} · ${novels.value.find((n) => n.id === novelId.value)?.title || ''}精选`
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '计算失败'))
  } finally {
    computing.value = false
  }
}

async function doImport() {
  if (!preview.value || !novelId.value) return
  importing.value = true
  try {
    const bookId = await invoke<number>('commit_preset_clone', {
      presetKey: props.presetKey,
      novelId: novelId.value,
      limit: preview.value.limit,
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
</style>
