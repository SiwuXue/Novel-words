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
        <el-button
          type="danger"
          :disabled="selectedRows.length === 0"
          @click="handleBatchDelete"
        >
          <el-icon><Delete /></el-icon> 批量删除{{ selectedRows.length ? ` (${selectedRows.length})` : '' }}
        </el-button>
        <el-button type="success" @click="goReview">
          <el-icon><Reading /></el-icon> 卡片复习
        </el-button>
        <el-dropdown trigger="click" @command="handleExportCommand">
          <el-button :disabled="store.words.length === 0">
            <el-icon><Download /></el-icon> 导出
            <el-icon class="el-icon--right"><ArrowDown /></el-icon>
          </el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="csv">CSV (.csv)</el-dropdown-item>
              <el-dropdown-item command="xlsx">Excel (.xlsx)</el-dropdown-item>
              <el-dropdown-item command="apkg">Anki (.apkg)</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
        <el-button @click="handleImportCsv">
          <el-icon><Upload /></el-icon> 导入 CSV
        </el-button>
      </div>
    </div>

    <!-- Proficiency filter (multi-select: only show checked categories) -->
    <div class="filter-tabs">
      <el-checkbox-group v-model="proficiencyFilter" size="small">
        <el-checkbox-button value="unknown">生疏</el-checkbox-button>
        <el-checkbox-button value="familiar">熟悉</el-checkbox-button>
        <el-checkbox-button value="mastered">已掌握</el-checkbox-button>
        <el-button size="small" link @click="proficiencyFilter = ['unknown','familiar','mastered']">全选</el-button>
        <el-button size="small" link @click="proficiencyFilter = ['unknown','familiar']">只看薄弱</el-button>
      </el-checkbox-group>
    </div>

    <!-- Word table -->
    <el-table
      v-loading="store.loading"
      :data="filteredWords"
      stripe
      style="width: 100%"
      empty-text="词汇本还没有单词，点击「添加单词」开始"
      @selection-change="handleSelectionChange"
    >
      <el-table-column type="selection" width="48" />
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
import { ArrowLeft, Search, Plus, Download, Upload, Delete, ArrowDown, Reading } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { save, open } from '@tauri-apps/plugin-dialog'
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
const proficiencyFilter = ref<('unknown' | 'familiar' | 'mastered')[]>([
  'unknown',
  'familiar',
])
const dialogVisible = ref(false)
const editingWord = ref<VocabWord | null>(null)
const selectedRows = ref<VocabWord[]>([])

const book = computed(() =>
  bookStore.books.find((b) => b.id === bookId.value) || null,
)

const filteredWords = computed(() => {
  let list = store.words
  // filter by proficiency (multi-select: only show checked)
  if (proficiencyFilter.value.length > 0) {
    list = list.filter((w) =>
      proficiencyFilter.value.includes(w.proficiency as 'unknown' | 'familiar' | 'mastered'),
    )
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

function proficiencyType(p: string): 'danger' | 'warning' | 'success' {
  if (p === 'mastered') return 'success'
  if (p === 'familiar') return 'warning'
  return 'danger'
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

function handleSelectionChange(rows: VocabWord[]) {
  selectedRows.value = rows
}

async function handleBatchDelete() {
  const rows = selectedRows.value
  if (rows.length === 0) return
  try {
    await ElMessageBox.confirm(
      `确定删除选中的 ${rows.length} 个单词吗？`,
      '批量删除',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' },
    )
    const count = await store.removeMany(rows.map((r) => r.id))
    selectedRows.value = []
    ElMessage.success(`已删除 ${count} 个单词`)
  } catch {
    // user cancelled
  }
}

function goBack() {
  router.push('/vocabulary')
}

function goReview() {
  router.push(`/vocabulary/${bookId.value}/review`)
}

async function handleExportCommand(cmd: string) {
  if (cmd === 'csv') {
    await handleExportCsv()
  } else if (cmd === 'xlsx') {
    await handleExportXlsx()
  } else if (cmd === 'apkg') {
    await handleExportApkg()
  }
}

async function handleExportXlsx() {
  try {
    const filePath = await save({
      filters: [{ name: 'Excel', extensions: ['xlsx'] }],
      defaultPath: `${book.value?.name || 'words'}.xlsx`,
    })
    if (!filePath) return
    await invoke('export_vocab_words_xlsx', {
      vocabBookId: bookId.value,
      filePath,
    })
    ElMessage.success('Excel 导出成功')
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '导出失败'))
  }
}

async function handleExportApkg() {
  try {
    const filePath = await save({
      filters: [{ name: 'Anki 卡组', extensions: ['apkg'] }],
      defaultPath: `${book.value?.name || 'words'}.apkg`,
    })
    if (!filePath) return
    await invoke('export_vocab_words_apkg', {
      vocabBookId: bookId.value,
      deckName: book.value?.name || '词阅单词',
      filePath,
    })
    ElMessage.success('Anki 卡组导出成功，可在 Anki 中导入')
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '导出失败'))
  }
}

async function handleExportCsv() {
  try {
    const filePath = await save({
      filters: [{ name: 'CSV', extensions: ['csv'] }],
      defaultPath: `${book.value?.name || 'words'}.csv`,
    })
    if (!filePath) return // user cancelled

    await invoke('export_vocab_words_csv', {
      vocabBookId: bookId.value,
      filePath,
    })
    ElMessage.success('导出成功')
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '导出失败'))
  }
}

async function handleImportCsv() {
  try {
    const filePath = await open({
      filters: [{ name: 'CSV', extensions: ['csv'] }],
      multiple: false,
    })
    if (!filePath) return // user cancelled

    const result = await invoke<{ imported: number; skipped: number }>(
      'import_vocab_words_csv',
      {
        vocabBookId: bookId.value,
        filePath,
      },
    )
    const msg = result.skipped > 0
      ? `已导入 ${result.imported} 个单词，跳过 ${result.skipped} 个重复`
      : `已导入 ${result.imported} 个单词`
    ElMessage.success(msg)
    await store.fetchAll(bookId.value)
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '导入失败'))
  }
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
  align-items: center;
  flex-wrap: wrap;
}

.filter-tabs {
  margin-bottom: 16px;
}
</style>
