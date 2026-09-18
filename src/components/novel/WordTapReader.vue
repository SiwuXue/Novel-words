<template>
  <div class="word-tap-reader">
    <!-- 计数条 + 短语操作 + 读完操作 -->
    <div class="wt-toolbar">
      <span class="wt-tts-controls">
        <el-button
          size="small"
          :type="ttsState === 'idle' ? 'default' : 'warning'"
          :disabled="finishing"
          @click="toggleTts"
        >
          {{ ttsLabel }}
        </el-button>
        <el-button v-if="ttsState !== 'idle'" size="small" @click="stopTts">
          {{ t('wordTap.ttsStop') }}
        </el-button>
        <span v-if="ttsState !== 'idle'" class="wt-tts-progress" role="status">
          {{ ttsProgress }}
        </span>
        <el-button v-if="novelId" size="small" @click="charPanel?.open()">
          {{ t('characters.open') }}
        </el-button>
      </span>
      <span class="wt-counts" role="status" :aria-label="t('wordTap.countsLabel')">
        <span class="wt-c wt-c-new">{{ t('wordTap.new') }} {{ counts.new }}</span>
        <span class="wt-c wt-c-unknown">{{ t('wordTap.unknown') }} {{ counts.unknown }}</span>
        <span class="wt-c wt-c-familiar">{{ t('wordTap.familiar') }} {{ counts.familiar }}</span>
        <span class="wt-c wt-c-mastered">{{ t('wordTap.mastered') }} {{ counts.mastered }}</span>
        <span class="wt-c wt-c-ignore">{{ t('wordTap.ignore') }} {{ counts.ignore }}</span>
      </span>

      <!-- 短语选择保存条 -->
      <span v-if="phraseDraft" class="wt-phrase-bar">
        <span class="wt-phrase-text">"{{ phraseDraft.text }}"</span>
        <el-button size="small" type="primary" :loading="savingPhrase" @click="savePhrase">
          {{ t('wordTap.phraseSave') }}
        </el-button>
        <el-button size="small" @click="clearSelection">{{ t('wordTap.phraseCancel') }}</el-button>
      </span>

      <!-- 朗读音频 -->
      <span v-if="novelId" class="wt-audio">
        <audio v-if="audioSrc" controls :src="audioSrc" preload="metadata" class="wt-audio-player" />
        <el-button size="small" link type="primary" @click="pickAudio">
          {{ audioPath ? t('wordTap.audioChange') : t('wordTap.audioSelect') }}
        </el-button>
        <el-button v-if="audioPath" size="small" link type="danger" @click="clearAudio">
          {{ t('wordTap.audioClear') }}
        </el-button>
      </span>

      <el-button v-if="!phraseDraft" size="small" :loading="finishing" @click="finishChapter">
        {{ t('wordTap.finish') }}
      </el-button>
    </div>

    <!-- 逐词正文：滚动懒渲染 -->
    <div ref="scrollRef" class="wt-scroll" @scroll="onScroll">
      <template v-for="{ block, index } in visibleBlocksWithIndex" :key="index">
        <component :is="block.tag" class="wt-block" :data-block="index">
          <template v-for="(token, ti) in block.tokens" :key="ti">
            <span
              v-if="token.type === 'word' || token.type === 'phrase'"
              class="wt-word"
              :class="[stateClass(states[token.key ?? '']), isSpeaking(token as SerialToken) ? 'wt-speaking' : '']"
              :data-key="token.key"
              :data-tok="token.serial"
              @click="onWordClick($event, token)"
            >{{ token.text }}</span>
            <template v-else>{{ token.text }}</template>
          </template>
        </component>
      </template>
      <p v-if="visibleCount < displayedBlocks.length" class="wt-loading-more">{{ t('ui.loading') }}</p>
    </div>

    <!-- 查词弹窗（复用，含快捷标记） -->
    <DictLookupPopover
      v-if="popover.visible"
      :text="popover.text"
      :position="popover.position"
      :novel-id="novelId"
      :chapter-id="chapterId"
      quick-mark
      @close="popover.visible = false"
      @marked="onMarked"
    />

    <!-- 角色音色面板（多角色对白分音色） -->
    <CharacterVoicePanel
      v-if="novelId"
      ref="charPanel"
      :novel-id="novelId"
      :chapter-text="fullText"
      @updated="onCharVoicesUpdated"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { open as openFileDialog } from '@tauri-apps/plugin-dialog'
import DictLookupPopover from './DictLookupPopover.vue'
import CharacterVoicePanel from './CharacterVoicePanel.vue'
import { useDictionaryStore } from '@/stores/dictionaryStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { speakWord } from '@/utils/speech'
import { ttsPlayer, splitSentenceSpans, type SentenceSpan } from '@/utils/ttsPlayer'
import { buildVoiceOverrides } from '@/utils/dialogue'
import {
  parseWordTapBlocks,
  collectWordKeys,
  mergePhrases,
  stateClass,
  wordKey,
  type WordTapBlock,
  type WordToken,
  type PhraseToken,
} from '@/utils/wordTap'
import type { Proficiency } from '@/types/vocabWord'
import { t } from '@/i18n'

const props = defineProps<{
  content: string
  novelId: number | null
  chapterId: number | null
}>()

const emit = defineEmits<{
  (e: 'mark', payload: { word: string; proficiency: Proficiency }): void
  (e: 'ttsNext'): void
}>()

/** 单次渲染的块数（滚动到底自动加载） */
const PAGE_SIZE = 60
const visibleCount = ref(PAGE_SIZE)
const scrollRef = ref<HTMLElement | null>(null)
const finishing = ref(false)
const savingPhrase = ref(false)
const disposed = ref(false)

const dictStore = useDictionaryStore()
const settingsStore = useSettingsStore()

const blocks = computed<WordTapBlock[]>(() => parseWordTapBlocks(props.content))

// ---------- 状态与短语 ----------

/** 归一化 key → 熟练度（含短语 key） */
const states = ref<Record<string, Proficiency>>({})
/** 已保存短语 key 集合（key 含空格） */
const phrases = ref<Set<string>>(new Set())
/** 唯一词 key → 首次出现的原文 */
const sampleWords = ref<Record<string, string>>({})
/** 今日新建的个人词条数（用于每日新词上限提醒） */
const todayNewCount = ref(0)

const displayedBlocks = computed<WordTapBlock[]>(() =>
  mergePhrases(blocks.value, phrases.value),
)

interface SerialToken {
  type: 'word' | 'text' | 'phrase'
  text: string
  key?: string
  serial?: number
  start: number
  end: number
}

/** 带全局 token 序号与字符偏移的渲染块（朗读高亮映射用） */
const renderInfo = computed(() => {
  let serial = 0
  let offset = 0
  const parts: string[] = []
  const blocks = displayedBlocks.value.map((block) => ({
    tag: block.tag,
    tokens: block.tokens.map<SerialToken>((t) => {
      const start = offset
      offset += t.text.length
      parts.push(t.text)
      if (t.type === 'word' || t.type === 'phrase') {
        return { type: t.type, text: t.text, key: t.key, serial: serial++, start, end: offset }
      }
      return { type: 'text', text: t.text, start, end: offset }
    }),
  }))
  parts.push('\n\n')
  return { blocks, fullText: parts.join(''), tokenCount: serial }
})
const fullText = computed(() => renderInfo.value.fullText)
const visibleBlocks = computed(() => renderInfo.value.blocks.slice(0, visibleCount.value))
const visibleBlocksWithIndex = computed(() =>
  visibleBlocks.value.map((block, index) => ({ block, index })),
)

const popover = ref<{
  visible: boolean
  text: string
  position: { x: number; y: number }
}>({ visible: false, text: '', position: { x: 0, y: 0 } })

// ---------- 状态加载 ----------

async function loadStates() {
  const keys = collectWordKeys(blocks.value)
  const samples: Record<string, string> = {}
  for (const block of blocks.value) {
    for (const token of block.tokens) {
      if (token.type === 'word' && !samples[token.key]) samples[token.key] = token.text
    }
  }
  sampleWords.value = samples
  const fetched: Record<string, Proficiency> = {}
  for (let i = 0; i < keys.length; i += 300) {
    if (disposed.value) return
    try {
      const list = await invoke<
        Array<{ key: string; word: string; proficiency: string }>
      >('lookup_word_tap_states', { words: keys.slice(i, i + 300) })
      for (const item of list) {
        fetched[item.key] = item.proficiency as Proficiency
      }
    } catch (e) {
      console.error('[wordTap] load states failed:', e)
      return
    }
  }
  // 短语状态一并载入（供合并渲染着色）
  try {
    const phraseList = await invoke<
      Array<{ key: string; word: string; proficiency: string }>
    >('lookup_word_tap_phrases')
    for (const item of phraseList) {
      fetched[item.key] = item.proficiency as Proficiency
    }
  } catch (e) {
    console.error('[wordTap] load phrases failed:', e)
  }
  if (!disposed.value) states.value = fetched
  try {
    todayNewCount.value = await invoke<number>('word_tap_today_new_count')
  } catch {
    todayNewCount.value = 0
  }
}

/** 标记了此前未收录的词 → 计入今日新词，超过上限时提醒（不阻断） */
function noteNewWordIfNeeded(key: string) {
  if (states.value[key]) return
  const limit = settingsStore.dailyNewWordLimit
  if (limit <= 0) return
  todayNewCount.value += 1
  if (todayNewCount.value > limit) {
    ElMessage.warning(t('wordTap.dailyLimitExceeded', { n: limit }))
  }
}

watch(
  () => props.content,
  () => {
    visibleCount.value = PAGE_SIZE
    if (scrollRef.value) scrollRef.value.scrollTop = 0
    void loadStates()
  },
  { immediate: true },
)

// ---------- 懒渲染 ----------

function onScroll() {
  const el = scrollRef.value
  if (!el) return
  if (el.scrollTop + el.clientHeight >= el.scrollHeight - 120) {
    if (visibleCount.value < displayedBlocks.value.length) {
      visibleCount.value = Math.min(visibleCount.value + PAGE_SIZE, displayedBlocks.value.length)
    }
  }
}

// ---------- 点击查词 / 短语选择 ----------

/** 上一次点击的单词元素（用于相邻判定） */
let lastWordEl: HTMLElement | null = null
/** 短语草稿：连续相邻点选的元素与词形 */
const selection = ref<{ els: HTMLElement[]; keys: string[]; texts: string[] } | null>(null)
const phraseDraft = computed(() => {
  if (!selection.value || selection.value.keys.length < 2) return null
  return { text: selection.value.texts.join(' '), count: selection.value.keys.length }
})

/** a 与 b 之间仅有空白文本节点 → 视为相邻 */
function isAdjacent(a: HTMLElement, b: HTMLElement): boolean {
  let node: Node | null = a.nextSibling
  while (node && node !== b) {
    if (node.nodeType === Node.TEXT_NODE) {
      if (!/^\s*$/.test(node.textContent ?? '')) return false
    } else {
      return false
    }
    node = node.nextSibling
  }
  return node === b
}

function clearSelection() {
  selection.value?.els.forEach((el) => el.classList.remove('wt-selecting'))
  selection.value = null
}

function onWordClick(e: MouseEvent, token: WordToken | PhraseToken | SerialToken) {
  const el = e.currentTarget as HTMLElement
  // 已保存短语：直接打开弹窗（快捷标记/收藏用），不参与组词
  if (token.type === 'phrase') {
    clearSelection()
    lastWordEl = null
    openPopover(e, token.text)
    return
  }
  // 相邻点击 → 扩展短语选择
  if (lastWordEl && isAdjacent(lastWordEl, el)) {
    if (!selection.value) {
      const first = lastWordEl
      first.classList.add('wt-selecting')
      const firstKey = first.dataset.key ?? ''
      const firstText = first.textContent ?? ''
      selection.value = { els: [first], keys: [firstKey], texts: [firstText] }
    }
    el.classList.add('wt-selecting')
    selection.value.els.push(el)
    selection.value.keys.push(token.key ?? wordKey(token.text))
    selection.value.texts.push(token.text)
    lastWordEl = el
    popover.value = { ...popover.value, visible: false }
    return
  }
  // 非相邻：重置选择并查词
  clearSelection()
  lastWordEl = el
  openPopover(e, token.text)
}

function openPopover(e: MouseEvent, text: string) {
  // 弹窗出现在点击位置（组件内部会自动防溢出）
  popover.value = {
    visible: true,
    text,
    position: { x: e.clientX, y: e.clientY + 8 },
  }
  void dictStore.lookupAuto(text)
  // 点击即朗读，无需再点弹窗里的朗读按钮
  speakWord(text, settingsStore.speechAccent)
}

// ---------- 快捷标记 ----------

function onMarked(payload: { word: string; proficiency: Proficiency }) {
  const key = wordKey(payload.word)
  const isNew = !states.value[key]
  states.value = { ...states.value, [key]: payload.proficiency }
  if (isNew) noteNewWordIfNeeded(key)
  emit('mark', payload)
}

// ---------- 朗读（TTS） ----------

const speakingRange = ref<{ from: number; to: number } | null>(null)
const autoRestart = ref(false)
const ttsState = computed(() => ttsPlayer.state)
const sentenceSpans = computed(() => splitSentenceSpans(fullText.value))
const ttsProgress = computed(() => {
  const i = ttsPlayer.currentIndex.value
  const n = ttsPlayer.totalSentences.value
  return n > 0 ? `${Math.max(1, i + 1)}/${n}` : ''
})
const ttsLabel = computed(() => {
  if (ttsState.value === 'playing') return t('wordTap.ttsPause')
  if (ttsState.value === 'paused') return t('wordTap.ttsResume')
  return t('wordTap.ttsPlay')
})

function isSpeaking(token: SerialToken): boolean {
  if (!speakingRange.value || token.serial === undefined) return false
  return token.serial >= speakingRange.value.from && token.serial <= speakingRange.value.to
}

function highlightSentence(index: number): void {
  const span: SentenceSpan | undefined = sentenceSpans.value[index]
  if (!span) {
    speakingRange.value = null
    return
  }
  let from = -1
  let to = -1
  for (const block of renderInfo.value.blocks) {
    for (const token of block.tokens) {
      if (token.serial === undefined) continue
      if (token.start < span.end && token.end > span.start) {
        if (from === -1) from = token.serial
        to = token.serial
      }
    }
  }
  speakingRange.value = from === -1 ? null : { from, to }
  void nextTick(() => {
    if (from === -1) return
    const first = document.querySelector(`.wt-word[data-tok="${from}"]`)
    first?.scrollIntoView({ block: 'center', behavior: 'smooth' })
  })
}

function currentTtsSettings() {
  return {
    provider: settingsStore.ttsProvider,
    voice: settingsStore.ttsVoice,
    rate: settingsStore.ttsRate,
    pitch: settingsStore.ttsPitch,
    volume: settingsStore.ttsVolume,
    apiKey:
      settingsStore.ttsProvider === 'dashscope'
        ? settingsStore.ttsDashKey
        : settingsStore.ttsMinimaxKey,
    groupId: settingsStore.ttsMinimaxGroupId,
    sentencePauseMs: settingsStore.ttsPauseSentence,
  }
}

async function toggleTts(): Promise<void> {
  if (ttsPlayer.state === 'playing') {
    ttsPlayer.pause()
    return
  }
  if (ttsPlayer.state === 'paused') {
    ttsPlayer.resume()
    return
  }
  await startTts()
}

async function startTts(): Promise<void> {
  const spans = sentenceSpans.value
  const sentences = spans.map((s) => s.text)
  if (sentences.length === 0) return
  // 对白分音色：角色面板开启时，按说话人覆盖每句音色
  //（显式指派音色 > 性别默认音色 > 主音色）
  const overrides = dialogueVoiceEnabled.value
    ? buildVoiceOverrides(
        spans,
        fullText.value,
        charVoices.value,
        charGenders.value,
        { male: settingsStore.ttsMaleVoice, female: settingsStore.ttsFemaleVoice },
        settingsStore.ttsQuoteStyles,
      )
    : undefined
  await ttsPlayer.start(
    sentences,
    currentTtsSettings(),
    {
      onSentenceStart: (i) => highlightSentence(i),
      onFinish: (completed) => {
        speakingRange.value = null
        if (completed && settingsStore.ttsAutoNext) {
          autoRestart.value = true
          emit('ttsNext')
        }
      },
    },
    overrides,
  )
}

// ---------- 角色分音色（自加载 + CharacterVoicePanel 回传） ----------
const charPanel = ref<InstanceType<typeof CharacterVoicePanel> | null>(null)
const charVoices = ref<Record<string, string>>({})
const charGenders = ref<Record<string, 'male' | 'female' | 'unknown'>>({})
const dialogueVoiceEnabled = ref(false)

/** 面板未打开也要生效：挂载/换书时自行加载角色信息 */
async function loadCharacters(): Promise<void> {
  if (props.novelId == null) return
  try {
    const saved = await invoke<
      Array<{ name: string; gender: string; voice: string }>
    >('list_novel_characters', { novelId: props.novelId })
    applyCharacters(saved)
  } catch {
    /* 静默：朗读走主音色 */
  }
}

function applyCharacters(
  saved: Array<{ name: string; gender: string; voice: string }>,
): void {
  const voices: Record<string, string> = {}
  const genders: Record<string, 'male' | 'female' | 'unknown'> = {}
  for (const c of saved) {
    if (c.voice) voices[c.name] = c.voice
    genders[c.name] = (c.gender as 'male' | 'female' | 'unknown') ?? 'unknown'
  }
  charVoices.value = voices
  charGenders.value = genders
}

function onCharVoicesUpdated(payload: {
  charVoices: Record<string, string>
  charGenders: Record<string, 'male' | 'female' | 'unknown'>
  dialogueEnabled: boolean
}): void {
  charVoices.value = payload.charVoices
  charGenders.value = payload.charGenders
  dialogueVoiceEnabled.value = payload.dialogueEnabled
}

onMounted(() => {
  if (props.novelId != null) {
    dialogueVoiceEnabled.value =
      localStorage.getItem(`dialogue-voice-enabled-${props.novelId}`) === '1'
  }
  void loadCharacters()
})
watch(
  () => props.novelId,
  () => void loadCharacters(),
)

function stopTts(): void {
  autoRestart.value = false
  ttsPlayer.stop()
  speakingRange.value = null
}

// 章节内容变化（连播切章）→ 因连播触发时自动重新开始朗读
watch(
  () => props.content,
  () => {
    speakingRange.value = null
    if (autoRestart.value) {
      autoRestart.value = false
      void nextTick(() => void startTts())
    }
  },
)

// ---------- 保存短语 ----------

async function savePhrase() {
  const draft = phraseDraft.value
  if (!draft || savingPhrase.value) return
  savingPhrase.value = true
  try {
    await invoke('mark_word_tap_proficiency', { words: [draft.text], proficiency: 'unknown' })
    phrases.value = new Set(phrases.value).add(wordKey(draft.text))
    states.value = { ...states.value, [wordKey(draft.text)]: 'unknown' }
    noteNewWordIfNeeded(wordKey(draft.text))
    clearSelection()
    lastWordEl = null
    ElMessage.success(t('wordTap.phraseSaved'))
  } catch (e: any) {
    ElMessage.error(String(e?.message || e))
  } finally {
    savingPhrase.value = false
  }
}

// ---------- 计数 ----------

const counts = computed(() => {
  const result = { new: 0, unknown: 0, familiar: 0, mastered: 0, ignore: 0 }
  for (const block of displayedBlocks.value) {
    for (const token of block.tokens) {
      if (token.type !== 'word' && token.type !== 'phrase') continue
      const st = states.value[token.key]
      if (!st) result.new += 1
      else if (st in result) result[st as keyof typeof result] += 1
    }
  }
  return result
})

// ---------- 读完本章 ----------

async function finishChapter() {
  const remaining: string[] = []
  for (const key of collectWordKeys(displayedBlocks.value)) {
    const st = states.value[key]
    if (!st || st === 'unknown') remaining.push(sampleWords.value[key] ?? key)
  }
  if (remaining.length === 0) {
    ElMessage.info(t('wordTap.nothingToIgnore'))
    return
  }
  try {
    await ElMessageBox.confirm(
      t('wordTap.finishConfirm', { n: remaining.length }),
      t('wordTap.finish'),
      { type: 'warning', confirmButtonText: t('wordTap.finishYes'), cancelButtonText: t('ui.cancel') },
    )
  } catch {
    return
  }
  finishing.value = true
  try {
    await invoke('mark_word_tap_proficiency', { words: remaining, proficiency: 'ignore' })
    const next = { ...states.value }
    for (const key of collectWordKeys(displayedBlocks.value)) {
      if (!next[key] || next[key] === 'unknown') next[key] = 'ignore'
    }
    states.value = next
    ElMessage.success(t('wordTap.finishDone'))
  } catch (e: any) {
    ElMessage.error(String(e?.message || e))
  } finally {
    finishing.value = false
  }
}

// ---------- 朗读音频 ----------

const audioPath = ref('')

/** 自定义协议 URL：Windows/Linux 为 http://<scheme>.localhost，macOS/Android 为 <scheme>://localhost */
const audioSrc = computed(() => {
  if (!props.novelId || !audioPath.value) return ''
  const macLike = /Macintosh|Android|iPhone|iPad/.test(navigator.userAgent)
  const base = macLike ? 'novelaudio://localhost' : 'http://novelaudio.localhost'
  return `${base}/stream/${props.novelId}`
})

async function loadAudioSetting() {
  if (!props.novelId) {
    audioPath.value = ''
    return
  }
  try {
    audioPath.value = await invoke<string>('get_setting', {
      key: `novel_audio_${props.novelId}`,
    })
  } catch {
    audioPath.value = ''
  }
}

async function pickAudio() {
  if (!props.novelId) return
  const file = await openFileDialog({
    multiple: false,
    filters: [{ name: 'Audio', extensions: ['mp3', 'm4a', 'aac', 'wav', 'ogg', 'flac'] }],
  })
  if (typeof file !== 'string') return
  await invoke('set_setting', { key: `novel_audio_${props.novelId}`, value: file })
  audioPath.value = file
  ElMessage.success(t('wordTap.audioSet'))
}

async function clearAudio() {
  if (!props.novelId) return
  await invoke('set_setting', { key: `novel_audio_${props.novelId}`, value: '' })
  audioPath.value = ''
}

watch(() => props.novelId, loadAudioSetting, { immediate: true })

// ---------- 全局按键（Esc 取消短语选择） ----------

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && selection.value) clearSelection()
}

/** 点击空白处（弹窗与单词之外）关闭查词弹窗，不耽误阅读 */
function onDocMouseDown(e: MouseEvent) {
  if (!popover.value.visible) return
  const target = e.target as HTMLElement | null
  if (!target) return
  if (target.closest('.dict-lookup-popover') || target.closest('.wt-word')) return
  popover.value = { ...popover.value, visible: false }
}

onMounted(() => {
  void loadStates()
  void loadAudioSetting()
  document.addEventListener('keydown', onKeydown)
  document.addEventListener('mousedown', onDocMouseDown)
})
onBeforeUnmount(() => {
  disposed.value = true
  document.removeEventListener('keydown', onKeydown)
  document.removeEventListener('mousedown', onDocMouseDown)
})
</script>

<style scoped>
.word-tap-reader {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.wt-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
  flex-wrap: wrap;
}
.wt-counts {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  font-size: 12px;
}
.wt-c::before {
  content: '';
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-right: 4px;
  vertical-align: middle;
}
.wt-c-new::before { background: var(--wt-new, #409eff); }
.wt-c-unknown::before { background: var(--wt-unknown, #e05252); }
.wt-c-familiar::before { background: var(--wt-familiar, #e6a23c); }
.wt-c-mastered::before { background: var(--wt-mastered, #67a35f); }
.wt-c-ignore::before { background: var(--wt-ignore, #b8bfc9); }

.wt-phrase-bar {
  display: flex;
  align-items: center;
  gap: 8px;
}
.wt-phrase-text {
  font-size: 13px;
  font-weight: 600;
  color: var(--accent-color, #409eff);
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.wt-audio {
  display: flex;
  align-items: center;
  gap: 4px;
}
.wt-audio-player {
  height: 32px;
  max-width: min(360px, 40vw);
}

.wt-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 16px clamp(16px, 6vw, 64px);
  user-select: none;
}
.wt-block {
  line-height: 2;
  margin: 0 0 12px;
  color: var(--text-primary, #303133);
  font-size: var(--reading-font-size, 17px);
}

.wt-word {
  cursor: pointer;
  border-radius: 3px;
  padding: 0 1px;
  transition: background-color 0.15s;
}
.wt-word:hover {
  background: var(--wt-hover, rgba(64, 158, 255, 0.18));
}

/* 短语选择中的高亮 */
.wt-word.wt-selecting {
  background: var(--wt-selecting, rgba(103, 163, 95, 0.35));
  outline: 1px solid var(--wt-mastered, #67a35f);
}

/* 状态着色（背景微高亮 + 文本色） */
.wt-st-new {
  background: color-mix(in srgb, var(--wt-new, #409eff) 22%, transparent);
}
.wt-st-unknown {
  background: color-mix(in srgb, var(--wt-unknown, #e05252) 22%, transparent);
  color: var(--wt-unknown, #c45656);
  font-weight: 600;
}
.wt-st-familiar {
  background: color-mix(in srgb, var(--wt-familiar, #e6a23c) 20%, transparent);
  color: var(--wt-familiar, #b88230);
}
.wt-st-mastered {
  color: var(--wt-mastered-text, #6b7280);
}
.wt-st-ignore {
  color: var(--wt-ignore, #b8bfc9);
}

.wt-loading-more {
  text-align: center;
  color: var(--text-secondary, #909399);
  font-size: 12px;
}
</style>
