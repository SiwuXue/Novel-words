<template>
  <div class="review-page">
    <div class="review-header">
      <el-button link @click="goBack">
        <el-icon><ArrowLeft /></el-icon> {{ t('review.back') }}
      </el-button>
      <h2>{{ book?.name || t('review.allBooks') }}</h2>
      <el-select v-if="queue.length" v-model="reviewMode" class="review-mode-select" size="small">
        <el-option
          v-for="option in reviewModeOptions"
          :key="option.value"
          :value="option.value"
          :label="option.label"
        />
      </el-select>
      <span class="review-progress" v-if="progress">
        {{ t('review.today') }} <strong>{{ progress.reviewed_today }}</strong> / {{ progress.goal }} ·
        {{ t('review.due') }} <strong>{{ progress.due_total }}</strong>
      </span>
      <el-progress
        v-if="progress"
        class="review-progress-bar"
        :percentage="goalPercent"
        :stroke-width="6"
        :show-text="false"
      />
    </div>

    <div v-if="loading" class="review-state">
      <el-icon class="is-loading" :size="32"><Loading /></el-icon>
      <span>…</span>
    </div>

    <!-- Empty state -->
    <div v-else-if="!queue.length && !reviewed" class="review-state">
      <div class="review-done-icon">🎉</div>
      <h3>{{ t('review.noDueTitle') }}</h3>
      <p>{{ t('review.noDueDesc') }}</p>
    </div>

    <!-- Finished state -->
    <div v-else-if="!queue.length" class="review-state">
      <div class="review-done-icon">✅</div>
      <h3>{{ t('review.doneTitle') }}</h3>
      <div class="review-summary">
        <div class="summary-item">
          <span class="summary-num">{{ reviewed }}</span>
          <span class="summary-label">{{ t('review.reviewed') }}</span>
        </div>
        <div class="summary-item">
          <span class="summary-num mastered">{{ stats.easy }}</span>
          <span class="summary-label">{{ t('review.mastered') }}</span>
        </div>
        <div class="summary-item">
          <span class="summary-num familiar">{{ stats.good }}</span>
          <span class="summary-label">{{ t('review.familiar') }}</span>
        </div>
        <div class="summary-item">
          <span class="summary-num unknown">{{ stats.again }}</span>
          <span class="summary-label">{{ t('review.unknown') }}</span>
        </div>
      </div>
      <el-button type="primary" @click="goBack">{{ t('review.backToBook') }}</el-button>
    </div>

    <!-- Card -->
    <div v-else class="review-card-wrap">
      <div class="review-card">
        <div class="mode-kicker">{{ modeLabel }}</div>

        <template v-if="reviewMode === 'translation'">
          <div class="prompt-label">{{ translationDirection === 'word-to-definition' ? t('review.translateWord') : t('review.translateDefinition') }}</div>
          <div class="card-word">{{ translationPrompt }}</div>
          <div v-if="translationDirection === 'word-to-definition'" class="card-phonetic">{{ current?.phonetic || '' }}</div>
        </template>

        <template v-else-if="reviewMode === 'spelling'">
          <div class="prompt-label">{{ t('review.spellPrompt') }}</div>
          <div class="card-definition">{{ current?.definition || '—' }}</div>
          <div v-if="current?.exampleSentence" class="card-example">{{ current.exampleSentence }}</div>
        </template>

        <template v-else-if="reviewMode === 'listening'">
          <div class="prompt-label">{{ t('review.listenPrompt') }}</div>
          <el-button class="listen-button" type="primary" plain @click="playCurrentAudio">
            {{ t('review.playAudio') }}
          </el-button>
        </template>

        <template v-else-if="reviewMode === 'cloze'">
          <div class="prompt-label">{{ t('review.clozePrompt') }}</div>
          <div class="context-text">{{ clozeSentence }}</div>
        </template>

        <template v-else-if="reviewMode === 'context'">
          <div class="prompt-label">{{ t('review.contextPrompt') }}</div>
          <el-icon v-if="contextLoading" class="is-loading"><Loading /></el-icon>
          <div v-else class="context-text">{{ contextSnippet || t('review.contextUnavailable') }}</div>
        </template>

        <template v-else>
          <div class="card-word">{{ current?.word }}</div>
          <div class="card-phonetic">{{ current?.phonetic || '' }}</div>
        </template>

        <template v-if="revealed">
          <el-divider />
          <div class="answer-label">{{ t('review.answer') }}</div>
          <div class="card-definition">{{ answerText }}</div>
          <div v-if="answerChecked && (isInputMode || selectedOption)" class="choice-result" :class="{ correct: inputCorrect, incorrect: !inputCorrect }">
            {{ inputCorrect ? t('review.correct') : t('review.incorrect') }}
          </div>
          <div v-if="current?.exampleSentence" class="card-example">
            {{ current.exampleSentence }}
          </div>
        </template>
        <template v-else>
          <div v-if="reviewMode === 'listening'" class="choice-list">
            <el-button
              v-for="option in listeningOptions"
              :key="option"
              class="choice-button"
              @click="chooseOption(option)"
            >
              {{ option }}
            </el-button>
          </div>
          <div v-else-if="reviewMode === 'context'" class="choice-list">
            <el-button
              v-for="option in contextOptions"
              :key="option"
              class="choice-button"
              @click="chooseOption(option)"
            >
              {{ option }}
            </el-button>
          </div>
          <div v-else-if="isInputMode" class="answer-input-wrap">
            <el-input
              v-model="answerInput"
              :placeholder="t('review.inputPlaceholder')"
              size="large"
              clearable
              autofocus
              @keyup.enter="checkInput"
            />
            <div v-if="answerChecked" class="choice-result" :class="{ correct: inputCorrect, incorrect: !inputCorrect }">
              {{ inputCorrect ? t('review.correct') : t('review.incorrect') }}
            </div>
          </div>
          <div v-else class="card-hint">{{ t('review.hint') }}</div>
        </template>

        <div class="next-review-preview">
          <span class="next-review-title">{{ t('review.nextReview') }}</span>
          <span v-for="item in nextReviewOptions" :key="item.rating" class="next-review-item">
            <strong>{{ ratingLabel(item.rating) }}</strong> {{ item.label }}
          </span>
        </div>
      </div>

      <div class="review-actions">
        <template v-if="!revealed && isInputMode">
          <el-button type="primary" size="large" :disabled="!answerInput.trim()" @click="checkInput">
            {{ t('review.checkAnswer') }}
          </el-button>
          <el-button size="large" @click="revealAnswer">
            {{ t('review.showAnswer') }}
          </el-button>
        </template>
        <template v-else-if="!revealed && reviewMode !== 'listening' && reviewMode !== 'context'">
          <el-button type="primary" size="large" @click="revealed = true">
            {{ t('review.showAnswer') }}
          </el-button>
        </template>
        <template v-else>
          <el-button type="danger" size="large" @click="answer('again')">
            {{ t('review.again') }}<br /><span class="btn-sub">{{ t('review.againSub') }}</span>
          </el-button>
          <el-button type="warning" size="large" @click="answer('good')">
            {{ t('review.good') }}<br /><span class="btn-sub">{{ t('review.goodSub') }}</span>
          </el-button>
          <el-button type="success" size="large" @click="answer('easy')">
            {{ t('review.easy') }}<br /><span class="btn-sub">{{ t('review.easySub') }}</span>
          </el-button>
        </template>
      </div>
      <div class="keyboard-hint">{{ t('review.keyboardHint') }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, Loading } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import type { VocabWord } from '@/types/vocabWord'
import type { Chapter } from '@/types/novel'
import { speakWord } from '@/utils/speech'
import { useSettingsStore } from '@/stores/settingsStore'
import { t } from '@/i18n'

interface ReviewProgress {
  vocab_book_id: number | null
  due_total: number
  reviewed_today: number
  goal: number
}

type ReviewMode = 'recall' | 'translation' | 'spelling' | 'listening' | 'cloze' | 'context'
type TranslationDirection = 'word-to-definition' | 'definition-to-word'

interface ReviewOptionPreview {
  rating: 'again' | 'good' | 'easy'
  label: string
}

const route = useRoute()
const router = useRouter()
const bookStore = useVocabBookStore()
const settingsStore = useSettingsStore()

const bookId = computed(() => Number(route.params.id))
const book = computed(() => bookStore.books.find((b) => b.id === bookId.value) || null)

const loading = ref(false)
const queue = ref<VocabWord[]>([])
const current = computed(() => queue.value[0] ?? null)
const revealed = ref(false)
const reviewed = ref(0)
const stats = ref({ again: 0, good: 0, easy: 0 })
const progress = ref<ReviewProgress | null>(null)
const answering = ref(false)
const reviewMode = ref<ReviewMode>('recall')
const answerInput = ref('')
const answerChecked = ref(false)
const inputCorrect = ref(false)
const selectedOption = ref('')
const translationDirection = ref<TranslationDirection>('word-to-definition')
const contextSnippet = ref('')
const contextLoading = ref(false)
let contextRequestId = 0
const chapterCache = new Map<number, Chapter[]>()
const distractorWords = ref<VocabWord[]>([])

const reviewModeOptions = computed(() => [
  { value: 'recall' as ReviewMode, label: t('review.modeRecall') },
  { value: 'translation' as ReviewMode, label: t('review.modeTranslation') },
  { value: 'spelling' as ReviewMode, label: t('review.modeSpelling') },
  { value: 'listening' as ReviewMode, label: t('review.modeListening') },
  { value: 'cloze' as ReviewMode, label: t('review.modeCloze') },
  { value: 'context' as ReviewMode, label: t('review.modeContext') },
])

const modeLabel = computed(
  () => reviewModeOptions.value.find((option) => option.value === reviewMode.value)?.label || '',
)
const isInputMode = computed(() =>
  reviewMode.value === 'translation' || reviewMode.value === 'spelling' || reviewMode.value === 'cloze',
)
const translationPrompt = computed(() => {
  if (!current.value) return ''
  return translationDirection.value === 'word-to-definition'
    ? current.value.word
    : current.value.definition
})
const answerText = computed(() => {
  if (!current.value) return ''
  if (
    reviewMode.value === 'recall' ||
    reviewMode.value === 'context' ||
    (reviewMode.value === 'translation' && translationDirection.value === 'word-to-definition')
  ) {
    return current.value.definition || '—'
  }
  return current.value.word || '—'
})
const clozeSentence = computed(() => {
  const sentence = current.value?.exampleSentence?.trim() || ''
  if (!sentence || !current.value) return t('review.noExample')
  const terms = [current.value.word, ...(current.value.matchTerms || '').split(/[|,;]/)]
    .map((term) => term.trim())
    .filter(Boolean)
    .sort((a, b) => b.length - a.length)
  for (const term of terms) {
    const escaped = term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    const replaced = sentence.replace(new RegExp(escaped, 'gi'), '＿＿＿＿')
    if (replaced !== sentence) return replaced
  }
  return `${sentence}（＿＿＿＿）`
})
const listeningOptions = computed(() => {
  const values = [
    current.value?.word || '',
    ...queue.value.slice(1, 6).map((word) => word.word),
    ...distractorWords.value.map((word) => word.word),
  ]
    .map((word) => word.trim())
    .filter(Boolean)
  return [...new Set(values)].slice(0, 4)
})
const contextOptions = computed(() => {
  const values = [
    current.value?.definition || '',
    ...queue.value.slice(1, 6).map((word) => word.definition),
    ...distractorWords.value.map((word) => word.definition),
  ]
    .map((definition) => definition.trim())
    .filter(Boolean)
  return [...new Set(values)].slice(0, 4)
})
const nextReviewOptions = computed<ReviewOptionPreview[]>(() => {
  if (!current.value) return []
  return (['again', 'good', 'easy'] as const).map((rating) => ({
    rating,
    label: getNextReviewLabel(current.value!.memoryTag, rating),
  }))
})

const goalPercent = computed(() => {
  if (!progress.value) return 0
  const goal = progress.value.goal || 1
  return Math.min(100, Math.round((progress.value.reviewed_today / goal) * 100))
})

function ratingLabel(rating: 'again' | 'good' | 'easy'): string {
  if (rating === 'again') return t('review.again')
  if (rating === 'good') return t('review.good')
  return t('review.easy')
}

function parseSrs(memoryTag: string): { ease: number; interval: number; reps: number } {
  try {
    const parsed = JSON.parse(memoryTag) as { srs?: { ease?: number; interval?: number; reps?: number } }
    return {
      ease: typeof parsed.srs?.ease === 'number' ? parsed.srs.ease : 2.5,
      interval: typeof parsed.srs?.interval === 'number' ? parsed.srs.interval : 0,
      reps: typeof parsed.srs?.reps === 'number' ? parsed.srs.reps : 0,
    }
  } catch {
    return { ease: 2.5, interval: 0, reps: 0 }
  }
}

function getNextReviewLabel(memoryTag: string, rating: 'again' | 'good' | 'easy'): string {
  const state = parseSrs(memoryTag)
  let days = 1
  if (rating === 'easy') {
    days = state.reps === 0
      ? 4
      : Math.max(1, Math.round(state.interval * state.ease * 1.3))
  } else if (rating === 'good') {
    days = state.reps === 0
      ? 1
      : state.reps === 1
        ? 6
        : Math.max(1, Math.round(state.interval * state.ease))
  }
  if (days === 1) return t('review.tomorrow')
  return t('review.daysLater', { n: days })
}

function normalizeAnswer(value: string): string {
  return value
    .trim()
    .toLocaleLowerCase()
    .replace(/[“”‘’'.,!?;:()[\]{}]/g, '')
    .replace(/\s+/g, ' ')
}

function expectedSpellingAnswers(): string[] {
  if (!current.value) return []
  return [current.value.word, ...(current.value.matchTerms || '').split(/[|,;]/)]
    .map((term) => normalizeAnswer(term))
    .filter(Boolean)
}

function resetCardState() {
  revealed.value = false
  answerInput.value = ''
  answerChecked.value = false
  inputCorrect.value = false
  selectedOption.value = ''
  contextSnippet.value = ''
  translationDirection.value = current.value?.id && current.value.id % 2 === 0
    ? 'definition-to-word'
    : 'word-to-definition'
}

function revealAnswer() {
  revealed.value = true
  answerChecked.value = false
}

function checkInput() {
  if (!current.value || !answerInput.value.trim()) return
  answerChecked.value = true
  if (reviewMode.value === 'translation') {
    const typed = normalizeAnswer(answerInput.value)
    if (translationDirection.value === 'definition-to-word') {
      inputCorrect.value = expectedSpellingAnswers().includes(typed)
    } else {
      const candidates = current.value.definition
        .split(/[\n；;]/)
        .map((part) => normalizeAnswer(part.replace(/^[a-z]+\.\s*/i, '')))
        .filter((part) => part.length >= 2)
      inputCorrect.value = candidates.some((part) => typed === part || part.includes(typed) || typed.includes(part))
    }
  } else {
    inputCorrect.value = expectedSpellingAnswers().includes(normalizeAnswer(answerInput.value))
  }
  revealed.value = true
}

function chooseOption(option: string) {
  selectedOption.value = option
  inputCorrect.value = reviewMode.value === 'listening'
    ? normalizeAnswer(option) === normalizeAnswer(current.value?.word || '')
    : option === (current.value?.definition || '')
  answerChecked.value = true
  revealed.value = true
}

function playCurrentAudio() {
  if (current.value?.word) speakWord(current.value.word, settingsStore.speechAccent)
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function buildContextSnippet(text: string, word: VocabWord): string {
  const terms = [word.word, ...(word.matchTerms || '').split(/[|,;]/)]
    .map((term) => term.trim())
    .filter(Boolean)
    .sort((a, b) => b.length - a.length)
  let index = -1
  let matchedTerm = ''
  for (const term of terms) {
    const found = text.toLocaleLowerCase().indexOf(term.toLocaleLowerCase())
    if (found >= 0) {
      index = found
      matchedTerm = term
      break
    }
  }
  const source = index >= 0 ? text : word.exampleSentence || ''
  if (!source) return ''
  if (index < 0) {
    const escaped = escapeRegExp(word.word)
    return source.replace(new RegExp(escaped, 'gi'), '＿＿＿＿')
  }
  const start = Math.max(0, index - 100)
  const end = Math.min(text.length, index + matchedTerm.length + 140)
  const snippet = text.slice(start, end)
  return `${start > 0 ? '…' : ''}${snippet.replace(new RegExp(escapeRegExp(matchedTerm), 'gi'), '＿＿＿＿')}${end < text.length ? '…' : ''}`
}

async function loadContext() {
  const word = current.value
  const requestId = ++contextRequestId
  if (!word) return
  contextLoading.value = true
  try {
    let chapters: Chapter[] = []
    if (word.novelId) {
      chapters = chapterCache.get(word.novelId) || []
      if (!chapters.length) {
        chapters = await invoke<Chapter[]>('get_chapters', { novelId: word.novelId })
        chapterCache.set(word.novelId, chapters)
      }
    }
    if (requestId !== contextRequestId) return
    const chapter = chapters.find((item) => item.id === word.chapterId)
    contextSnippet.value = buildContextSnippet(chapter?.content || '', word)
  } catch {
    if (requestId === contextRequestId) contextSnippet.value = word.exampleSentence || ''
  } finally {
    if (requestId === contextRequestId) contextLoading.value = false
  }
}

watch(
  () => [current.value?.id, reviewMode.value] as const,
  async () => {
    resetCardState()
    if (reviewMode.value === 'context') {
      await loadContext()
    }
    if (reviewMode.value === 'listening' && current.value) {
      await nextTick()
      playCurrentAudio()
    }
  },
)

async function refreshProgress() {
  try {
    progress.value = await invoke<ReviewProgress>('get_review_progress', {
      vocabBookId: bookId.value,
    })
  } catch {
    /* ignore */
  }
}

async function loadDistractors() {
  try {
    const ids = bookId.value
      ? [bookId.value]
      : bookStore.books.filter((book) => !book.isPreset).map((book) => book.id)
    const results = await Promise.all(
      ids.slice(0, 8).map((vocabBookId) =>
        invoke<VocabWord[]>('get_vocab_words', { vocabBookId }),
      ),
    )
    distractorWords.value = results.flat().slice(0, 200)
  } catch {
    distractorWords.value = []
  }
}

onMounted(async () => {
  window.addEventListener('keydown', onKeyDown)
  if (bookStore.books.length === 0) {
    await bookStore.fetchAll()
  }
  loading.value = true
  try {
    queue.value = bookId.value
      ? await invoke<VocabWord[]>('get_due_words', { vocabBookId: bookId.value })
      : await invoke<VocabWord[]>('get_all_due_words')
    void loadDistractors()
    await refreshProgress()
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '加载复习队列失败'))
  } finally {
    loading.value = false
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeyDown)
})

async function answer(rating: 'again' | 'good' | 'easy') {
  const card = current.value
  if (!card || answering.value) return
  answering.value = true
  try {
    await invoke('review_vocab_word', { id: card.id, rating })
    stats.value[rating] += 1
    reviewed.value += 1
    queue.value = queue.value.slice(1)
    revealed.value = false
    await refreshProgress()
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '提交失败'))
  } finally {
    answering.value = false
  }
}

function onKeyDown(e: KeyboardEvent) {
  const target = e.target as HTMLElement | null
  if (target?.closest('input, textarea, select, [contenteditable="true"]')) return
  if (e.code === 'Space') {
    e.preventDefault()
    if (!current.value) return
    if (!revealed.value) revealAnswer()
    else void answer('good')
    return
  }
  if (!revealed.value) return
  if (e.key === '1') void answer('again')
  else if (e.key === '2') void answer('good')
  else if (e.key === '3') void answer('easy')
}

function goBack() {
  router.push(bookId.value ? `/vocabulary/${bookId.value}` : '/vocabulary')
}
</script>

<style scoped>
.review-page {
  width: 100%;
  min-width: 0;
  padding: clamp(4px, 2vw, 24px);
  display: flex;
  flex-direction: column;
  min-height: 100%;
}
.review-header {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 24px;
}
.review-header h2 {
  margin: 0;
  font-size: 20px;
}
.review-mode-select {
  width: 150px;
}
.review-progress {
  margin-left: auto;
  color: var(--text-secondary, #909399);
  font-size: 14px;
}
.review-progress strong {
  color: var(--accent-color, #409eff);
  font-size: 16px;
}
.review-progress-bar {
  margin-top: 4px;
  width: 100%;
  flex-basis: 100%;
}
.review-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  text-align: center;
  color: var(--text-secondary, #909399);
}
.review-state h3 {
  margin: 0;
  font-size: 18px;
  color: var(--text-regular, #303133);
}
.review-done-icon {
  font-size: 56px;
}
.review-summary {
  display: flex;
  gap: 32px;
  margin: 8px 0 20px;
}
.summary-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}
.summary-num {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-regular, #303133);
}
.summary-num.mastered { color: var(--success-color, #67c23a); }
.summary-num.familiar { color: var(--warning-color, #e6a23c); }
.summary-num.unknown { color: var(--danger-color, #f56c6c); }
.summary-label {
  font-size: 12px;
  color: var(--text-secondary, #909399);
}
.review-card-wrap {
  flex: 1;
  display: flex;
  flex-direction: column;
}
.review-card {
  background: var(--bg-secondary, #f5f7fa);
  border-radius: 16px;
  padding: clamp(28px, 6vw, 64px) clamp(16px, 5vw, 48px);
  text-align: center;
  margin-bottom: 24px;
}
.mode-kicker {
  margin-bottom: 20px;
  color: var(--accent-color, #409eff);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.prompt-label,
.answer-label {
  margin-bottom: 10px;
  color: var(--text-secondary, #909399);
  font-size: 13px;
}
.card-word {
  font-size: 36px;
  font-weight: 700;
  color: var(--text-regular, #303133);
  word-break: break-word;
}
.card-phonetic {
  margin-top: 8px;
  color: var(--text-secondary, #909399);
  font-size: 15px;
  min-height: 20px;
}
.card-hint {
  margin-top: 40px;
  color: var(--text-placeholder, #c0c4cc);
  font-size: 14px;
}
.card-definition {
  font-size: 18px;
  color: var(--text-regular, #303133);
  white-space: pre-wrap;
  word-break: break-word;
}
.card-example {
  margin-top: 12px;
  font-size: 14px;
  color: var(--text-secondary, #909399);
}
.listen-button {
  margin: 20px 0 8px;
}
.context-text {
  max-width: 760px;
  margin: 18px auto 0;
  color: var(--text-regular, #303133);
  font-size: 17px;
  line-height: 1.8;
  white-space: pre-wrap;
  word-break: break-word;
}
.choice-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(160px, 1fr));
  gap: 10px;
  width: min(100%, 620px);
  margin: 24px auto 0;
}
.choice-button {
  height: auto;
  min-height: 46px;
  margin: 0 !important;
  white-space: normal;
}
.answer-input-wrap {
  width: min(100%, 520px);
  margin: 24px auto 0;
}
.choice-result {
  margin-top: 10px;
  font-size: 13px;
  font-weight: 600;
}
.choice-result.correct {
  color: var(--success-color, #67c23a);
}
.choice-result.incorrect {
  color: var(--danger-color, #f56c6c);
}
.next-review-preview {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: 6px 12px;
  margin-top: 28px;
  padding-top: 14px;
  border-top: 1px solid var(--border-color, #ebeef5);
  color: var(--text-secondary, #909399);
  font-size: 12px;
}
.next-review-title {
  width: 100%;
  margin-bottom: 2px;
}
.next-review-item strong {
  color: var(--text-regular, #606266);
}
.review-actions {
  display: flex;
  gap: 16px;
  justify-content: center;
}
.review-actions .el-button {
  min-width: min(120px, 100%);
  height: auto;
  padding: 12px 0;
}
.btn-sub {
  font-size: 11px;
  font-weight: normal;
  opacity: 0.85;
}
.keyboard-hint {
  margin-top: 14px;
  color: var(--text-placeholder, #c0c4cc);
  font-size: 12px;
  text-align: center;
}

@media (max-width: 640px) {
  .review-header {
    gap: 8px;
    margin-bottom: 16px;
  }
  .review-header h2 {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .review-mode-select {
    width: 100%;
  }
  .review-progress {
    width: 100%;
    margin-left: 0;
  }
  .review-summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 18px 28px;
    width: min(100%, 360px);
  }
  .review-card {
    margin-bottom: 16px;
    border-radius: 12px;
  }
  .choice-list {
    grid-template-columns: 1fr;
  }
  .card-word {
    font-size: clamp(28px, 10vw, 36px);
  }
  .review-actions {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }
  .review-actions .el-button {
    width: 100%;
    min-width: 0;
    margin-left: 0;
    white-space: normal;
  }
  .review-actions .el-button:only-child {
    grid-column: 1 / -1;
    max-width: 240px;
    justify-self: center;
  }
}

@media (max-width: 390px) {
  .review-actions {
    grid-template-columns: 1fr;
  }
}
</style>
