<template>
  <div class="stats-page">
    <h2>学习统计</h2>

    <div v-if="loading" class="stats-loading">
      <el-icon class="is-loading" :size="28"><Loading /></el-icon>
      <span>加载中…</span>
    </div>

    <template v-else-if="stats">
      <!-- Top summary cards -->
      <div class="stats-cards">
        <div class="stat-card">
          <div class="stat-num">{{ stats.total_words }}</div>
          <div class="stat-label">生词总数</div>
        </div>
        <div class="stat-card">
          <div class="stat-num">{{ stats.total_books }}</div>
          <div class="stat-label">词汇本</div>
        </div>
        <div class="stat-card">
          <div class="stat-num">{{ stats.total_novels }}</div>
          <div class="stat-label">小说</div>
        </div>
        <div class="stat-card">
          <div class="stat-num accent">{{ stats.reviewed_today }}</div>
          <div class="stat-label">今日已复习</div>
        </div>
        <div class="stat-card">
          <div class="stat-num">{{ stats.total_reviews }}</div>
          <div class="stat-label">累计复习次数</div>
        </div>
      </div>

      <!-- Proficiency distribution -->
      <el-card class="stats-section">
        <template #header>
          <span class="section-title">熟练度分布</span>
        </template>
        <div class="prof-bars">
          <div class="prof-row">
            <span class="prof-label prof-unknown">生疏</span>
            <el-progress
              class="prof-progress"
              :percentage="profPercent('unknown')"
              :stroke-width="14"
              :color="'#f56c6c'"
              :format="(v: number) => `${v}`"
            />
            <span class="prof-count">{{ stats.by_proficiency.unknown || 0 }}</span>
          </div>
          <div class="prof-row">
            <span class="prof-label prof-familiar">熟悉</span>
            <el-progress
              class="prof-progress"
              :percentage="profPercent('familiar')"
              :stroke-width="14"
              :color="'#e6a23c'"
              :format="(v: number) => `${v}`"
            />
            <span class="prof-count">{{ stats.by_proficiency.familiar || 0 }}</span>
          </div>
          <div class="prof-row">
            <span class="prof-label prof-mastered">掌握</span>
            <el-progress
              class="prof-progress"
              :percentage="profPercent('mastered')"
              :stroke-width="14"
              :color="'#67c23a'"
              :format="(v: number) => `${v}`"
            />
            <span class="prof-count">{{ stats.by_proficiency.mastered || 0 }}</span>
          </div>
        </div>
      </el-card>

      <!-- Last 7 days reviews -->
      <el-card class="stats-section">
        <template #header>
          <span class="section-title">最近 7 天复习量</span>
        </template>
        <div class="week-bars">
          <div v-for="d in stats.reviews_last_7_days" :key="d.date" class="week-bar-wrap">
            <div class="week-bar-track">
              <div
                class="week-bar-fill"
                :style="{ height: `${weekBarHeight(d.count)}%` }"
              ></div>
            </div>
            <div class="week-bar-count">{{ d.count }}</div>
            <div class="week-bar-date">{{ formatShortDate(d.date) }}</div>
          </div>
        </div>
      </el-card>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Loading } from '@element-plus/icons-vue'

interface DailyReviewCount {
  date: string
  count: number
}

interface LearningStats {
  total_words: number
  total_books: number
  total_novels: number
  by_proficiency: Record<string, number>
  reviewed_today: number
  total_reviews: number
  reviews_last_7_days: DailyReviewCount[]
}

const loading = ref(true)
const stats = ref<LearningStats | null>(null)

function profPercent(key: string): number {
  if (!stats.value || stats.value.total_words === 0) return 0
  const n = stats.value.by_proficiency[key] || 0
  return Math.round((n / stats.value.total_words) * 100)
}

const maxWeekCount = computed(() => {
  if (!stats.value) return 0
  return stats.value.reviews_last_7_days.reduce((m, d) => Math.max(m, d.count), 0)
})

function weekBarHeight(count: number): number {
  const max = maxWeekCount.value
  if (max <= 0) return 0
  // Use a minimum of 4% so a non-zero bar is visible.
  return Math.max(4, Math.round((count / max) * 100))
}

function formatShortDate(iso: string): string {
  // "2026-03-12" → "3/12"
  const parts = iso.split('-')
  if (parts.length !== 3) return iso
  return `${parseInt(parts[1], 10)}/${parseInt(parts[2], 10)}}`
}

onMounted(async () => {
  try {
    stats.value = await invoke<LearningStats>('get_learning_stats')
  } catch (e) {
    console.error('[stats] get_learning_stats failed:', e)
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.stats-page {
  width: 100%;
  max-width: 960px;
  margin: 0 auto;
  padding: 24px;
}
.stats-page h2 {
  margin: 0 0 20px;
  font-size: 20px;
}
.stats-loading {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-secondary);
  padding: 40px;
  justify-content: center;
}

.stats-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: 16px;
  margin-bottom: 20px;
}
.stat-card {
  background: var(--bg-secondary, #f5f7fa);
  border-radius: 10px;
  padding: 18px 16px;
  text-align: center;
}
.stat-num {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-regular, #303133);
  line-height: 1.1;
}
.stat-num.accent {
  color: var(--accent-color, #409eff);
}
.stat-label {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-secondary, #909399);
}

.stats-section {
  margin-bottom: 16px;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
}

.prof-bars {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.prof-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.prof-label {
  width: 40px;
  font-size: 13px;
  font-weight: 600;
  text-align: right;
}
.prof-unknown { color: var(--danger-color, #f56c6c); }
.prof-familiar { color: var(--warning-color, #e6a23c); }
.prof-mastered { color: var(--success-color, #67c23a); }
.prof-progress {
  flex: 1;
}
.prof-count {
  width: 48px;
  font-size: 13px;
  color: var(--text-secondary, #909399);
  text-align: right;
}

.week-bars {
  display: flex;
  gap: 12px;
  align-items: flex-end;
  height: 180px;
  padding: 0 4px;
}
.week-bar-wrap {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  height: 100%;
}
.week-bar-track {
  flex: 1;
  width: 100%;
  background: var(--bg-secondary, #f5f7fa);
  border-radius: 4px 4px 0 0;
  display: flex;
  align-items: flex-end;
  overflow: hidden;
}
.week-bar-fill {
  width: 100%;
  background: linear-gradient(180deg, #67c23a 0%, #409eff 100%);
  border-radius: 4px 4px 0 0;
  min-height: 2px;
  transition: height 0.3s ease;
}
.week-bar-count {
  font-size: 12px;
  color: var(--text-regular);
  font-weight: 600;
}
.week-bar-date {
  font-size: 11px;
  color: var(--text-secondary);
}
</style>
