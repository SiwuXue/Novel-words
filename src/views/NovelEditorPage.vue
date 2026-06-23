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
    </div>

    <!-- Loaded: three-column body -->
    <div class="editor-body" v-if="loadState === 'loaded'">
      <ChapterList
        :chapters="editorStore.chapterList"
        :active-index="editorStore.activeChapterIndex"
        @select="scrollToChapter"
      />
      <NovelEditor
        ref="editorRef"
        :novel-id="currentNovelId"
        :content="editorContent"
        @update:content="onEditorContentChange"
      />
      <PreviewPanel ref="previewRef" :html="previewHtml" />
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
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, Loading } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { useNovelStore } from '@/stores/novelStore'
import { useEditorStore } from '@/stores/editorStore'
import NovelEditor from '@/components/novel/NovelEditor.vue'
import ChapterList from '@/components/novel/ChapterList.vue'
import PreviewPanel from '@/components/novel/PreviewPanel.vue'

const route = useRoute()
const router = useRouter()
const store = useNovelStore()
const editorStore = useEditorStore()
const editorRef = ref<InstanceType<typeof NovelEditor> | null>(null)
const previewRef = ref<InstanceType<typeof PreviewPanel> | null>(null)

type LoadState = 'loading' | 'loaded' | 'error'
const loadState = ref<LoadState>('loading')
const errorMessage = ref('')
const elapsedSeconds = ref(0)

let loadStartedAt = 0
let elapsedTimer: number | null = null
let hardTimeoutTimer: number | null = null

// currentNovelId is a pure derivation of the route — no need for a ref.
const currentNovelId = computed(() => {
  const n = Number(route.params.id)
  return Number.isFinite(n) ? n : 0
})

// editorContent mirrors store.currentNovel.cleanedText for read, but the
// NovelEditor pushes live Tiptap HTML here via @update:content so the
// preview panel stays in sync between autosaves.
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

const previewHtml = computed(
  () => editorContent.value || store.currentNovel?.cleanedText || '<p>无内容</p>',
)

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

  // Hard 10s timeout — independent of any timeout the store may have.
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
    if (text) editorStore.loadChapters(text)
    loadState.value = 'loaded'
    await nextTick()
    console.log('[NovelEditorPage] loaded, novel id =', store.currentNovel.id)
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

onMounted(() => {
  console.log('[NovelEditorPage] onMounted, route.params.id =', route.params.id)
  loadNovel()
})

onBeforeUnmount(() => {
  cleanupTimers()
  editorStore.reset()
})

function goBack() {
  router.push('/novels')
}

/**
 * Click handler for the chapter list. Waits two animation frames so any
 * in-flight setContent / sanitization has time to paint, then asks the
 * editor and preview panel to scroll. No polling — if the content isn't
 * ready the scroll call is simply a no-op.
 */
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
.editor-body {
  display: flex;
  flex: 1;
  overflow: hidden;
  min-height: 0;
}
.editor-body > :deep(.chapter-list-panel) {
  width: 200px;
  flex-shrink: 0;
}
.editor-body > :deep(.novel-editor-wrapper) {
  flex: 1;
  min-width: 0;
}
.editor-body > :deep(.preview-panel) {
  width: 280px;
  flex-shrink: 0;
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