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
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Plus, Star, StarFilled } from '@element-plus/icons-vue'
import { useNovelStore } from '@/stores/novelStore'
import type { Novel } from '@/types/novel'
import NovelFormDialog from '@/components/novel/NovelFormDialog.vue'

const router = useRouter()
const store = useNovelStore()

const searchQuery = ref('')
const dialogVisible = ref(false)
const editingNovel = ref<Novel | null>(null)

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

function editNovel(novel: Novel) {
  editingNovel.value = novel
  dialogVisible.value = true
}

async function handleSubmit(data: { title: string; author: string; category: string; rawText: string }) {
  if (editingNovel.value) {
    await store.update(editingNovel.value.id, data)
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
  max-width: 1100px;
  margin: 0 auto;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
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
