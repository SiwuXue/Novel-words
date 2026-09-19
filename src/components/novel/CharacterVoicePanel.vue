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

    <!-- 已识别角色（数据库：AI 抽取 / 手工添加），参与对白分音色 -->
    <div class="cv-group-title">{{ t('characters.savedGroup') }}</div>
    <el-table v-if="savedRows.length" :data="savedRows" size="small" class="cv-table">
      <el-table-column :label="t('characters.name')" width="180">
        <template #default="{ row }">
          <span>{{ row.name }}</span>
          <el-tag v-if="row.source === 'ai'" size="small" class="cv-tag" type="success">
            {{ t('characters.sourceAi') }}
          </el-tag>
          <span v-if="row.count" class="cv-count">{{ t('characters.count', { n: row.count }) }}</span>
          <div v-if="row.aliases.length" class="cv-sub" :title="row.evidence">
            {{ t('characters.aliases') }}：{{ row.aliases.join('、') }}
          </div>
          <div v-else-if="row.evidence" class="cv-sub" :title="row.evidence">{{ row.evidence }}</div>
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

    <!-- 待确认候选（本章启发式识别，尚未入库，不参与朗读） -->
    <div class="cv-group-title">
      {{ t('characters.candidateGroup') }}
      <span class="cv-ai-hint">{{ t('characters.candidateHint') }}</span>
    </div>
    <el-table v-if="candidateRows.length" :data="candidateRows" size="small" class="cv-table">
      <el-table-column :label="t('characters.name')">
        <template #default="{ row }">
          <span>{{ row.name }}</span>
          <span v-if="row.count" class="cv-count">{{ t('characters.count', { n: row.count }) }}</span>
        </template>
      </el-table-column>
      <el-table-column width="150" align="right">
        <template #default="{ row }">
          <el-button size="small" type="primary" plain @click="confirmCandidate(row)">
            {{ t('characters.confirm') }}
          </el-button>
          <el-button size="small" link @click="ignoreCandidate(row)">
            {{ t('characters.ignore') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>
    <p v-else class="cv-empty">{{ t('characters.candidatesEmpty') }}</p>

    <div v-if="ignoredNames.length" class="cv-ignored">
      <span class="cv-ai-hint">
        {{ t('characters.ignored') }} ({{ ignoredNames.length }})：{{ ignoredNames.join('、') }}
      </span>
      <el-button size="small" link @click="restoreIgnored">{{ t('characters.restore') }}</el-button>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { useSettingsStore } from '@/stores/settingsStore'
import { collectCandidates } from '@/utils/dialogue'
import { t } from '@/i18n'
import type { TtsProvider } from '@/utils/ttsPlayer'
import { getVoices, voiceOptionLabel } from '@/utils/ttsVoices'

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

/** 数据库返回的角色行（P1 起带别名/来源/置信度/证据） */
interface SavedCharacter {
  id: number
  name: string
  gender: string
  voice: string
  aliases?: string[]
  source?: string
  confidence?: number
  evidence?: string
}

interface CharRow {
  name: string
  gender: 'male' | 'female' | 'unknown'
  voice: string
  /** 数据库 id；仅从本章识别、尚未确认的行没有 id */
  id: number | null
  count: number
  /** ai / manual = 已入库；candidate = 本章候选未确认 */
  source: 'ai' | 'manual' | 'candidate'
  aliases: string[]
  confidence: number
  evidence: string
}

const visible = ref(false)
const rows = ref<CharRow[]>([])
const dialogueEnabled = ref(false)
const aiDetecting = ref(false)
const voiceOptions = ref<Array<{ id: string; label: string }>>([])
/** 被忽略的候选名（本章内不再展示，可一键全部恢复） */
const ignored = ref<Set<string>>(new Set())

const MODE_KEY = (novelId: number) => `dialogue-voice-enabled-${novelId}`
const IGNORE_KEY = (novelId: number) => `char-ignored-${novelId}`

const aiOk = ref(false)

/** 已入库角色（AI / 手工），参与对白分音色 */
const savedRows = computed(() =>
  rows.value
    .filter((r) => r.id != null)
    .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name)),
)

/** 本章候选：未入库、未被忽略 */
const candidateRows = computed(() =>
  rows.value
    .filter((r) => r.id == null && !ignored.value.has(r.name))
    .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name)),
)

const ignoredNames = computed(() => [...ignored.value])

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

function loadIgnored(): void {
  if (props.novelId == null) return
  try {
    const raw = localStorage.getItem(IGNORE_KEY(props.novelId))
    const list = raw ? (JSON.parse(raw) as unknown) : []
    ignored.value = new Set(Array.isArray(list) ? list.filter((x): x is string => typeof x === 'string') : [])
  } catch {
    ignored.value = new Set()
  }
}

function persistIgnored(): void {
  if (props.novelId == null) return
  if (ignored.value.size === 0) localStorage.removeItem(IGNORE_KEY(props.novelId))
  else localStorage.setItem(IGNORE_KEY(props.novelId), JSON.stringify([...ignored.value]))
}

function open(): void {
  visible.value = true
  if (props.novelId != null) {
    dialogueEnabled.value = localStorage.getItem(MODE_KEY(props.novelId)) === '1'
  }
  loadIgnored()
  void checkAiAvailable()
  void loadVoiceOptions()
  void loadCharacters()
}
defineExpose({ open })

async function loadVoiceOptions(): Promise<void> {
  const provider: TtsProvider = settingsStore.ttsProvider
  try {
    if (provider === 'sapi') {
      // SAPI5 本机音色动态枚举（仅 Windows）
      const list = await invoke<Array<[string, string, string, string]>>('tts_sapi_voices')
      voiceOptions.value = list.map(([id, label, locale]) => ({
        id,
        label: `${label} (${locale})`,
      }))
    } else if (provider === 'system') {
      const voices = window.speechSynthesis ? window.speechSynthesis.getVoices() : []
      voiceOptions.value = voices.map((v) => ({ id: v.name, label: `${v.name} (${v.lang})` }))
    } else {
      const list = getVoices(provider)
      if (provider === 'minimax' && settingsStore.ttsMinimaxKey.trim()) {
        // MiniMax 有 Key 时优先动态拉取（含克隆音色），失败静默回退静态表
        try {
          const rows = await invoke<Array<[string, string, string]>>('tts_voices_v3', {
            provider: 'minimax',
            apiKey: settingsStore.ttsMinimaxKey.trim(),
          })
          if (rows.length) {
            voiceOptions.value = rows.map(([id, label]) => ({ id, label }))
            return
          }
        } catch {
          /* 回退静态表 */
        }
      }
      voiceOptions.value = list.map((v) => ({ id: v.id, label: voiceOptionLabel(v) }))
    }
  } catch {
    voiceOptions.value = []
  }
}

function toRow(c: SavedCharacter, count: number): CharRow {
  return {
    name: c.name,
    gender: (c.gender as CharRow['gender']) ?? 'unknown',
    voice: c.voice ?? '',
    id: c.id,
    count,
    source: c.source === 'ai' ? 'ai' : 'manual',
    aliases: Array.isArray(c.aliases) ? c.aliases : [],
    confidence: typeof c.confidence === 'number' ? c.confidence : 0,
    evidence: c.evidence ?? '',
  }
}

/** 数据库角色 + 本章候选合并展示（候选仅出现在「待确认」分组） */
async function loadCharacters(): Promise<void> {
  const rowsMap = new Map<string, CharRow>()
  const counts = new Map<string, number>()
  for (const sp of collectCandidates(props.chapterText, settingsStore.ttsQuoteStyles)) {
    counts.set(sp.name, sp.count)
  }
  if (props.novelId != null) {
    try {
      const saved = await invoke<SavedCharacter[]>('list_novel_characters', {
        novelId: props.novelId,
      })
      for (const c of saved) {
        rowsMap.set(c.name, toRow(c, counts.get(c.name) ?? 0))
      }
    } catch (e) {
      console.error('[CharacterVoicePanel] load failed:', e)
    }
  }
  // 只把"出现在 ≥2 段对白 + 名字不超长"的候选并入列表，避免单次出现的动词残片（说/笑/感慨）进面板
  for (const [name, count] of counts) {
    if (rowsMap.has(name)) continue
    rowsMap.set(name, {
      name,
      gender: 'unknown',
      voice: '',
      id: null,
      count,
      source: 'candidate',
      aliases: [],
      confidence: 0,
      evidence: '',
    })
  }
  rows.value = [...rowsMap.values()]
}

function emitUpdated(): void {
  const charVoices: Record<string, string> = {}
  const charGenders: Record<string, 'male' | 'female' | 'unknown'> = {}
  for (const r of savedRows.value) {
    // 名字与所有别名都指向同一音色/性别，朗读时按别名命中
    for (const key of [r.name, ...r.aliases]) {
      if (r.voice) charVoices[key] = r.voice
      charGenders[key] = r.gender
    }
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
    const saved = await invoke<{ id: number }>('upsert_novel_character', {
      novelId: props.novelId,
      name: row.name,
      gender: row.gender,
      voice: row.voice,
    })
    row.id = saved.id
    row.source = row.source === 'ai' ? 'ai' : 'manual'
    emitUpdated()
  } catch (e) {
    ElMessage.error(String(e))
  }
}

/** 确认候选 → 入库成为正式角色（此后参与对白分音色） */
async function confirmCandidate(row: CharRow): Promise<void> {
  await saveRow(row)
  if (row.id != null) {
    ElMessage.success(t('characters.saved'))
  }
}

/** 忽略候选：本地屏蔽（不入库，避免污染角色表），可一键恢复 */
function ignoreCandidate(row: CharRow): void {
  rows.value = rows.value.filter((r) => r.name !== row.name || r.id != null)
  ignored.value.add(row.name)
  persistIgnored()
  emitUpdated()
}

function restoreIgnored(): void {
  ignored.value = new Set()
  persistIgnored()
  void loadCharacters()
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
  const row: CharRow = {
    name: trimmed,
    gender: 'unknown',
    voice: '',
    id: null,
    count: 0,
    source: 'manual',
    aliases: [],
    confidence: 0,
    evidence: '',
  }
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
    const saved = await invoke<SavedCharacter[]>('ai_detect_characters', {
      novelId: props.novelId,
      text: props.chapterText,
    })
    aiDetecting.value = false
    // 保留本章统计；AI 结果全部落库，其余候选保留在待确认分组
    const counts = new Map(rows.value.map((r) => [r.name, r.count]))
    const savedNames = new Set(saved.map((c) => c.name))
    const candidates = rows.value.filter((r) => r.id == null && !savedNames.has(r.name))
    rows.value = [
      ...saved.map((c) => toRow(c, counts.get(c.name) ?? 0)),
      ...candidates.map((c) => ({ ...c, count: counts.get(c.name) ?? c.count })),
    ]
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
.cv-group-title {
  margin: 10px 0 6px;
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  display: flex;
  align-items: baseline;
  gap: 8px;
}
.cv-count {
  margin-left: 6px;
  font-size: 11px;
  color: var(--el-text-color-secondary);
}
.cv-tag {
  margin-left: 6px;
}
.cv-sub {
  margin-top: 2px;
  font-size: 11px;
  color: var(--el-text-color-secondary);
  line-height: 1.5;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 170px;
}
.cv-ignored {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}
.cv-empty {
  margin: 8px 0 12px;
  font-size: 13px;
  color: var(--el-text-color-secondary);
  text-align: center;
}
</style>
