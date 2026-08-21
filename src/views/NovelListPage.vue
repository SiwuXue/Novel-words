<template>
  <div class="novel-list-page">
    <div class="page-header">
      <h2>小说库</h2>
      <div class="header-actions">
        <el-input
          v-model="searchQuery"
          placeholder="搜索书名、作者、分类..."
          clearable
          style="width: 240px"
          @input="onSearch"
        >
          <template #prefix>
            <el-icon><Search /></el-icon>
          </template>
        </el-input>
        <el-button type="primary" @click="showCreateDialog">
          <el-icon><Plus /></el-icon> 新建小说
        </el-button>
        <el-button @click="showImportDialog">
          <el-icon><FolderOpened /></el-icon> 导入文件
        </el-button>
      </div>
    </div>

    <!-- Table -->
    <el-table
      v-loading="store.loading"
      :data="store.novels"
      stripe
      style="width: 100%"
      empty-text="还没有小说，点击上方按钮创建或导入"
    >
      <el-table-column prop="title" label="书名" min-width="160">
        <template #default="{ row }">
          <el-link type="primary" @click="openNovel(row.id)">{{ row.title }}</el-link>
        </template>
      </el-table-column>
      <el-table-column prop="author" label="作者" width="140" />
      <el-table-column prop="category" label="分类" width="80">
        <template #default="{ row }">
          <el-tag size="small" type="info">{{ row.category }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="updatedAt" label="更新时间" width="170">
        <template #default="{ row }">
          {{ formatDate(row.updatedAt) }}
        </template>
      </el-table-column>
      <el-table-column label="收藏" width="70" align="center">
        <template #default="{ row }">
          <el-icon
            :class="{ 'is-favorite': row.isFavorite }"
            class="fav-icon"
            @click="toggleFavorite(row)"
          >
            <StarFilled v-if="row.isFavorite" />
            <Star v-else />
          </el-icon>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="140" fixed="right">
        <template #default="{ row }">
          <el-button size="small" link type="primary" @click="editNovel(row)">编辑</el-button>
          <el-button size="small" link type="danger" @click="confirmDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- Create/Edit Dialog -->
    <NovelFormDialog
      v-model="dialogVisible"
      :novel="editingNovel"
      @submit="handleSubmit"
    />

    <!-- Import Dialog -->
    <ImportDialog
      v-if="showImport"
      @confirm="handleImportConfirm"
      @close="showImport = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Plus, Star, StarFilled, FolderOpened } from '@element-plus/icons-vue'
import { useNovelStore } from '@/stores/novelStore'
import type { Novel, ImportResult } from '@/types/novel'
import NovelFormDialog from '@/components/novel/NovelFormDialog.vue'
import ImportDialog from '@/components/novel/ImportDialog.vue'

const router = useRouter()
const store = useNovelStore()

const searchQuery = ref('')
const dialogVisible = ref(false)
const editingNovel = ref<Novel | null>(null)
const showImport = ref(false)

onMounted(() => {
  store.fetchAll()
})

function onSearch() {
  store.search(searchQuery.value)
}

function showCreateDialog() {
  editingNovel.value = null
  dialogVisible.value = true
}

function showImportDialog() {
  showImport.value = true
}

async function handleImportConfirm(result: ImportResult, _filePath: string) {
  try {
    const novel = await store.create({
      title: result.detectedTitle || '未命名小说',
      author: '',
      category: '其他',
      rawText: result.rawText,
      cleanedText: result.cleanedText,
    })
    // Save chapters to DB
    try {
      const chapters = result.chapters.map((ch, i) => ({
        id: 0,
        novelId: novel.id,
        title: ch.title,
        content: ch.content || result.cleanedText.slice(ch.startIndex),
        sortOrder: i,
        startIndex: ch.startIndex,
        createdAt: '',
      }))
      await invoke('save_chapters', { novelId: novel.id, chapters })
    } catch (e) {
      console.error('[NovelListPage] Failed to save chapters:', e)
    }
    showImport.value = false
    if (novel) {
      router.push(`/novels/${novel.id}`)
    }
  } catch (e: any) {
    ElMessage.error(typeof e === 'string' ? e : (e?.message || '导入失败'))
  }
}

function editNovel(novel: Novel) {
  editingNovel.value = novel
  dialogVisible.value = true
}

async function handleSubmit(data: { title: string; author: string; category: string; rawText: string }) {
  if (editingNovel.value) {
    // Only update metadata fields; preserve rawText and cleanedText
    await store.update(editingNovel.value.id, {
      title: data.title,
      author: data.author,
      category: data.category,
    })
    ElMessage.success('小说已更新')
  } else {
    const novel = await store.create(data)
    ElMessage.success('小说已创建')
    // Navigate to editor
    router.push(`/novels/${novel.id}`)
  }
  editingNovel.value = null
}

function openNovel(id: number) {
  router.push(`/novels/${id}`)
}

async function toggleFavorite(novel: Novel) {
  await store.update(novel.id, { isFavorite: !novel.isFavorite })
}

async function confirmDelete(novel: Novel) {
  try {
    await ElMessageBox.confirm(`确定删除「${novel.title}」吗？此操作不可恢复。`, '确认删除', {
      type: 'warning',
      confirmButtonText: '删除',
      cancelButtonText: '取消',
    })
    await store.remove(novel.id)
    ElMessage.success('已删除')
  } catch {
    // cancelled
  }
}

function formatDate(dateStr: string): string {
  if (!dateStr) return ''
  // SQLite datetime format: "YYYY-MM-DD HH:MM:SS"
  return dateStr.replace('T', ' ').substring(0, 19)
}
</script>

<style scoped>
.novel-list-page {
  width: 100%;
  max-width: 1440px;
  margin: 0 auto;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 16px;
}
.page-header h2 {
  font-size: 20px;
  font-weight: 600;
}
.header-actions {
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
  row-gap: 8px;
}
.fav-icon {
  cursor: pointer;
  font-size: 18px;
  color: var(--text-secondary);
  transition: color 0.2s;
}
.fav-icon.is-favorite {
  color: var(--warning-color);
}
.fav-icon:hover {
  color: var(--warning-color);
}
</style>
