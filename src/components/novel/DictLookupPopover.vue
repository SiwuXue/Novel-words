<template>
  <div
    ref="popoverRef"
    class="dict-lookup-popover"
    :style="{ left: posX + 'px', top: posY + 'px' }"
    @mousedown.stop
  >
    <!-- Loading -->
    <div v-if="store.looking" class="dict-state">
      <el-icon class="is-loading"><Loading /></el-icon>
      <span>{{ t('dict.checking') }}</span>
    </div>

    <!-- Error（离线/中→英查询错误） -->
    <div v-else-if="store.lookupError && store.direction !== 'sentence'" class="dict-state dict-error">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ store.lookupError }}</span>
    </div>

    <!-- 英→中：词典源切换 + 单条结果 -->
    <template v-else-if="store.direction === 'english'">
      <div class="dict-source-tabs" role="tablist">
        <button
          v-for="s in sourceTabs"
          :key="s.key"
          type="button"
          class="dict-source-tab"
          :class="{ active: store.activeSource === s.key }"
          role="tab"
          :aria-selected="store.activeSource === s.key"
          :disabled="store.looking || store.onlineLooking"
          @click="switchSource(s.key)"
        >
          {{ s.label }}
        </button>
      </div>

      <!-- 离线词典 -->
      <template v-if="store.activeSource === 'offline'">
        <template v-if="store.currentWord">
          <div class="dict-header">
            <span class="dict-word">{{ store.currentWord.word }}</span>
            <span v-if="preferredPhonetic" class="dict-phonetic">/{{ preferredPhonetic }}/</span>
            <el-button
              link
              size="small"
              class="dict-speak-btn"
              :title="t('dict.pronounce', { word: store.currentWord.word })"
              :aria-label="t('dict.pronounce', { word: store.currentWord.word })"
              @click="speak(store.currentWord!.word)"
            >
              <el-icon><Microphone /></el-icon>
            </el-button>
          </div>
          <div class="dict-translation">{{ store.currentWord.translation || t('dict.noDefinition') }}</div>
          <p v-if="learningState" class="dict-learning-state" role="status">{{ t('wordForm.inherited', { status: t(`vocabDetail.${learningState.proficiency}`) }) }}</p>
          <p v-else-if="checkingLearningState" class="dict-learning-state" role="status">{{ t('wordForm.checking') }}</p>
          <div v-if="quickMark" class="dict-quick-mark">
            <button
              v-for="opt in quickMarkOptions"
              :key="opt.value"
              type="button"
              class="dict-quick-btn"
              :class="`dict-quick-${opt.value}`"
              :disabled="marking !== null"
              @click="quickMark(opt.value)"
            >{{ opt.label }}</button>
          </div>
          <div class="dict-footer">
            <el-select
              popper-class="dict-book-dropdown"
              v-model="selectedBookId"
              size="small"
              :placeholder="t('dict.chooseBook')"
              :aria-label="t('dict.chooseBook')"
              :disabled="!!addingWord"
              style="flex: 1; min-width: 120px"
            >
              <el-option
                v-for="b in vocabBookStore.books"
                :key="b.id"
                :label="b.name"
                :value="b.id"
              />
            </el-select>
            <el-button
              size="small"
              type="primary"
              :disabled="!selectedBookId || hasCollected(store.currentWord.word) || !!addingWord"
              :loading="addingWord === collectionKey(selectedBookId, store.currentWord.word)"
              @click="addEnglishWord(store.currentWord)"
            >
              {{ hasCollected(store.currentWord.word) ? t('dict.added') : t('dict.addToBook') }}
            </el-button>
          </div>
        </template>
        <div v-else class="dict-state dict-empty">
          <el-icon><Search /></el-icon>
          <span>{{ t('dict.notFound') }}</span>
        </div>
      </template>

      <!-- 在线词典（有道 / 剑桥） -->
      <template v-else>
        <div v-if="store.onlineLooking" class="dict-state">
          <el-icon class="is-loading"><Loading /></el-icon>
          <span>{{ t('dict.onlineLooking') }}</span>
        </div>
        <div v-else-if="store.onlineError" class="dict-state dict-error">
          <el-icon><WarningFilled /></el-icon>
          <span>{{ store.onlineError }}</span>
        </div>
        <template v-else-if="store.onlineResult">
          <div class="dict-header">
            <span class="dict-word">{{ store.onlineResult.word }}</span>
            <span v-if="onlinePhonetic" class="dict-phonetic">/{{ onlinePhonetic }}/</span>
            <el-button
              link
              size="small"
              class="dict-speak-btn"
              :title="t('dict.pronounce', { word: store.onlineResult.word })"
              :aria-label="t('dict.pronounce', { word: store.onlineResult.word })"
              @click="speakOnline"
            >
              <el-icon><Microphone /></el-icon>
            </el-button>
          </div>
          <div class="dict-translation">
            <!-- 剑桥：按词性分组展示 -->
            <template v-if="store.onlineResult.senses.length > 0">
              <div v-for="(sense, i) in store.onlineResult.senses" :key="i" class="dict-online-sense">
                <div v-if="sense.pos" class="dict-online-pos">{{ sense.pos }}</div>
                <div v-for="(def, j) in sense.defs" :key="j" class="dict-online-def">{{ def }}</div>
                <div v-if="sense.examples.length > 0" class="dict-online-examples">
                  <div v-for="(ex, k) in sense.examples" :key="k" class="dict-online-example">
                    <div class="dict-online-example-en">{{ ex.en }}</div>
                    <div v-if="ex.zh" class="dict-online-example-zh">{{ ex.zh }}</div>
                  </div>
                </div>
              </div>
            </template>
            <!-- 有道：扁平释义行 -->
            <template v-else>
              <div v-for="(line, i) in store.onlineResult.translations" :key="i" class="dict-online-def">{{ line }}</div>
              <div v-if="store.onlineResult.examples.length > 0" class="dict-online-examples">
                <div v-for="(ex, k) in store.onlineResult.examples" :key="k" class="dict-online-example">
                  <div class="dict-online-example-en">{{ ex.en }}</div>
                  <div v-if="ex.zh" class="dict-online-example-zh">{{ ex.zh }}</div>
                </div>
              </div>
            </template>
          </div>
          <a
            v-if="store.onlineResult.url"
            class="dict-online-link"
            :href="store.onlineResult.url"
            target="_blank"
            rel="noopener"
          >{{ t('dict.openInBrowser') }}</a>
          <div v-if="quickMark" class="dict-quick-mark">
            <button
              v-for="opt in quickMarkOptions"
              :key="opt.value"
              type="button"
              class="dict-quick-btn"
              :class="`dict-quick-${opt.value}`"
              :disabled="marking !== null"
              @click="quickMark(opt.value)"
            >{{ opt.label }}</button>
          </div>
          <div class="dict-footer">
            <el-select
              popper-class="dict-book-dropdown"
              v-model="selectedBookId"
              size="small"
              :placeholder="t('dict.chooseBook')"
              :aria-label="t('dict.chooseBook')"
              :disabled="!!addingWord"
              style="flex: 1; min-width: 120px"
            >
              <el-option
                v-for="b in vocabBookStore.books"
                :key="b.id"
                :label="b.name"
                :value="b.id"
              />
            </el-select>
            <el-button
              size="small"
              type="primary"
              :disabled="!selectedBookId || hasCollected(store.onlineResult.word) || !!addingWord"
              :loading="addingWord === collectionKey(selectedBookId, store.onlineResult.word)"
              @click="addEnglishWord(onlineDictWord!)"
            >
              {{ hasCollected(store.onlineResult.word) ? t('dict.added') : t('dict.addToBook') }}
            </el-button>
          </div>
        </template>
        <div v-else class="dict-state dict-empty">
          <el-icon><Search /></el-icon>
          <span>{{ t('dict.onlineNotFound') }}</span>
        </div>
      </template>
    </template>

    <!-- 中→英：列表结果 -->
    <template v-else-if="store.direction === 'chinese'">
      <div class="dict-header">
        <span class="dict-word cn">{{ store.keyword }}</span>
        <span class="dict-count" v-if="!store.looking">
          {{ t('dict.matchCount', { n: store.chineseResults.length }) }}
        </span>
      </div>
      <div v-if="store.chineseResults.length === 0" class="dict-state dict-empty">
        <el-icon><Search /></el-icon>
        <span>{{ t('dict.noEnglishMatches') }}</span>
      </div>
      <div v-else class="dict-list">
        <div
          v-for="w in store.chineseResults"
          :key="w.word"
          class="dict-list-item"
        >
          <div class="dict-list-main">
            <div class="dict-list-word">
              {{ w.word }}
              <span v-if="w.phonetic_us" class="dict-list-phonetic">/{{ w.phonetic_us }}/</span>
              <el-button link size="small" class="dict-list-speak" :title="t('dict.pronounce', { word: w.word })" :aria-label="t('dict.pronounce', { word: w.word })" @click="speak(w.word)"><el-icon><Microphone /></el-icon></el-button>
            </div>
            <div class="dict-list-translation">{{ w.translation }}</div>
          </div>
          <el-button
            size="small"
            link
            type="primary"
            :disabled="!selectedBookId || hasCollected(w.word) || !!addingWord"
            :loading="addingWord === collectionKey(selectedBookId, w.word)"
            @click="addEnglishWord(w)"
          >
            {{ hasCollected(w.word) ? t('dict.added') : t('dict.add') }}
          </el-button>
        </div>
      </div>
      <div class="dict-footer" v-if="store.chineseResults.length > 0">
        <el-select
          popper-class="dict-book-dropdown"
          v-model="selectedBookId"
          size="small"
          :placeholder="t('dict.chooseBook')"
          :aria-label="t('dict.chooseBook')"
          :disabled="!!addingWord"
          style="flex: 1; min-width: 120px"
        >
          <el-option
            v-for="b in vocabBookStore.books"
            :key="b.id"
            :label="b.name"
            :value="b.id"
          />
        </el-select>
      </div>
    </template>

    <!-- 整句翻译（DeepLX） -->
    <template v-else-if="store.direction === 'sentence'">
      <div class="dict-header">
        <span class="dict-word cn">{{ t('dict.sentenceTitle') }}</span>
        <span class="dict-count">{{ t('dict.sentenceViaDeepL') }}</span>
      </div>
      <div v-if="store.sentenceLooking" class="dict-state">
        <el-icon class="is-loading"><Loading /></el-icon>
        <span>{{ t('dict.sentenceLooking') }}</span>
      </div>
      <div v-else-if="store.sentenceError" class="dict-state dict-error">
        <el-icon><WarningFilled /></el-icon>
        <span>{{ store.sentenceError }}</span>
        <el-button link size="small" type="primary" @click="store.lookupSentence(store.keyword)">
          {{ t('dict.retry') }}
        </el-button>
      </div>
      <template v-else>
        <div class="dict-sentence-original" :title="t('dict.sentenceOriginal')">{{ store.keyword }}</div>
        <div class="dict-translation">{{ store.sentenceTranslation || t('dict.noDefinition') }}</div>
      </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { Loading, WarningFilled, Search, Microphone } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { useDictionaryStore, type DictWord, type DictSource } from '@/stores/dictionaryStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { speakWord } from '@/utils/speech'
import type { UserVocabEntry, VocabWord, Proficiency } from '@/types/vocabWord'
import { t } from '@/i18n'

const props = defineProps<{
  text: string
  position: { x: number; y: number }
  novelId: number | null
  chapterId: number | null
  /** 逐词阅读模式：显示快捷标记按钮 */
  quickMark?: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'marked', payload: { word: string; proficiency: Proficiency }): void
}>()

const store = useDictionaryStore()
const vocabBookStore = useVocabBookStore()
const settingsStore = useSettingsStore()

const popoverRef = ref<HTMLElement | null>(null)
const selectedBookId = ref<number | null>(null)
const addingWord = ref<string | null>(null)
const addedWords = ref<Set<string>>(new Set())
const learningState = ref<UserVocabEntry | null>(null)
const checkingLearningState = ref(false)
let learningRequest = 0
let disposed = false
const posX = ref(0)
const posY = ref(0)

/** 词典源标签（英→中方向可切换） */
const sourceTabs: Array<{ key: DictSource; label: string }> = [
  { key: 'offline', label: t('dict.sourceOffline') },
  { key: 'youdao', label: t('dict.sourceYoudao') },
  { key: 'cambridge', label: t('dict.sourceCambridge') },
]

function collectionKey(bookId: number | null, word: string): string {
  return `${bookId}:${word.replace(/[‘’]/g, "'").trim().replace(/\s+/g, ' ').toLowerCase()}`
}
function hasCollected(word: string): boolean {
  return selectedBookId.value !== null && addedWords.value.has(collectionKey(selectedBookId.value, word))
}

/** 切换词典源：离线重新查库，在线走网页解析 */
function switchSource(source: DictSource) {
  if (store.activeSource === source) return
  if (source === 'offline') {
    void store.lookupEnglish(store.keyword)
  } else {
    void store.lookupOnline(store.keyword, source)
  }
}

watch(() => store.direction === 'english' && store.activeSource === 'offline' && !store.looking ? store.currentWord?.word : null, async word => {
  const request = ++learningRequest
  learningState.value = null
  checkingLearningState.value = false
  if (!word?.trim()) return
  checkingLearningState.value = true
  try {
    const state = await invoke<UserVocabEntry | null>('lookup_user_vocab', { word: word.trim() })
    if (!disposed && request === learningRequest) learningState.value = state
  } catch {
    // Collection still resolves shared state atomically in the backend.
  } finally { if (!disposed && request === learningRequest) checkingLearningState.value = false }
}, { immediate: true })

watch(() => vocabBookStore.books.map(book => book.id), ids => {
  if (selectedBookId.value && ids.includes(selectedBookId.value)) return
  const preferred = settingsStore.defaultVocabBookId
  selectedBookId.value = preferred && ids.includes(preferred) ? preferred : ids[0] ?? null
})

const preferredPhonetic = computed(() => {
  if (!store.currentWord) return ''
  return settingsStore.speechAccent === 'uk'
    ? store.currentWord.phonetic_uk
    : store.currentWord.phonetic_us
})

/** 在线结果按口音取音标 */
const onlinePhonetic = computed(() => {
  if (!store.onlineResult) return ''
  return settingsStore.speechAccent === 'uk'
    ? store.onlineResult.phonetic_uk
    : store.onlineResult.phonetic_us
})

/** 在线结果转 DictWord 以复用"加入词汇本" */
const onlineDictWord = computed<DictWord | null>(() => {
  const r = store.onlineResult
  if (!r) return null
  return {
    word: r.word,
    phonetic_uk: r.phonetic_uk,
    phonetic_us: r.phonetic_us,
    translation: r.translations.join('\n'),
    frequency: 0,
    difficulty: 0,
  }
})

/** 在线发音：优先用词典站真人音频，失败回落有道 TTS */
function speakOnline() {
  const r = store.onlineResult
  if (!r) return
  const url = settingsStore.speechAccent === 'uk' ? r.audio_uk : r.audio_us
  if (url) {
    // 有道/剑桥的 mp3 直链：直接用 Audio 播放
    const audio = new Audio(url)
    void audio.play().catch(() => speakWord(r.word, settingsStore.speechAccent))
  } else {
    speakWord(r.word, settingsStore.speechAccent)
  }
}

/** Manual speak button handler */
function speak(word: string) {
  speakWord(word, settingsStore.speechAccent)
}

/** 逐词阅读快捷标记：写入个人总词汇库并通知父级刷新颜色 */
const marking = ref<string | null>(null)
const quickMarkOptions: Array<{ value: Proficiency; label: string }> = [
  { value: 'unknown', label: t('wordTap.unknown') },
  { value: 'familiar', label: t('wordTap.familiar') },
  { value: 'mastered', label: t('wordTap.mastered') },
  { value: 'ignore', label: t('wordTap.ignore') },
]

async function quickMark(proficiency: Proficiency) {
  const word = store.currentWord?.word ?? store.onlineResult?.word
  if (!word || marking.value) return
  marking.value = proficiency
  try {
    await invoke('mark_word_tap_proficiency', { words: [word], proficiency })
    emit('marked', { word, proficiency })
    ElMessage.success(t('wordTap.marked', { status: t(`wordTap.${proficiency}`) }))
  } catch (e: any) {
    ElMessage.error(String(e?.message || e))
  } finally {
    marking.value = null
  }
}

/** Adjust position to keep popover inside viewport */
function adjustPosition() {
  if (!popoverRef.value) return
  const el = popoverRef.value
  const rect = el.getBoundingClientRect()
  let x = props.position.x
  let y = props.position.y
  // Right overflow
  if (x + rect.width > window.innerWidth - 8) {
    x = window.innerWidth - rect.width - 8
  }
  // Bottom overflow
  if (y + rect.height > window.innerHeight - 8) {
    y = props.position.y - rect.height - 8
  }
  if (x < 8) x = 8
  if (y < 8) y = 8
  posX.value = x
  posY.value = y
}

async function addEnglishWord(w: DictWord) {
  const bookId = selectedBookId.value
  if (!bookId || !vocabBookStore.books.some(book => book.id === bookId)) {
    ElMessage.warning(t('dict.chooseBookFirst'))
    return
  }
  const key = collectionKey(bookId, w.word)
  if (addedWords.value.has(key) || addingWord.value) return
  addingWord.value = key
  try {
    await invoke<VocabWord>('create_vocab_word', {
      vocabBookId: bookId,
      word: w.word.trim(),
      definition: w.translation,
      phonetic: w.phonetic_us || w.phonetic_uk,
      exampleSentence: '',
      novelId: props.novelId,
      chapterId: props.chapterId,
      proficiency: 'unknown',
      memoryTag: '',
    })
    if (disposed) return
    addedWords.value.add(key)
    ElMessage.success(t('dict.collected', { word: w.word }))
  } catch (e: any) {
    const msg = String(e?.message || e)
    if (disposed) return
    if (msg.includes('已存在')) {
      addedWords.value.add(key)
      ElMessage.info(t('dict.alreadyInBook', { word: w.word }))
    } else {
      ElMessage.error(msg)
    }
  } finally {
    addingWord.value = null
  }
}

/** Close on Escape key */
function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    emit('close')
  }
}

onMounted(async () => {
  await vocabBookStore.fetchAll()
  if (disposed) return
  // Preserve a selection made from cached books while the refresh was pending.
  if (!selectedBookId.value || !vocabBookStore.books.some(book => book.id === selectedBookId.value)) {
    if (settingsStore.defaultVocabBookId && vocabBookStore.books.some(book => book.id === settingsStore.defaultVocabBookId)) {
      selectedBookId.value = settingsStore.defaultVocabBookId
    } else if (vocabBookStore.books.length > 0) {
      selectedBookId.value = vocabBookStore.books[0].id
    }
  }
  document.addEventListener('keydown', onKeydown)
  // Wait for DOM to render then adjust
  await nextTick()
  if (!disposed) adjustPosition()
})

onBeforeUnmount(() => {
  disposed = true
  ++learningRequest
  document.removeEventListener('keydown', onKeydown)
})
</script>

<style scoped>
.dict-lookup-popover {
  position: fixed;
  z-index: 3000;
  width: min(360px, calc(100vw - 16px));
  min-width: 0;
  max-width: calc(100vw - 16px);
  max-height: 60vh;
  background: var(--bg-primary, #fff);
  border: 1px solid var(--border-color, #dcdfe6);
  border-radius: 8px;
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.15);
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  font-size: 13px;
}

.dict-state {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 4px;
  color: var(--text-secondary, #909399);
}
.dict-state.dict-error { color: var(--danger-color, #f56c6c); }
.dict-state.dict-empty { color: var(--text-placeholder, #c0c4cc); }

.dict-source-tabs {
  display: flex;
  gap: 4px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
  padding-bottom: 6px;
}
.dict-source-tab {
  border: none;
  background: transparent;
  color: var(--text-secondary, #909399);
  font-size: 12px;
  padding: 3px 10px;
  border-radius: 4px;
  cursor: pointer;
}
.dict-source-tab:hover { color: var(--text-primary, #303133); }
.dict-source-tab.active {
  color: var(--accent-color, #409eff);
  background: var(--bg-secondary, #ecf5ff);
  font-weight: 600;
}
.dict-source-tab:disabled { cursor: default; opacity: 0.6; }

.dict-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
  flex-wrap: wrap;
}
.dict-word {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary, #303133);
}
.dict-word.cn { font-size: 15px; }
.dict-phonetic {
  color: var(--text-secondary, #909399);
  font-size: 12px;
}
.dict-speak-btn {
  margin-left: auto;
  color: var(--accent-color, #409eff);
  padding: 2px;
}
.dict-list-speak {
  cursor: pointer;
  color: var(--accent-color, #409eff);
  margin-left: 4px;
  font-size: 13px;
}
.dict-list-speak:hover { opacity: 0.7; }
.dict-count {
  color: var(--text-secondary, #909399);
  font-size: 12px;
  margin-left: auto;
}

.dict-translation {
  color: var(--text-regular, #606266);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 200px;
  overflow-y: auto;
  padding: 4px 0;
}
.dict-learning-state { color: var(--text-secondary); font-size: 12px; line-height: 1.6; margin: 0; }

.dict-online-sense { margin-bottom: 6px; }
.dict-online-pos {
  font-weight: 600;
  color: var(--accent-color, #409eff);
  font-size: 12px;
  margin: 2px 0;
}
.dict-online-def { margin: 2px 0; }
.dict-online-examples {
  margin-top: 4px;
  padding-left: 10px;
  border-left: 2px solid var(--border-color, #ebeef5);
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.dict-online-example-en { color: var(--text-regular, #606266); font-style: italic; }
.dict-online-example-zh { color: var(--text-secondary, #909399); font-size: 12px; }
.dict-online-link {
  color: var(--accent-color, #409eff);
  font-size: 12px;
  text-decoration: none;
  align-self: flex-start;
}
.dict-online-link:hover { text-decoration: underline; }

.dict-sentence-original {
  color: var(--text-secondary, #909399);
  font-size: 12px;
  line-height: 1.5;
  padding: 6px 8px;
  background: var(--bg-secondary, #fafafa);
  border-radius: 4px;
  max-height: 90px;
  overflow-y: auto;
  word-break: break-word;
}

.dict-list {
  max-height: 280px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.dict-list-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 4px;
  background: var(--bg-secondary, #fafafa);
}
.dict-list-main { flex: 1; min-width: 0; }
.dict-list-word {
  font-weight: 600;
  color: var(--text-primary, #303133);
}
.dict-list-phonetic {
  color: var(--text-secondary, #909399);
  font-size: 11px;
  font-weight: normal;
}
.dict-list-translation {
  color: var(--text-regular, #606266);
  font-size: 12px;
  margin-top: 2px;
  word-break: break-word;
}

.dict-footer {
  display: flex;
  gap: 8px;
  align-items: center;
  padding-top: 4px;
  border-top: 1px solid var(--border-color, #ebeef5);
}

/* 逐词阅读快捷标记 */
.dict-quick-mark {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.dict-quick-btn {
  border: 1px solid var(--border-color, #dcdfe6);
  background: var(--bg-secondary, #fafafa);
  color: var(--text-regular, #606266);
  font-size: 12px;
  padding: 3px 10px;
  border-radius: 4px;
  cursor: pointer;
}
.dict-quick-btn:hover { border-color: currentColor; }
.dict-quick-btn:disabled { opacity: 0.5; cursor: default; }
.dict-quick-unknown { color: var(--wt-unknown, #c45656); }
.dict-quick-familiar { color: var(--wt-familiar, #b88230); }
.dict-quick-mastered { color: var(--wt-mastered, #5a8a53); }
.dict-quick-ignore { color: var(--wt-ignore, #8b929c); }
</style>
