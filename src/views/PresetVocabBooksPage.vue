<template>
  <div class="preset-page">
    <PageHeader :title="t('preset.title')" :description="t('preset.subtitle')" />
    <div class="page-toolbar">
      <el-input v-model="query" class="header-search" :placeholder="t('preset.search')" :aria-label="t('preset.search')" clearable>
        <template #prefix><el-icon><Search /></el-icon></template>
      </el-input>
      <el-radio-group v-model="category" :aria-label="t('preset.category')">
        <el-radio-button value="all">{{ t('preset.category.all') }}</el-radio-button>
        <el-radio-button v-for="item in categories" :key="item" :value="item">{{ t(`preset.category.${item}`) }}</el-radio-button>
      </el-radio-group>
    </div>
    <p v-if="!loading && !error" class="catalog-count">{{ t('preset.catalogCount', { n: filteredPresets.length }) }}</p>
    <PageState v-if="error" :title="t('ui.loadFailed')" :description="error" error @retry="load" />

    <div v-if="loading" class="state-row">
      <el-icon class="is-loading"><Loading /></el-icon>
    </div>

    <div v-else-if="!error && filteredPresets.length === 0" class="state-row">{{ t('preset.noResults') }}</div>

    <div v-else-if="!error" class="preset-grid">
      <div v-for="p in filteredPresets" :key="p.id" class="preset-card">
        <div class="card-top">
          <div class="card-icon"><Collection /></div>
          <el-tag size="small" type="warning">{{ t(`preset.category.${p.category || 'university'}`) }}</el-tag>
        </div>
        <div class="card-name">{{ presetName(p) }}</div>
        <div class="card-desc">{{ presetDescription(p) || '—' }}</div>
        <div class="card-meta">{{ t('vocabDetail.wordCount', { n: p.wordCount }) }}</div>
        <details v-if="p.sources?.length" class="source-details">
          <summary>{{ t('preset.sourceVersions', { n: p.sources.length }) }}</summary>
          <ul><li v-for="source in p.sources" :key="source">{{ presetSourceName(source) }}</li></ul>
        </details>
        <div class="card-actions">
          <el-button type="primary" size="small" @click="openImport(p)">{{ t('preset.fullImport') }}</el-button>
          <el-button size="small" @click="openStudy(p)"><el-icon><Reading /></el-icon> {{ t('preset.novelSelection') }}</el-button>
        </div>
      </div>
    </div>

    <PresetCloneDialog
      v-model="studyVisible"
      :preset-key="activePreset?.presetKey || ''"
      :preset-name="activePreset ? presetName(activePreset) : ''"
      @imported="onImported"
    />
    <PresetImportDialog v-model="importVisible" :preset-key="activePreset?.presetKey || ''" :preset-name="activePreset ? presetName(activePreset) : ''" @imported="onImported" />
  </div>
</template>

<script setup lang="ts">
import PageState from '@/components/common/PageState.vue'
import PageHeader from '@/components/common/PageHeader.vue'
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { Collection, Loading, Reading, Search } from '@element-plus/icons-vue'
import { t } from '@/i18n'
import type { PresetVocabBook } from '@/types/vocabBook'
import PresetCloneDialog from '@/components/vocabulary/PresetCloneDialog.vue'
import PresetImportDialog from '@/components/vocabulary/PresetImportDialog.vue'
import { presetName, presetDescription, presetSourceName } from '@/utils/presetText'

const router = useRouter()
const presets = ref<PresetVocabBook[]>([])
const loading = ref(true)
const error = ref('')
const studyVisible = ref(false)
const activePreset = ref<PresetVocabBook | null>(null)
const importVisible = ref(false)
const query = ref('')
const category = ref('all')
const categories = ['university', 'international', 'school', 'textbook'] as const
let disposed = false
const filteredPresets = computed(() => {
  const search = query.value.trim().toLocaleLowerCase()
  return presets.value.filter(p => (category.value === 'all' || p.category === category.value)
    && (!search || [p.name, p.description, p.presetKey, presetName(p), presetDescription(p), ...(p.sources || []).flatMap(source => [source, presetSourceName(source)])].join(' ').toLocaleLowerCase().includes(search)))
})

async function load() {
  loading.value = true; error.value = ''
  try {
    const result = await invoke<PresetVocabBook[]>('list_preset_vocab_books')
    if (!disposed) presets.value = result
  } catch (e: any) {
    error.value = e instanceof Error ? e.message : String(e)
    if (!disposed) ElMessage.error(String(e?.message || e || t('ui.loadFailed')))
  } finally {
    loading.value = false
  }
}
onMounted(load)
onBeforeUnmount(() => { disposed = true })

function openImport(p: PresetVocabBook) { activePreset.value = p; importVisible.value = true }

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
  min-width: 0;
  min-height: 100%;
  padding: clamp(8px, 1.5vw, 24px);
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
  grid-template-columns: repeat(auto-fill, minmax(min(240px, 100%), 1fr));
  gap: clamp(10px, 1.5vw, 18px);
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
.catalog-count { color: var(--text-secondary); font-size: 13px; margin: 0 0 16px; }
.source-details { color: var(--text-secondary); font-size: 12px; line-height: 1.6; }
.source-details summary { cursor: pointer; width: fit-content; }
.source-details ul { padding-left: 20px; margin: 6px 0; }
.card-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: auto; padding-top: 8px; }

@media (max-width: 560px) {
  .preset-page {
    padding: 4px;
  }
  .subtitle {
    margin-bottom: 14px;
  }
  .preset-card {
    padding: 14px;
  }
  .preset-card .el-button {
    width: 100%;
  }
  .card-actions { flex-direction: column; }
  .page-toolbar .header-search { width: 100%; }
  .page-toolbar :deep(.el-radio-group) { display: flex; flex-wrap: wrap; gap: 4px; }
}
</style>
