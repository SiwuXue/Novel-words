<template>
  <div class="vocab-book-list-page">
    <div class="page-header">
      <h2>{{ t('vocabList.title') }}</h2>
      <div class="header-actions">
        <el-input
          class="header-search"
          v-model="searchQuery"
          :placeholder="t('vocabList.search')"
          clearable
        >
          <template #prefix>
            <el-icon><Search /></el-icon>
          </template>
        </el-input>
        <el-button type="primary" @click="showCreateDialog">
          <el-icon><Plus /></el-icon> {{ t('vocabList.new') }}
        </el-button>
      </div>
    </div>

    <el-table
      v-loading="store.loading"
      :data="filteredBooks"
      stripe
      style="width: 100%"
      :empty-text="t('vocabList.empty')"
    >
      <el-table-column prop="name" :label="t('vocabList.name')" min-width="160">
        <template #default="{ row }">
          <el-link type="primary" @click="openDetail(row.id)">{{ row.name }}</el-link>
          <el-tag v-if="row.isPreset" size="small" type="warning" style="margin-left:6px;">
            {{ t('preset.preset') }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="description" :label="t('vocabList.description')" min-width="200">
        <template #default="{ row }">
          {{ row.description || '—' }}
        </template>
      </el-table-column>
      <el-table-column prop="updatedAt" :label="t('vocabList.updatedAt')" width="170">
        <template #default="{ row }">
          {{ formatDate(row.updatedAt) }}
        </template>
      </el-table-column>
      <el-table-column :label="t('novelList.actions')" width="200" fixed="right">
        <template #default="{ row }">
          <el-button size="small" link type="primary" :disabled="row.isPreset" @click="editBook(row)">{{ t('vocabList.edit') }}</el-button>
          <el-button size="small" link @click="exportJson(row)">{{ t('vocabList.export') }}</el-button>
          <el-button size="small" link @click="importJson(row)">{{ t('vocabList.import') }}</el-button>
          <el-button size="small" link type="danger" :disabled="row.isPreset" @click="confirmDelete(row)">{{ t('vocabList.delete') }}</el-button>
        </template>
      </el-table-column>
    </el-table>

    <VocabBookFormDialog
      v-model="dialogVisible"
      :book="editingBook"
      @submit="handleSubmit"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { Search, Plus } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import type { VocabBook, VocabBookFormData } from '@/types/vocabBook'
import VocabBookFormDialog from '@/components/vocabulary/VocabBookFormDialog.vue'
import { t } from '@/i18n'

const router = useRouter()
const store = useVocabBookStore()

const searchQuery = ref('')
const dialogVisible = ref(false)
const editingBook = ref<VocabBook | null>(null)

const filteredBooks = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  // Preset books (CET4 etc.) live on the /presets page; hide them here.
  const userBooks = store.books.filter((b) => !b.isPreset)
  if (!q) return userBooks
  return userBooks.filter(
    (b) =>
      b.name.toLowerCase().includes(q) ||
      b.description.toLowerCase().includes(q),
  )
})

onMounted(() => {
  store.fetchAll()
})

function showCreateDialog() {
  editingBook.value = null
  dialogVisible.value = true
}

function editBook(book: VocabBook) {
  editingBook.value = book
  dialogVisible.value = true
}

async function handleSubmit(data: VocabBookFormData) {
  try {
    if (editingBook.value) {
      await store.update(editingBook.value.id, data)
      ElMessage.success('词汇本已更新')
    } else {
      await store.create(data)
      ElMessage.success('词汇本已创建')
    }
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '操作失败'))
  }
}

/** 导出词汇本为 JSON 文件（轻量便携格式，便于分享/多设备迁移）。 */
async function exportJson(book: VocabBook) {
  try {
    const json = await invoke<string>('export_vocab_book_json', { vocabBookId: book.id })
    const dest = await save({
      defaultPath: `${book.name || '词汇本'}.json`,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!dest) return
    await invoke('write_text_file', { path: dest, contents: json })
    ElMessage.success(`已导出 ${book.name} 的单词到 JSON`)
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '导出失败'))
  }
}

/** 从 JSON 文件导入单词到词汇本（按单词去重）。 */
async function importJson(book: VocabBook) {
  try {
    const src = await open({
      multiple: false,
      filters: [{ name: 'JSON', extensions: ['json'] }],
    })
    if (!src || typeof src !== 'string') return
    const json = await invoke<string>('read_text_file', { path: src })
    const inserted = await invoke<number>('import_vocab_book_json', {
      vocabBookId: book.id,
      json,
    })
    ElMessage.success(`已导入 ${inserted} 个新单词（重复自动跳过）`)
    store.fetchAll()
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '导入失败'))
  }
}

async function confirmDelete(book: VocabBook) {
  try {
    await ElMessageBox.confirm(
      `确定删除词汇本「${book.name}」吗？关联的生词也会被删除。`,
      '确认删除',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' },
    )
    await store.remove(book.id)
    ElMessage.success('已删除')
  } catch {
    // user cancelled
  }
}

function openDetail(id: number) {
  router.push(`/vocabulary/${id}`)
}

function formatDate(raw: string): string {
  if (!raw) return ''
  try {
    const d = new Date(raw)
    if (Number.isNaN(d.getTime())) return raw
    return d.toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    })
  } catch {
    return raw
  }
}
</script>

<style scoped>
.vocab-book-list-page {
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
  margin-bottom: 20px;
}

.page-header h2 {
  margin: 0;
  font-size: 20px;
}

.header-actions {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  row-gap: 8px;
}
.header-search {
  width: min(100%, 280px);
}

@media (max-width: 640px) {
  .page-header {
    align-items: stretch;
    flex-direction: column;
  }
  .header-actions {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
  }
  .header-search {
    width: 100%;
  }
  .header-actions .el-button {
    margin-left: 0;
  }
}

@media (max-width: 420px) {
  .header-actions {
    grid-template-columns: 1fr;
  }
  .header-actions .el-button {
    width: 100%;
  }
}
</style>
