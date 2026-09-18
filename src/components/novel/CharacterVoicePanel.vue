<template>
  <el-dialog
    v-model="visible"
    :title="t('characters.title')"
    width="640px"
    :close-on-click-modal="false"
    append-to-body
  >
    <p class="cv-hint">{{ t('characters.hint') }}</p>

    <el-form-item class="cv-mode-row" :label="t('characters.dialogueMode')">
      <el-switch v-model="dialogueEnabled" @change="onDialogueToggle" />
    </el-form-item>

    <div class="cv-actions">
      <el-button
        size="small"
        type="primary"
        plain
        :loading="aiDetecting"
        :disabled="!aiOk"
        @click="onAiDetect"
      >
        {{ aiDetecting ? t('characters.aiDetecting') : t('characters.aiDetect') }}
      </el-button>
      <span v-if="!aiOk" class="cv-ai-hint">{{ t('characters.aiDisabledHint') }}</span>
      <el-button size="small" @click="addManual">{{ t('characters.addChar') }}</el-button>
    </div>

    <el-table v-if="rows.length" :data="rows" size="small" class="cv-table">
      <el-table-column :label="t('characters.name')" width="150">
        <template #default="{ row }">
          <span>{{ row.name }}</span>
          <span v-if="row.count" class="cv-count">{{ t('characters.count', { n: row.count }) }}</span>
        </template>
      </el-table-column>
      <el-table-column :label="t('characters.gender')" width="110">
        <template #default="{ row }">
          <el-select v-model="row.gender" size="small" @change="saveRow(row)">
            <el-option value="male" :label="t('characters.genderMale')" />
            <el-option value="female" :label="t('characters.genderFemale')" />
            <el-option value="unknown" :label="t('characters.genderUnknown')" />
          </el-select>
        </template>
      </el-table-column>
      <el-table-column :label="t('characters.voice')">
        <template #default="{ row }">
          <el-select
            v-model="row.voice"
            size="small"
            filterable
            allow-create
            default-first-option
            @change="saveRow(row)"
          >
            <el-option value="" :label="t('characters.voiceDefault')" />
            <el-option v-for="v in voiceOptions" :key="v.id" :value="v.id" :label="v.label" />
          </el-select>
        </template>
      </el-table-column>
      <el-table-column width="60" align="right">
        <template #default="{ row }">
          <el-button size="small" link type="danger" @click="removeRow(row)">
            {{ t('characters.delete') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>
    <p v-else class="cv-empty">{{ t('characters.detectedEmpty') }}</p>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { useSettingsStore } from '@/stores/settingsStore'
import { collectSpeakers } from '@/utils/dialogue'
import { t } from '@/i18n'
import type { TtsProvider } from '@/utils/ttsPlayer'

const settingsStore = useSettingsStore()

const props = defineProps<{
  novelId: number | null
  chapterText: string
}>()

const emit = defineEmits<{
  (e: 'updated', payload: {
    charVoices: Record<string, string>
    charGenders: Record<string, 'male' | 'female' | 'unknown'>
    dialogueEnabled: boolean
  }): void
}>()

interface CharRow {
  name: string
  gender: 'male' | 'female' | 'unknown'
  voice: string
  /** 数据库 id；仅从本章识别、尚未保存的行没有 id */
  id: number | null
  count: number
}

const visible = ref(false)
const rows = ref<CharRow[]>([])
const dialogueEnabled = ref(false)
const aiDetecting = ref(false)
const voiceOptions = ref<Array<{ id: string; label: string }>>([])

const MODE_KEY = (novelId: number) => `dialogue-voice-enabled-${novelId}`

const aiOk = ref(false)

/** AI 增强是否可用：设置页已启用 + 已配置模型和 Key */
async function checkAiAvailable(): Promise<void> {
  try {
    const s = await invoke<{
      enabled: boolean
      model: string
      api_key_configured: boolean
    }>('get_ai_settings')
    aiOk.value = Boolean(s && s.enabled && s.model && s.api_key_configured)
  } catch {
    aiOk.value = false
  }
}

function open(): void {
  visible.value = true
  if (props.novelId != null) {
    dialogueEnabled.value = localStorage.getItem(MODE_KEY(props.novelId)) === '1'
  }
  void checkAiAvailable()
  void loadVoiceOptions()
  void loadCharacters()
}
defineExpose({ open })

async function loadVoiceOptions(): Promise<void> {
  const provider: TtsProvider = settingsStore.ttsProvider
  try {
    if (provider === 'edge') {
      const list = await invoke<Array<[string, string, string]>>('tts_voices')
      voiceOptions.value = list.map(([id, name, lang]) => ({ id, label: `${name} (${lang})` }))
    } else if (provider === 'dashscope' || provider === 'minimax') {
      const list = await invoke<Array<[string, string, string]>>('tts_cloud_voices', { provider })
      voiceOptions.value = list.map(([id, name, lang]) => ({ id, label: `${name} (${lang})` }))
    } else {
      const voices = window.speechSynthesis ? window.speechSynthesis.getVoices() : []
      voiceOptions.value = voices.map((v) => ({ id: v.name, label: `${v.name} (${v.lang})` }))
    }
  } catch {
    voiceOptions.value = []
  }
}

/** 数据库角色 + 本章识别的说话人合并展示 */
async function loadCharacters(): Promise<void> {
  const rowsMap = new Map<string, CharRow>()
  if (props.novelId != null) {
    try {
      const saved = await invoke<Array<{ id: number; name: string; gender: string; voice: string }>>(
        'list_novel_characters',
        { novelId: props.novelId },
      )
      for (const c of saved) {
        rowsMap.set(c.name, {
          name: c.name,
          gender: (c.gender as CharRow['gender']) ?? 'unknown',
          voice: c.voice ?? '',
          id: c.id,
          count: 0,
        })
      }
    } catch (e) {
      console.error('[CharacterVoicePanel] load failed:', e)
    }
  }
  for (const sp of collectSpeakers(props.chapterText, settingsStore.ttsQuoteStyles)) {
    const existing = rowsMap.get(sp.name)
    if (existing) existing.count = sp.count
    else rowsMap.set(sp.name, { name: sp.name, gender: 'unknown', voice: '', id: null, count: sp.count })
  }
  rows.value = [...rowsMap.values()].sort((a, b) => b.count - a.count || a.name.localeCompare(b.name))
}

function emitUpdated(): void {
  const charVoices: Record<string, string> = {}
  const charGenders: Record<string, 'male' | 'female' | 'unknown'> = {}
  for (const r of rows.value) {
    if (r.voice) charVoices[r.name] = r.voice
    charGenders[r.name] = r.gender
  }
  emit('updated', { charVoices, charGenders, dialogueEnabled: dialogueEnabled.value })
}

function onDialogueToggle(): void {
  if (props.novelId != null) {
    localStorage.setItem(MODE_KEY(props.novelId), dialogueEnabled.value ? '1' : '0')
  }
  emitUpdated()
}

async function saveRow(row: CharRow): Promise<void> {
  if (props.novelId == null) return
  try {
    const saved = await invoke<{ id: number; name: string }>('upsert_novel_character', {
      novelId: props.novelId,
      name: row.name,
      gender: row.gender,
      voice: row.voice,
    })
    row.id = saved.id
    emitUpdated()
  } catch (e) {
    ElMessage.error(String(e))
  }
}

async function removeRow(row: CharRow): Promise<void> {
  rows.value = rows.value.filter((r) => r.name !== row.name)
  if (row.id != null && props.novelId != null) {
    try {
      await invoke('delete_novel_character', { id: row.id })
    } catch (e) {
      ElMessage.error(String(e))
    }
  }
  emitUpdated()
}

function addManual(): void {
  const name = window.prompt(t('characters.name'))
  if (!name || !name.trim()) return
  const trimmed = name.trim()
  if (rows.value.some((r) => r.name === trimmed)) return
  const row: CharRow = { name: trimmed, gender: 'unknown', voice: '', id: null, count: 0 }
  rows.value.push(row)
  void saveRow(row)
}

async function onAiDetect(): Promise<void> {
  if (props.novelId == null) return
  if (!props.chapterText.trim()) {
    ElMessage.warning(t('characters.detectedEmpty'))
    return
  }
  aiDetecting.value = true
  try {
    const saved = await invoke<
      Array<{ id: number; name: string; gender: string; voice: string }>
    >('ai_detect_characters', { novelId: props.novelId, text: props.chapterText })
    aiDetecting.value = false
    // 保留本章统计，刷新数据库结果
    const counts = new Map(rows.value.map((r) => [r.name, r.count]))
    rows.value = saved.map((c) => ({
      name: c.name,
      gender: (c.gender as CharRow['gender']) ?? 'unknown',
      voice: c.voice ?? '',
      id: c.id,
      count: counts.get(c.name) ?? 0,
    }))
    emitUpdated()
  } catch (e) {
    aiDetecting.value = false
    ElMessage.error(String(e))
  }
}
</script>

<style scoped>
.cv-hint {
  margin: 0 0 12px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  line-height: 1.6;
}
.cv-mode-row {
  margin-bottom: 12px;
}
.cv-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}
.cv-ai-hint {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.cv-count {
  margin-left: 6px;
  font-size: 11px;
  color: var(--el-text-color-secondary);
}
.cv-empty {
  margin: 20px 0;
  font-size: 13px;
  color: var(--el-text-color-secondary);
  text-align: center;
}
</style>
