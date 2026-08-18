<template>
  <div class="editor-page">
    <div class="editor-topbar">
      <el-button link @click="goBack">
        <el-icon><ArrowLeft /></el-icon> 返回列表
      </el-button>
      <span class="novel-title">{{ topbarTitle }}</span>
      <span class="save-status">
        <el-icon v-if="editorStore.saving" class="is-loading"><Loading /></el-icon>
        <template v-else>{{ editorStore.isDirty ? '未保存' : '已保存' }}</template>
      </span>
      <el-dropdown
        v-if="loadState === 'loaded'"
        trigger="click"
        @change="onStepsDropdownClick"
      >
        <el-button size="small" link>
          ⚙ 导出步骤：已选 {{ pdfSteps.length }} 项
        </el-button>
        <template #dropdown>
          <div class="pdf-steps-dropdown">
            <div class="pdf-steps-dropdown-title">本次导出包含（不影响默认设置）</div>
            <el-checkbox-group
              v-model="pdfSteps"
              @change="onPdfStepsChange"
              style="display:flex;flex-direction:column;gap:6px;padding:6px 4px 2px;"
            >
              <el-checkbox v-for="n in stepNums" :key="n" :label="n" :value="n">
                {{ STEP_LABELS[n] }}
              </el-checkbox>
            </el-checkbox-group>
          </div>
        </template>
      </el-dropdown>
      <el-button
        v-if="loadState === 'loaded'"
        size="small"
        @click="handleExportPdf"
        :loading="exportingPdf"
      >
        <el-icon><Printer /></el-icon> 导出 PDF
      </el-button>
    </div>

    <!-- Loaded: three-column body with draggable splitters -->
    <div class="editor-body" v-if="loadState === 'loaded'">
      <div
        class="editor-pane left-pane"
        :style="{ width: split.state.leftWidth + 'px' }"
        v-show="split.state.leftWidth > 0"
      >
        <ChapterList
          :chapters="editorStore.chapterList"
          :active-index="editorStore.activeChapterIndex"
          @select="scrollToChapter"
        />
      </div>
      <div
        class="split-divider"
        :class="{ collapsed: split.state.leftWidth === 0 }"
        @mousedown="split.startLeftDrag"
        @dblclick="split.toggleLeft"
        title="拖拽调整宽度 · 双击折叠/恢复"
      >
        <el-icon v-if="split.state.leftWidth === 0" class="divider-icon">
          <component :is="DArrowRight" />
        </el-icon>
      </div>
      <div class="editor-pane center-pane">
        <NovelEditor
          ref="editorRef"
          :novel-id="currentNovelId"
          :content="editorContent"
          :highlight-words="highlightWords"
          :highlight-book-id="highlightBookId"
          @update:content="onEditorContentChange"
          @update:highlight-book-id="highlightBookId = $event"
        />
      </div>
      <div
        class="split-divider"
        :class="{ collapsed: split.state.rightWidth === 0 }"
        @mousedown="split.startRightDrag"
        @dblclick="split.toggleRight"
        title="拖拽调整宽度 · 双击折叠/恢复"
      >
        <el-icon v-if="split.state.rightWidth === 0" class="divider-icon">
          <component :is="DArrowLeft" />
        </el-icon>
      </div>
      <div
        class="editor-pane right-pane"
        :style="{ width: split.state.rightWidth + 'px' }"
        v-show="split.state.rightWidth > 0"
      >
        <PreviewPanel
          ref="previewRef"
          :html="previewHtml"
          :fullscreen="previewFullscreen"
          @toggle-fullscreen="togglePreviewFullscreen"
        />
      </div>
    </div>

    <!-- Loading -->
    <div v-else-if="loadState === 'loading'" class="editor-state-block">
      <el-icon class="is-loading" :size="32"><Loading /></el-icon>
      <span>加载中...（已等待 {{ elapsedSeconds }}s）</span>
    </div>

    <!-- Error / not-found -->
    <div v-else-if="loadState === 'error'" class="editor-state-block error">
      <el-result icon="error" title="无法加载小说" :sub-title="errorMessage">
        <template #extra>
          <el-button type="primary" @click="retry">重试</el-button>
          <el-button @click="goBack">返回列表</el-button>
        </template>
      </el-result>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRoute, useRouter, onBeforeRouteLeave } from 'vue-router'
import { ArrowLeft, Loading, Printer, DArrowLeft, DArrowRight } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { useNovelStore } from '@/stores/novelStore'
import { useEditorStore } from '@/stores/editorStore'
import { useSettingsStore } from '@/stores/settingsStore'
import type { HighlightWord } from '@/types/vocabWord'
import { normalizeSteps, STEP_LABELS, type StepNum } from '@/types/pdfSteps'
import NovelEditor from '@/components/novel/NovelEditor.vue'
import ChapterList from '@/components/novel/ChapterList.vue'
import PreviewPanel from '@/components/novel/PreviewPanel.vue'
import { buildHtml as buildPreviewHtml } from '@/utils/pdfPreview'
import { useSplitLayout } from '@/composables/useSplitLayout'

const settingsStore = useSettingsStore()

const split = useSplitLayout({
  left: 200,
  right: 280,
  min: 50,
  max: 500,
  storageKey: 'novel-editor-layout-v2',
})
const route = useRoute()
const router = useRouter()
const store = useNovelStore()
const editorStore = useEditorStore()
const editorRef = ref<InstanceType<typeof NovelEditor> | null>(null)
const previewRef = ref<InstanceType<typeof PreviewPanel> | null>(null)

const highlightBookId = ref<number | null>(null)
const highlightWords = ref<HighlightWord[]>([])
const exportingPdf = ref(false)
const previewFullscreen = ref(false)
const stepNums: StepNum[] = [1, 2, 3]
const pdfSteps = ref<StepNum[]>([...settingsStore.pdfIntensiveSteps])

// Once store is loaded (async), sync session buffer once.
watch(
  () => [settingsStore.loaded, settingsStore.pdfIntensiveSteps] as const,
  ([loaded, v]) => {
    if (loaded) pdfSteps.value = [...v]
  },
  { immediate: true, once: true },
)

function togglePreviewFullscreen() {
  previewFullscreen.value = !previewFullscreen.value
}

function onStepsDropdownClick(_cmd: any) {
  // Checkbox group is handled by v-model on pdfSteps directly.
}

function onPdfStepsChange(next: StepNum[]) {
  if (next.length === 0) {
    ElMessage.warning('至少勾选一个步骤')
    // rollback to a normalized safe value without affecting settingsStore
    pdfSteps.value = normalizeSteps(pdfSteps.value)
    return
  }
  pdfSteps.value = next
}

type LoadState = 'loading' | 'loaded' | 'error'
const loadState = ref<LoadState>('loading')
const errorMessage = ref('')
const elapsedSeconds = ref(0)

let loadStartedAt = 0
let elapsedTimer: number | null = null
let hardTimeoutTimer: number | null = null

const currentNovelId = computed(() => {
  const n = Number(route.params.id)
  return Number.isFinite(n) ? n : 0
})

const editorContentOverride = ref<string | null>(null)
const editorContent = computed<string>({
  get: () =>
    editorContentOverride.value ?? store.currentNovel?.cleanedText ?? '',
  set: (v) => {
    editorContentOverride.value = v
  },
})
function onEditorContentChange(html: string) {
  editorContentOverride.value = html
}

const topbarTitle = computed(() => {
  if (loadState.value === 'loaded') return store.currentNovel?.title || ''
  if (loadState.value === 'error') return '加载失败'
  return '加载中...'
})

const previewHtml = computed(() => {
  const content = editorContent.value || store.currentNovel?.cleanedText || ''
  if (!content) return '<p>无内容</p>'
  const chapterList = editorStore.chapterList
  const chapters =
    chapterList.length > 0
      ? chapterList
      : [{ id: 0, novelId: 0, title: '', content, sortOrder: 0, startIndex: 0, createdAt: '' }]
  return buildPreviewHtml({
    chapters,
    words: highlightWords.value as any,
    novelTitle: store.currentNovel?.title,
    steps: normalizeSteps(pdfSteps.value),
  })
})

/** Remove characters that are invalid in Windows file names. */
function sanitizeFilename(name: string): string {
  return name
    .replace(/[<>:"/\\|?*]/g, '_')
    .replace(/[\x00-\x1f]/g, '')
    .trim()
    .replace(/\.+$/, '')
    .slice(0, 200) || 'export'
}

async function handleExportPdf() {
  const novel = store.currentNovel
  if (!novel) {
    ElMessage.error('请先打开小说')
    return
  }
  let filePath: string | null = null
  try {
    filePath = await save({
      defaultPath: `${sanitizeFilename(novel.title || 'export')}.pdf`,
      filters: [{ name: 'PDF', extensions: ['pdf'] }],
    })
  } catch (e: any) {
    console.error('[PdfExport] save dialog failed:', e)
    ElMessage.error('打开保存对话框失败: ' + String(e?.message || e))
    return
  }
  if (!filePath) return
  exportingPdf.value = true
  try {
    await invoke<string>('export_pdf', {
      novelId: novel.id,
      templateType: 'intensive',
      vocabBookId: highlightBookId.value,
      steps: normalizeSteps(pdfSteps.value),
      outputPath: filePath,
    })
    ElMessage.success('PDF 已导出')
  } catch (e: any) {
    console.error('[PdfExport] export_pdf failed:', e)
    ElMessage.error(String(e?.message || e || '导出失败'))
  } finally {
    exportingPdf.value = false
  }
}

async function loadNovel() {
  loadStartedAt = Date.now()
  errorMessage.value = ''
  editorContentOverride.value = null

  const id = currentNovelId.value
  if (!id) {
    errorMessage.value = `无效的小说 ID：${route.params.id}`
    loadState.value = 'error'
    return
  }

  elapsedTimer = window.setInterval(() => {
    elapsedSeconds.value = Math.floor((Date.now() - loadStartedAt) / 1000)
  }, 1000)

  hardTimeoutTimer = window.setTimeout(() => {
    if (loadState.value === 'loading') {
      errorMessage.value = '加载超时（10s）—— 请检查网络或重启应用'
      loadState.value = 'error'
      ElMessage.error('加载超时')
    }
  }, 10000)

  try {
    editorStore.reset()
    await store.fetchOne(id)
    if (loadState.value === 'error') return
    if (!store.currentNovel) {
      errorMessage.value = '小说不存在'
      loadState.value = 'error'
      return
    }
    const text = store.currentNovel.cleanedText || store.currentNovel.rawText || ''
    editorContentOverride.value = text
    if (text) await editorStore.loadChapters(id, text)
    loadState.value = 'loaded'
    await nextTick()
  } catch (e: any) {
    if (loadState.value === 'error') return
    errorMessage.value = String(e?.message || e || '未知错误')
    loadState.value = 'error'
    ElMessage.error(errorMessage.value)
  } finally {
    cleanupTimers()
  }
}

function cleanupTimers() {
  if (elapsedTimer) {
    clearInterval(elapsedTimer)
    elapsedTimer = null
  }
  if (hardTimeoutTimer) {
    clearTimeout(hardTimeoutTimer)
    hardTimeoutTimer = null
  }
}

function retry() {
  loadState.value = 'loading'
  loadNovel()
}

onMounted(async () => {
  loadNovel()
})

watch(highlightBookId, async (bookId) => {
  if (!bookId) {
    highlightWords.value = []
    return
  }
  try {
    highlightWords.value = await invoke<HighlightWord[]>('get_highlight_words', {
      vocabBookId: bookId,
    })
  } catch (e) {
    console.error('[NovelEditorPage] get_highlight_words failed:', e)
    highlightWords.value = []
  }
})

onBeforeUnmount(() => {
  cleanupTimers()
  editorStore.reset()
})

function goBack() {
  router.push('/novels')
}

onBeforeRouteLeave(async (_to, _from, next) => {
  if (editorStore.isDirty) {
    try {
      await ElMessageBox.confirm(
        '你有未保存的修改，确定离开吗？',
        '未保存',
        { confirmButtonText: '离开', cancelButtonText: '留下', type: 'warning' },
      )
      next()
    } catch {
      next(false)
    }
  } else {
    next()
  }
})

async function scrollToChapter(index: number) {
  editorStore.activeChapterIndex = index
  const ch = editorStore.chapterList[index]
  if (!ch) return
  await new Promise<void>((r) =>
    requestAnimationFrame(() => requestAnimationFrame(() => r())),
  )
  editorRef.value?.scrollToText(ch.title)
  previewRef.value?.scrollToText(ch.title)
}
</script>

<style scoped>
.editor-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.editor-topbar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 8px 16px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
  background: var(--bg-secondary, #fafafa);
  flex-shrink: 0;
}
.novel-title {
  font-size: 15px;
  font-weight: 600;
}
.save-status {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 4px;
}
.pdf-steps-dropdown {
  min-width: 260px;
  padding: 8px 12px 10px;
}
.pdf-steps-dropdown-title {
  font-size: 12px;
  color: var(--text-secondary, #909399);
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
  margin-bottom: 4px;
}
.editor-body {
  display: flex;
  flex: 1;
  overflow: hidden;
  min-height: 0;
}
.editor-pane {
  overflow: hidden;
  flex-shrink: 0;
}
.editor-pane.left-pane,
.editor-pane.right-pane {
  min-width: 0;
}
.editor-pane.center-pane {
  flex: 1;
  min-width: 200px;
}

/* Draggable divider between panels */
.split-divider {
  width: 6px;
  flex-shrink: 0;
  cursor: col-resize;
  background: var(--border-color, #e0e0e0);
  transition: background 0.15s;
  display: flex;
  align-items: center;
  justify-content: center;
  user-select: none;
}
.split-divider:hover,
.split-divider:active {
  background: var(--accent-color, #409eff);
}
.split-divider.collapsed {
  width: 22px;
  cursor: pointer;
  background: var(--bg-secondary, #fafafa);
}
.split-divider.collapsed:hover {
  background: var(--accent-light, #ecf5ff);
}
.divider-icon {
  font-size: 14px;
  color: var(--text-secondary, #909399);
}
.editor-state-block {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--text-secondary);
  font-size: 14px;
}
.editor-state-block.error {
  color: var(--danger-color, #f56c6c);
}
</style>
