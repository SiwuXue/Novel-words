<template>
  <div class="home-page">
    <div class="hero">
      <h1>词阅</h1>
      <p class="subtitle">本地小说阅读与词汇管理，助力外语学习</p>
    </div>

    <div class="stats-row">
      <div class="stat-card" @click="$router.push('/novels')">
        <el-icon :size="28"><Document /></el-icon>
        <div class="stat-num">{{ novelCount }}</div>
        <div class="stat-label">小说</div>
      </div>
      <div class="stat-card" @click="$router.push('/vocabulary')">
        <el-icon :size="28"><Collection /></el-icon>
        <div class="stat-num">{{ bookCount }}</div>
        <div class="stat-label">词汇本</div>
      </div>
      <div class="stat-card">
        <el-icon :size="28"><Notebook /></el-icon>
        <div class="stat-num">{{ wordCount }}</div>
        <div class="stat-label">生词</div>
      </div>
    </div>

    <div class="recent-section" v-if="recentNovels.length">
      <h3>最近阅读</h3>
      <div class="recent-list">
        <div
          v-for="n in recentNovels"
          :key="n.id"
          class="recent-item"
          @click="$router.push(`/novels/${n.id}`)"
        >
          <span class="recent-title">{{ n.title || '未命名' }}</span>
          <span class="recent-author">{{ n.author || '未知作者' }}</span>
          <el-icon class="recent-arrow"><ArrowRight /></el-icon>
        </div>
      </div>
    </div>

    <div class="quick-actions">
      <h3>快捷操作</h3>
      <div class="actions-row">
        <el-button type="primary" size="large" @click="$router.push('/novels/new')">
          <el-icon><Plus /></el-icon> 导入小说
        </el-button>
        <el-button size="large" @click="$router.push('/vocabulary')">
          <el-icon><Collection /></el-icon> 词汇本
        </el-button>
        <el-button size="large" @click="$router.push('/settings')">
          <el-icon><Setting /></el-icon> 设置
        </el-button>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Document, Collection, Notebook, Plus, Setting, ArrowRight } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import type { VocabBook } from '@/types/vocabBook'
import type { VocabWord } from '@/types/vocabWord'
import type { Novel } from '@/types/novel'

const novelCount = ref(0)
const bookCount = ref(0)
const wordCount = ref(0)
const recentNovels = ref<Novel[]>([])

onMounted(async () => {
  try {
    const novels = await invoke<Novel[]>('get_all_novels')
    novelCount.value = novels.length
    recentNovels.value = novels.slice(0, 3)
  } catch { /* ignore */ }

  try {
    const books = await invoke<VocabBook[]>('get_all_vocab_books')
    bookCount.value = books.length

    // Count words across all books
    const results = await Promise.allSettled(
      books.map((b) => invoke<VocabWord[]>('get_vocab_words', { vocabBookId: b.id })),
    )
    wordCount.value = results.reduce((sum, r) => {
      if (r.status === 'fulfilled') return sum + r.value.length
      return sum
    }, 0)
  } catch { /* ignore */ }
})
</script>

<style scoped>
.home-page {
  padding: 48px 40px;
  max-width: 720px;
  margin: 0 auto;
}

.hero {
  text-align: center;
  margin-bottom: 40px;
}

.hero h1 {
  font-size: 28px;
  font-weight: 700;
  margin: 0 0 8px 0;
  color: var(--text-regular, #303133);
}

.subtitle {
  font-size: 15px;
  color: var(--text-secondary, #909399);
  margin: 0;
}

.stats-row {
  display: flex;
  gap: 20px;
  justify-content: center;
  margin-bottom: 40px;
}

.stat-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 24px 32px;
  border-radius: 12px;
  background: var(--bg-secondary, #f5f7fa);
  cursor: pointer;
  transition: background 0.2s, transform 0.2s;
  user-select: none;
}

.stat-card:hover {
  background: var(--accent-light, #ecf5ff);
  transform: translateY(-2px);
}

.stat-card .el-icon {
  color: var(--accent-color, #409eff);
}

.stat-num {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-regular, #303133);
  line-height: 1;
}

.stat-label {
  font-size: 13px;
  color: var(--text-secondary, #909399);
}

.recent-section {
  margin-bottom: 32px;
}

.recent-section h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 12px 0;
  color: var(--text-regular, #303133);
}

.recent-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.recent-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-radius: 10px;
  background: var(--bg-secondary, #f5f7fa);
  cursor: pointer;
  transition: background 0.2s, transform 0.2s;
  user-select: none;
}

.recent-item:hover {
  background: var(--accent-light, #ecf5ff);
  transform: translateY(-1px);
}

.recent-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-regular, #303133);
  max-width: 60%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-author {
  font-size: 12px;
  color: var(--text-secondary, #909399);
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-arrow {
  color: var(--text-placeholder, #c0c4cc);
}

.quick-actions {
  text-align: center;
  margin-bottom: 8px;
}

.quick-actions h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 16px 0;
  color: var(--text-regular, #303133);
}

.actions-row {
  display: flex;
  gap: 12px;
  justify-content: center;
  flex-wrap: wrap;
}

</style>
