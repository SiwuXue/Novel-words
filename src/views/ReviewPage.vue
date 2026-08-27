<template>
  <div class="review-page">
    <div class="review-header">
      <el-button link @click="goBack">
        <el-icon><ArrowLeft /></el-icon> {{ t('review.back') }}
      </el-button>
      <h2 v-if="book">{{ book.name }}</h2>
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
        <div class="card-word">{{ current?.word }}</div>
        <div class="card-phonetic">{{ current?.phonetic || '' }}</div>

        <template v-if="revealed">
          <el-divider />
          <div class="card-definition">{{ current?.definition || '—' }}</div>
          <div v-if="current?.exampleSentence" class="card-example">
            {{ current.exampleSentence }}
          </div>
        </template>
        <template v-else>
          <div class="card-hint">{{ t('review.hint') }}</div>
        </template>
      </div>

      <div class="review-actions">
        <template v-if="!revealed">
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
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, Loading } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import type { VocabWord } from '@/types/vocabWord'
import { t } from '@/i18n'

interface ReviewProgress {
  vocab_book_id: number | null
  due_total: number
  reviewed_today: number
  goal: number
}

const route = useRoute()
const router = useRouter()
const bookStore = useVocabBookStore()

const bookId = computed(() => Number(route.params.id))
const book = computed(() => bookStore.books.find((b) => b.id === bookId.value) || null)

const loading = ref(false)
const queue = ref<VocabWord[]>([])
const current = computed(() => queue.value[0] ?? null)
const revealed = ref(false)
const reviewed = ref(0)
const stats = ref({ again: 0, good: 0, easy: 0 })
const progress = ref<ReviewProgress | null>(null)

const goalPercent = computed(() => {
  if (!progress.value) return 0
  const goal = progress.value.goal || 1
  return Math.min(100, Math.round((progress.value.reviewed_today / goal) * 100))
})

async function refreshProgress() {
  try {
    progress.value = await invoke<ReviewProgress>('get_review_progress', {
      vocabBookId: bookId.value,
    })
  } catch {
    /* ignore */
  }
}

onMounted(async () => {
  if (bookStore.books.length === 0) {
    await bookStore.fetchAll()
  }
  loading.value = true
  try {
    queue.value = await invoke<VocabWord[]>('get_due_words', {
      vocabBookId: bookId.value,
    })
    await refreshProgress()
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '加载复习队列失败'))
  } finally {
    loading.value = false
  }
})

async function answer(rating: 'again' | 'good' | 'easy') {
  const card = current.value
  if (!card) return
  try {
    await invoke('review_vocab_word', { id: card.id, rating })
    stats.value[rating] += 1
    reviewed.value += 1
    queue.value = queue.value.slice(1)
    revealed.value = false
    await refreshProgress()
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '提交失败'))
  }
}

function goBack() {
  router.push(`/vocabulary/${bookId.value}`)
}
</script>

<style scoped>
.review-page {
  max-width: 720px;
  margin: 0 auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  min-height: 100%;
}
.review-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
}
.review-header h2 {
  margin: 0;
  font-size: 20px;
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
  padding: 48px 32px;
  text-align: center;
  margin-bottom: 24px;
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
.review-actions {
  display: flex;
  gap: 16px;
  justify-content: center;
}
.review-actions .el-button {
  min-width: 120px;
  height: auto;
  padding: 12px 0;
}
.btn-sub {
  font-size: 11px;
  font-weight: normal;
  opacity: 0.85;
}
</style>
