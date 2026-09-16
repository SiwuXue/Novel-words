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

    <!-- Error -->
    <div v-else-if="store.lookupError" class="dict-state dict-error">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ store.lookupError }}</span>
    </div>

    <!-- 英→中：单条结果 -->
    <template v-else-if="store.direction === 'english'">
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
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { Loading, WarningFilled, Search, Microphone } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { useDictionaryStore, type DictWord } from '@/stores/dictionaryStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { speakWord } from '@/utils/speech'
import type { UserVocabEntry, VocabWord } from '@/types/vocabWord'
import { t } from '@/i18n'

const props = defineProps<{
  text: string
  position: { x: number; y: number }
  novelId: number | null
  chapterId: number | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
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

function collectionKey(bookId: number | null, word: string): string {
  return `${bookId}:${word.replace(/[‘’]/g, "'").trim().replace(/\s+/g, ' ').toLowerCase()}`
}
function hasCollected(word: string): boolean {
  return selectedBookId.value !== null && addedWords.value.has(collectionKey(selectedBookId.value, word))
}

watch(() => store.direction === 'english' && !store.looking ? store.currentWord?.word : null, async word => {
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

/** Manual speak button handler */
function speak(word: string) {
  speakWord(word, settingsStore.speechAccent)
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
</style>
