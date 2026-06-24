<template>
  <div class="vocab-book-list-page">
    <div class="page-header">
      <h2>词汇本</h2>
      <div class="header-actions">
        <el-input
          v-model="searchQuery"
          placeholder="搜索词汇本名称..."
          clearable
          style="width: 240px"
        >
          <template #prefix>
            <el-icon><Search /></el-icon>
          </template>
        </el-input>
        <el-button type="primary" @click="showCreateDialog">
          <el-icon><Plus /></el-icon> 新建词汇本
        </el-button>
      </div>
    </div>

    <el-table
      v-loading="store.loading"
      :data="filteredBooks"
      stripe
      style="width: 100%"
      empty-text="还没有词汇本，点击上方按钮创建"
    >
      <el-table-column prop="name" label="名称" min-width="160">
        <template #default="{ row }">
          <el-link type="primary" @click="openDetail(row.id)">{{ row.name }}</el-link>
        </template>
      </el-table-column>
      <el-table-column prop="description" label="描述" min-width="200">
        <template #default="{ row }">
          {{ row.description || '—' }}
        </template>
      </el-table-column>
      <el-table-column prop="updatedAt" label="更新时间" width="170">
        <template #default="{ row }">
          {{ formatDate(row.updatedAt) }}
        </template>
      </el-table-column>
      <el-table-column label="操作" width="140" fixed="right">
        <template #default="{ row }">
          <el-button size="small" link type="primary" @click="editBook(row)">编辑</el-button>
          <el-button size="small" link type="danger" @click="confirmDelete(row)">删除</el-button>
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
import { useVocabBookStore } from '@/stores/vocabBookStore'
import type { VocabBook, VocabBookFormData } from '@/types/vocabBook'
import VocabBookFormDialog from '@/components/vocabulary/VocabBookFormDialog.vue'

const router = useRouter()
const store = useVocabBookStore()

const searchQuery = ref('')
const dialogVisible = ref(false)
const editingBook = ref<VocabBook | null>(null)

const filteredBooks = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return store.books
  return store.books.filter(
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
  padding: 24px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.page-header h2 {
  margin: 0;
  font-size: 20px;
}

.header-actions {
  display: flex;
  gap: 12px;
}
</style>
