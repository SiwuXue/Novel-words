<template>
  <section class="all-vocabulary-panel" :aria-label="t('allVocab.title')">
    <p class="panel-hint">{{ t('allVocab.hint') }}</p>
    <div class="page-toolbar">
      <el-input v-model="query" class="header-search" clearable :placeholder="t('allVocab.search')" :aria-label="t('allVocab.search')">
        <template #prefix><el-icon><Search /></el-icon></template>
      </el-input>
      <el-checkbox-group v-model="filters" :aria-label="t('vocabDetail.proficiency')">
        <el-checkbox-button v-for="value in proficiencyValues" :key="value" :value="value">{{ proficiencyLabel(value) }}</el-checkbox-button>
      </el-checkbox-group>
    </div>
    <div class="batch-toolbar" :aria-label="t('allVocab.batchStatus')">
      <span>{{ t('allVocab.selected', { n: selected.length }) }}</span>
      <el-button v-for="value in proficiencyValues" :key="value" size="small" :disabled="!selected.length || busy" @click="setStatus(selected.map(word => word.id), value)">
        {{ t('allVocab.markAs', { status: proficiencyLabel(value) }) }}
      </el-button>
    </div>
    <PageState v-if="error" :title="t('ui.loadFailed')" :description="error" error @retry="load" />
    <el-table ref="table" v-loading="loading" :data="words" row-key="id" stripe :empty-text="t('allVocab.empty')" @selection-change="selected = $event">
      <el-table-column type="selection" width="48" fixed="left" />
      <el-table-column prop="word" :label="t('vocabDetail.word')" width="110" fixed="left" show-overflow-tooltip />
      <el-table-column :label="t('vocabDetail.proficiency')" width="140" fixed="left">
        <template #default="{ row }">
          <el-select :model-value="row.proficiency" :disabled="busy" :aria-label="t('allVocab.wordStatus', { word: row.word })" @change="setStatus([row.id], $event)">
            <el-option v-for="value in proficiencyValues" :key="value" :value="value" :label="proficiencyLabel(value)" />
          </el-select>
        </template>
      </el-table-column>
      <el-table-column prop="definition" :label="t('vocabDetail.definition')" min-width="180" show-overflow-tooltip />
      <el-table-column prop="phonetic" :label="t('vocabDetail.phonetic')" min-width="120" />
      <el-table-column :label="t('allVocab.sources')" min-width="170">
        <template #default="{ row }">
          <div class="source-books">
            <RouterLink v-for="book in row.sourceBooks" :key="book.id" class="library-title" :to="`/vocabulary/${book.id}`">{{ book.name }}</RouterLink>
            <el-tag v-if="!row.active || !row.sourceBooks.length" type="info" size="small">{{ t('allVocab.paused') }}</el-tag>
          </div>
        </template>
      </el-table-column>
    </el-table>
    <el-pagination v-model:current-page="page" v-model:page-size="pageSize" :total="total" :page-sizes="[20, 50, 100, 200]" layout="total, sizes, prev, pager, next" background class="pagination" @current-change="load" @size-change="resize" />
  </section>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { Search } from '@element-plus/icons-vue'
import PageState from '@/components/common/PageState.vue'
import { t } from '@/i18n'
import type { Proficiency, UserVocabEntry, UserVocabPage } from '@/types/vocabWord'

const proficiencyValues: Proficiency[] = ['unknown', 'familiar', 'mastered']
const query = ref('')
const filters = ref<Proficiency[]>([...proficiencyValues])
const words = ref<UserVocabEntry[]>([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(50)
const loading = ref(false)
const busy = ref(false)
const error = ref('')
const selected = ref<UserVocabEntry[]>([])
const table = ref<{ clearSelection(): void } | null>(null)
let generation = 0
let disposed = false
let searchTimer: ReturnType<typeof setTimeout> | null = null

function proficiencyLabel(value: Proficiency) { return t(`vocabDetail.${value}`) }

async function load() {
  const request = ++generation
  selected.value = []
  table.value?.clearSelection()
  loading.value = true
  error.value = ''
  try {
    // An empty selection means no categories; null means every category.
    if (!filters.value.length) { words.value = []; total.value = 0; return }
    const result = await invoke<UserVocabPage>('get_user_vocab_page', {
      query: query.value.trim() || null,
      proficiencies: filters.value.length === proficiencyValues.length ? null : [...filters.value],
      offset: (page.value - 1) * pageSize.value,
      limit: pageSize.value,
    })
    if (disposed || request !== generation) return
    words.value = result.words
    total.value = result.total
    const maxPage = Math.max(1, Math.ceil(result.total / pageSize.value))
    if (page.value > maxPage) { page.value = maxPage; await load() }
  } catch (e) {
    if (!disposed && request === generation) error.value = e instanceof Error ? e.message : String(e)
  } finally {
    if (!disposed && request === generation) loading.value = false
  }
}

async function setStatus(ids: number[], proficiency: Proficiency) {
  if (!ids.length || busy.value) return
  busy.value = true
  try {
    const affected = await invoke<number>('set_user_vocab_proficiency', { ids: [...new Set(ids)], proficiency })
    if (disposed) return
    ElMessage.success(t('allVocab.statusUpdated', { n: affected }))
    await load()
  } catch (e) {
    if (!disposed) ElMessage.error(e instanceof Error ? e.message : String(e))
  } finally { if (!disposed) busy.value = false }
}

function resize() { page.value = 1; void load() }
watch(query, () => {
  ++generation // Invalidate the previous search as soon as the input changes.
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(() => { page.value = 1; void load() }, 300)
})
watch(filters, () => { page.value = 1; void load() })
onMounted(load)
onBeforeUnmount(() => { disposed = true; ++generation; if (searchTimer) clearTimeout(searchTimer) })
</script>

<style scoped>
.all-vocabulary-panel { min-width: 0; }
.panel-hint { margin: 0 0 16px; font-size: 13px; color: var(--text-secondary); line-height: 1.6; }
.batch-toolbar { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin-bottom: 16px; }
.batch-toolbar > span { font-size: 13px; color: var(--text-secondary); }
.source-books { display: flex; flex-wrap: wrap; gap: 6px 12px; }
.source-books a { overflow-wrap: anywhere; }
.pagination { margin-top: 16px; justify-content: flex-end; }
@media (max-width: 560px) { .page-toolbar .header-search { width: 100%; }.batch-toolbar { align-items: stretch; }.pagination { justify-content: flex-start; } }
</style>
