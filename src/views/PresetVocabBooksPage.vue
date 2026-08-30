<template>
  <div class="preset-page">
    <div class="page-header">
      <h2>{{ t('preset.title') }}</h2>
      <p class="subtitle">{{ t('preset.subtitle') }}</p>
    </div>

    <div v-if="loading" class="state-row">
      <el-icon class="is-loading"><Loading /></el-icon>
    </div>

    <div v-else-if="presets.length === 0" class="state-row">{{ t('vocabList.empty') }}</div>

    <div v-else class="preset-grid">
      <div v-for="p in presets" :key="p.id" class="preset-card">
        <div class="card-top">
          <div class="card-icon"><Collection /></div>
          <el-tag size="small" type="warning">{{ t('preset.preset') }}</el-tag>
        </div>
        <div class="card-name">{{ p.name }}</div>
        <div class="card-desc">{{ p.description || '—' }}</div>
        <div class="card-meta">{{ p.wordCount }} 词</div>
        <el-button type="primary" size="small" @click="openStudy(p)">
          <el-icon><Reading /></el-icon> {{ t('preset.study') }}
        </el-button>
      </div>
    </div>

    <PresetCloneDialog
      v-model="studyVisible"
      :preset-key="activePreset?.presetKey || ''"
      :preset-name="activePreset?.name || ''"
      @imported="onImported"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { Collection, Loading, Reading } from '@element-plus/icons-vue'
import { t } from '@/i18n'
import type { PresetVocabBook } from '@/types/vocabBook'
import PresetCloneDialog from '@/components/vocabulary/PresetCloneDialog.vue'

const router = useRouter()
const presets = ref<PresetVocabBook[]>([])
const loading = ref(true)
const studyVisible = ref(false)
const activePreset = ref<PresetVocabBook | null>(null)

onMounted(async () => {
  try {
    presets.value = await invoke<PresetVocabBook[]>('list_preset_vocab_books')
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '加载预设词表失败'))
  } finally {
    loading.value = false
  }
})

function openStudy(p: PresetVocabBook) {
  activePreset.value = p
  studyVisible.value = true
}

function onImported(bookId: number) {
  router.push(`/vocabulary/${bookId}`)
}
</script>

<style scoped>
.preset-page {
  width: 100%;
  max-width: 1120px;
  margin: 0 auto;
  padding: 24px;
}
.page-header h2 {
  margin: 0 0 6px;
  font-size: 20px;
}
.subtitle {
  margin: 0 0 20px;
  font-size: 13px;
  color: var(--text-secondary, #909399);
}
.state-row {
  display: flex;
  justify-content: center;
  padding: 60px 0;
  color: var(--text-secondary, #909399);
}
.preset-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 16px;
}
.preset-card {
  background: var(--bg-secondary, #f5f7fa);
  border-radius: 12px;
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  transition: transform 0.15s, box-shadow 0.15s;
}
.preset-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
}
.card-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.card-icon {
  color: var(--accent-color, #409eff);
  font-size: 22px;
}
.card-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-regular, #303133);
}
.card-desc {
  font-size: 12px;
  color: var(--text-secondary, #909399);
  min-height: 32px;
}
.card-meta {
  font-size: 12px;
  color: var(--text-placeholder, #c0c4cc);
}
.preset-card .el-button {
  align-self: flex-start;
  margin-top: 4px;
}
</style>
