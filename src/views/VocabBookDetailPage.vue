<template>
  <div class="vocab-book-detail-page">
    <!-- Top bar -->
    <div class="page-header">
      <div class="header-left">
        <el-button link @click="goBack">
          <el-icon><ArrowLeft /></el-icon> {{ t('vocabDetail.back') }}
        </el-button>
        <h2 v-if="book">{{ book.name }}</h2>
        <span class="word-count" v-if="!store.loading">{{ t('vocabDetail.wordCount', { n: store.total }) }}</span>
      </div>
      <div class="header-right">
        <el-input
          class="header-search"
          v-model="searchQuery"
          :placeholder="t('vocabDetail.search')"
          clearable
        >
          <template #prefix>
            <el-icon><Search /></el-icon>
          </template>
        </el-input>
        <el-button type="primary" @click="showCreateDialog">
          <el-icon><Plus /></el-icon> {{ t('vocabDetail.addWord') }}
        </el-button>
        <el-button
          type="danger"
          :disabled="selectedRows.length === 0"
          @click="handleBatchDelete"
        >
          <el-icon><Delete /></el-icon> {{ t('vocabDetail.batchDelete') }}{{ selectedRows.length ? ` (${selectedRows.length})` : '' }}
        </el-button>
        <el-button type="success" @click="goReview">
          <el-icon><Reading /></el-icon> {{ t('vocabDetail.review') }}
        </el-button>
        <el-dropdown trigger="click" @command="handleExportCommand">
          <el-button :disabled="store.words.length === 0">
            <el-icon><Download /></el-icon> {{ t('vocabDetail.export') }}
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
          <el-icon><Upload /></el-icon> {{ t('vocabDetail.importCsv') }}
        </el-button>
      </div>
    </div>

    <!-- Proficiency filter (multi-select: only show checked categories) -->
    <div class="filter-tabs">
      <el-checkbox-group v-model="proficiencyFilter" size="small">
        <el-checkbox-button value="unknown">{{ t('vocabDetail.unknown') }}</el-checkbox-button>
        <el-checkbox-button value="familiar">{{ t('vocabDetail.familiar') }}</el-checkbox-button>
        <el-checkbox-button value="mastered">{{ t('vocabDetail.mastered') }}</el-checkbox-button>
        <el-button size="small" link @click="proficiencyFilter = ['unknown','familiar','mastered']">{{ t('vocabDetail.selectAll') }}</el-button>
        <el-button size="small" link @click="proficiencyFilter = ['unknown','familiar']">{{ t('vocabDetail.weakOnly') }}</el-button>
      </el-checkbox-group>
    </div>

    <!-- Word table -->
    <el-table
      ref="tableRef"
      v-loading="store.loading"
      :data="store.words"
      row-key="id"
      stripe
      style="width: 100%"
      empty-text="词汇本还没有单词，点击「添加单词」开始"
      @selection-change="handleSelectionChange"
    >
      <el-table-column type="selection" width="48" reserve-selection />
      <el-table-column prop="word" :label="t('vocabDetail.word')" min-width="120" />
      <el-table-column prop="phonetic" :label="t('vocabDetail.phonetic')" width="140">
        <template #default="{ row }">
          {{ row.phonetic || '—' }}
        </template>
      </el-table-column>
      <el-table-column prop="definition" :label="t('vocabDetail.definition')" min-width="160">
        <template #default="{ row }">
          {{ row.definition || '—' }}
        </template>
      </el-table-column>
      <el-table-column prop="exampleSentence" :label="t('vocabDetail.example')" min-width="180">
        <template #default="{ row }">
          {{ row.exampleSentence || '—' }}
        </template>
      </el-table-column>
      <el-table-column :label="t('vocabDetail.source')" min-width="150">
        <template #default="{ row }">
          <el-button
            v-if="row.novelId"
            link
            type="primary"
            size="small"
            @click="goToSource(row)"
          >
            {{ sourceLabel(row) }}
          </el-button>
          <span v-else>—</span>
        </template>
      </el-table-column>
      <el-table-column prop="proficiency" :label="t('vocabDetail.proficiency')" width="100">
        <template #default="{ row }">
          <el-tag :type="proficiencyType(row.proficiency)" size="small">
            {{ proficiencyLabel(row.proficiency) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('vocabDetail.actions')" width="140" fixed="right">
        <template #default="{ row }">
          <el-button size="small" link type="primary" @click="editWord(row)">{{ t('vocabDetail.edit') }}</el-button>
          <el-button size="small" link type="danger" @click="confirmDelete(row)">{{ t('vocabDetail.delete') }}</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-pagination
      v-model:current-page="page"
      v-model:page-size="pageSize"
      :total="store.total"
      :page-sizes="[20, 50, 100, 200]"
      layout="total, sizes, prev, pager, next, jumper"
      background
      class="pagination"
      @current-change="load"
      @size-change="onSizeChange"
    />

    <!-- Form dialog -->
    <VocabWordFormDialog
      v-model="dialogVisible"
      :word="editingWord"
      @submit="handleSubmit"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, Search, Plus, Download, Upload, Delete, ArrowDown, Reading } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { save, open } from '@tauri-apps/plugin-dialog'
import { useVocabWordStore } from '@/stores/vocabWordStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import type { VocabWord, VocabWordFormData } from '@/types/vocabWord'
import type { Chapter } from '@/types/novel'
import VocabWordFormDialog from '@/components/vocabulary/VocabWordFormDialog.vue'
import { t } from '@/i18n'

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
const page = ref(1)
const pageSize = ref(50)
const tableRef = ref<{ clearSelection: () => void } | null>(null)
const chapterTitles = ref<Record<number, string>>({})

const book = computed(() =>
  bookStore.books.find((b) => b.id === bookId.value) || null,
)

async function load() {
  await store.fetchPage(bookId.value, {
    query: searchQuery.value,
    proficiencies: proficiencyFilter.value,
    offset: (page.value - 1) * pageSize.value,
    limit: pageSize.value,
  })
  await loadSourceTitles()
}

async function loadSourceTitles() {
  const novelIds = [...new Set(store.words.map((word) => word.novelId).filter((id): id is number => Boolean(id)))]
  const next: Record<number, string> = { ...chapterTitles.value }
  await Promise.all(novelIds.map(async (novelId) => {
    try {
      const chapters = await invoke<Chapter[]>('get_chapters', { novelId })
      for (const chapter of chapters) {
        next[chapter.id] = chapter.title
      }
    } catch {
      // Source metadata is supplementary; the word remains usable if it is unavailable.
    }
  }))
  chapterTitles.value = next
}

function sourceLabel(word: VocabWord): string {
  if (word.chapterId && chapterTitles.value[word.chapterId]) {
    return chapterTitles.value[word.chapterId]
  }
  return word.chapterId ? `章节 ${word.chapterId}` : '原文'
}

let sourceWindowSequence = 0

function goToSource(word: VocabWord) {
  if (!word.novelId) return

  const params = new URLSearchParams({ focusWord: word.word.trim() })
  if (word.chapterId) params.set('focusChapterId', String(word.chapterId))
  const path = `/novels/${word.novelId}?${params.toString()}`
  const label = `source-${word.novelId}-${Date.now()}-${sourceWindowSequence++}`

  try {
    const sourceWindow = new WebviewWindow(label, {
      // Tauri app routes must be passed as relative paths so they are
      // resolved against the bundled app URL in both dev and production.
      url: path,
      title: `原文：${word.word}`,
      width: 1200,
      height: 800,
      minWidth: 800,
      minHeight: 500,
      resizable: true,
      decorations: false,
    })
    void sourceWindow.once('tauri://error', (event) => {
      console.error('[VocabBookDetailPage] source window failed:', event.payload)
      ElMessage.error('打开原文窗口失败')
    })
  } catch (error) {
    console.error('[VocabBookDetailPage] source window creation failed:', error)
    ElMessage.error('打开原文窗口失败')
  }
}

function onSizeChange() {
  page.value = 1
  load()
}

let searchTimer: ReturnType<typeof setTimeout> | null = null
watch(searchQuery, () => {
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    page.value = 1
    load()
  }, 300)
})

watch(proficiencyFilter, () => {
  page.value = 1
  load()
})

onMounted(async () => {
  // ensure book store has data so we can display the book name
  if (bookStore.books.length === 0) {
    await bookStore.fetchAll()
  }
  load()
})

function proficiencyType(p: string): 'danger' | 'warning' | 'success' {
  if (p === 'mastered') return 'success'
  if (p === 'familiar') return 'warning'
  return 'danger'
}

function proficiencyLabel(p: string): string {
  if (p === 'mastered') return t('vocabDetail.mastered')
  if (p === 'familiar') return t('vocabDetail.familiar')
  return t('vocabDetail.unknown')
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
    await load()
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
    await load()
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
    tableRef.value?.clearSelection()
    ElMessage.success(`已删除 ${count} 个单词`)
    await load()
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
    await load()
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '导入失败'))
  }
}
</script>

<style scoped>
.vocab-book-detail-page {
  width: 100%;
  min-width: 0;
  min-height: 100%;
  padding: clamp(4px, 1.5vw, 20px);
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 16px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
  flex-wrap: wrap;
}

.header-left h2 {
  margin: 0;
  font-size: 20px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
.header-search {
  width: min(100%, 240px);
}

.filter-tabs {
  margin-bottom: 16px;
}

.pagination {
  margin-top: 16px;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 6px 0;
}

@media (max-width: 760px) {
  .page-header {
    align-items: stretch;
    flex-direction: column;
  }
  .header-left {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
  }
  .header-left .word-count {
    grid-column: 1 / -1;
  }
  .header-right {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .header-search {
    width: 100%;
    grid-column: 1 / -1;
  }
  .header-right .el-button,
  .header-right :deep(.el-dropdown),
  .header-right :deep(.el-dropdown .el-button) {
    width: 100%;
    margin-left: 0;
  }
  .filter-tabs :deep(.el-checkbox-group) {
    display: flex;
    flex-wrap: wrap;
  }
  .pagination :deep(.el-pagination__sizes),
  .pagination :deep(.el-pagination__jump) {
    display: none;
  }
}

@media (max-width: 440px) {
  .header-right {
    grid-template-columns: 1fr;
  }
  .pagination :deep(.el-pagination__total) {
    display: none;
  }
}
</style>
