<template>
  <div class="vocab-book-detail-page">
    <!-- Top bar -->
    <div class="page-header">
      <div class="header-left">
        <el-button link @click="goBack">
          <el-icon><ArrowLeft /></el-icon> 返回
        </el-button>
        <h2 v-if="book">{{ book.name }}</h2>
        <span class="word-count" v-if="!store.loading">{{ filteredWords.length }} 词</span>
      </div>
      <div class="header-right">
        <el-input
          v-model="searchQuery"
          placeholder="搜索单词..."
          clearable
          style="width: 200px"
        >
          <template #prefix>
            <el-icon><Search /></el-icon>
          </template>
        </el-input>
        <el-button type="primary" @click="showCreateDialog">
          <el-icon><Plus /></el-icon> 添加单词
        </el-button>
      </div>
    </div>

    <!-- Proficiency tabs -->
    <div class="filter-tabs">
      <el-radio-group v-model="proficiencyFilter" size="small">
        <el-radio-button value="all">全部</el-radio-button>
        <el-radio-button value="unknown">生疏</el-radio-button>
        <el-radio-button value="familiar">熟悉</el-radio-button>
        <el-radio-button value="mastered">已掌握</el-radio-button>
      </el-radio-group>
    </div>

    <!-- Word table -->
    <el-table
      v-loading="store.loading"
      :data="filteredWords"
      stripe
      style="width: 100%"
      empty-text="词汇本还没有单词，点击「添加单词」开始"
    >
      <el-table-column prop="word" label="单词" min-width="120" />
      <el-table-column prop="phonetic" label="音标" width="140">
        <template #default="{ row }">
          {{ row.phonetic || '—' }}
        </template>
      </el-table-column>
      <el-table-column prop="definition" label="释义" min-width="160">
        <template #default="{ row }">
          {{ row.definition || '—' }}
        </template>
      </el-table-column>
      <el-table-column prop="exampleSentence" label="例句" min-width="180">
        <template #default="{ row }">
          {{ row.exampleSentence || '—' }}
        </template>
      </el-table-column>
      <el-table-column prop="proficiency" label="熟练度" width="100">
        <template #default="{ row }">
          <el-tag :type="proficiencyType(row.proficiency)" size="small">
            {{ proficiencyLabel(row.proficiency) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="140" fixed="right">
        <template #default="{ row }">
          <el-button size="small" link type="primary" @click="editWord(row)">编辑</el-button>
          <el-button size="small" link type="danger" @click="confirmDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- Form dialog -->
    <VocabWordFormDialog
      v-model="dialogVisible"
      :word="editingWord"
      @submit="handleSubmit"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, Search, Plus } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useVocabWordStore } from '@/stores/vocabWordStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import type { VocabWord, VocabWordFormData } from '@/types/vocabWord'
import VocabWordFormDialog from '@/components/vocabulary/VocabWordFormDialog.vue'

const route = useRoute()
const router = useRouter()
const store = useVocabWordStore()
const bookStore = useVocabBookStore()

const bookId = computed(() => Number(route.params.id))

const searchQuery = ref('')
const proficiencyFilter = ref<'all' | 'unknown' | 'familiar' | 'mastered'>('all')
const dialogVisible = ref(false)
const editingWord = ref<VocabWord | null>(null)

const book = computed(() =>
  bookStore.books.find((b) => b.id === bookId.value) || null,
)

const filteredWords = computed(() => {
  let list = store.words
  // filter by proficiency
  if (proficiencyFilter.value !== 'all') {
    list = list.filter((w) => w.proficiency === proficiencyFilter.value)
  }
  // filter by search query
  const q = searchQuery.value.trim().toLowerCase()
  if (q) {
    list = list.filter(
      (w) =>
        w.word.toLowerCase().includes(q) ||
        w.definition.toLowerCase().includes(q) ||
        w.phonetic.toLowerCase().includes(q),
    )
  }
  return list
})

onMounted(async () => {
  // ensure book store has data so we can display the book name
  if (bookStore.books.length === 0) {
    await bookStore.fetchAll()
  }
  store.fetchAll(bookId.value)
})

function proficiencyType(p: string): 'info' | 'warning' | 'success' {
  if (p === 'mastered') return 'success'
  if (p === 'familiar') return 'warning'
  return 'info'
}

function proficiencyLabel(p: string): string {
  if (p === 'mastered') return '已掌握'
  if (p === 'familiar') return '熟悉'
  return '生疏'
}

function showCreateDialog() {
  editingWord.value = null
  dialogVisible.value = true
}

function editWord(word: VocabWord) {
  editingWord.value = word
  dialogVisible.value = true
}

async function handleSubmit(data: VocabWordFormData) {
  try {
    if (editingWord.value) {
      await store.update(editingWord.value.id, data)
      ElMessage.success('单词已更新')
    } else {
      await store.create(bookId.value, data)
      ElMessage.success('单词已添加')
    }
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '操作失败'))
  }
}

async function confirmDelete(word: VocabWord) {
  try {
    await ElMessageBox.confirm(
      `确定删除单词「${word.word}」吗？`,
      '确认删除',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' },
    )
    await store.remove(word.id)
    ElMessage.success('已删除')
  } catch {
    // user cancelled
  }
}

function goBack() {
  router.push('/vocabulary')
}
</script>

<style scoped>
.vocab-book-detail-page {
  padding: 24px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-left h2 {
  margin: 0;
  font-size: 20px;
}

.word-count {
  color: var(--text-secondary, #909399);
  font-size: 13px;
}

.header-right {
  display: flex;
  gap: 12px;
}

.filter-tabs {
  margin-bottom: 16px;
}
</style>
