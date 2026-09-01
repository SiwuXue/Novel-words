<template>
  <div class="preview-panel" :class="{ fullscreen }">
    <div class="panel-header">
      <h4>实时预览</h4>
      <div class="panel-actions">
        <el-radio-group
          :model-value="previewScope"
          size="small"
          @change="$emit('update:previewScope', $event as 'current' | 'all')"
        >
          <el-radio-button value="current">当前章</el-radio-button>
          <el-radio-button value="all">全部</el-radio-button>
        </el-radio-group>
        <span v-if="previewScope === 'all' && totalChapters" class="chapter-progress">
          {{ loadedChapters }}/{{ totalChapters }}章
        </span>
        <button
          class="fullscreen-btn"
          :title="fullscreen ? '退出全屏' : '全屏预览'"
          @click="$emit('toggle-fullscreen')"
        >
          <el-icon>
            <FullScreen v-if="!fullscreen" />
            <Aim v-else />
          </el-icon>
        </button>
      </div>
    </div>
    <div class="preview-content" ref="contentRef" @click="onContentClick" @scroll="onScroll">
      <template v-if="previewScope === 'all'">
        <div
          v-for="(chunk, index) in displayChunks"
          :key="index"
          class="preview-chapter-chunk"
          v-html="chunk"
        ></div>
        <button
          v-if="(loadedChapters || 0) < (totalChapters || 0)"
          class="load-more-btn"
          @click="$emit('load-more')"
        >
          继续加载后续章节
        </button>
        <p v-else-if="totalChapters" class="all-loaded">已加载全部 {{ totalChapters }} 章</p>
      </template>
      <div v-else v-html="displayHtml"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { FullScreen, Aim } from '@element-plus/icons-vue'
import { looksLikeHtml } from '@/utils/editorHtml'
import { speakWord } from '@/utils/speech'
import { useSettingsStore } from '@/stores/settingsStore'

const settingsStore = useSettingsStore()
const contentRef = ref<HTMLElement | null>(null)

const props = defineProps<{
  html: string
  htmlChunks?: string[]
  previewScope?: 'current' | 'all'
  loadedChapters?: number
  totalChapters?: number
  fullscreen?: boolean
}>()

const emit = defineEmits<{
  (e: 'toggle-fullscreen'): void
  (e: 'update:previewScope', scope: 'current' | 'all'): void
  (e: 'load-more'): void
}>()

const displayHtml = computed(() => {
  if (!props.html) return '<p style="color:#909399">暂无内容</p>'
  if (looksLikeHtml(props.html)) return props.html
  return sanitizeAndWrap(props.html)
})

const displayChunks = computed(() =>
  (props.htmlChunks || []).map((html) =>
    looksLikeHtml(html) ? html : sanitizeAndWrap(html),
  ),
)

let loadMorePending = false
function onScroll() {
  if (props.previewScope !== 'all' || loadMorePending || !contentRef.value) return
  if ((props.loadedChapters || 0) >= (props.totalChapters || 0)) return
  const el = contentRef.value
  if (el.scrollHeight - el.scrollTop - el.clientHeight < 800) {
    loadMorePending = true
    emit('load-more')
    requestAnimationFrame(() => {
      loadMorePending = false
    })
  }
}

function sanitizeAndWrap(raw: string): string {
  return raw
    .replace(/<script[\s\S]*?<\/script>/gi, '')
    .replace(/\son\w+\s*=\s*"[^"]*"/gi, '')
    .replace(/\son\w+\s*=\s*'[^']*'/gi, '')
    .split(/\n{2,}/)
    .map((p) => '<p>' + p.replace(/\n/g, '<br>') + '</p>')
    .join('')
}

/** 点击预览中的英文单词（.vocab-en）朗读该单词。 */
function onContentClick(e: MouseEvent) {
  const target = e.target as HTMLElement
  const span = target.closest('.vocab-en') as HTMLElement | null
  if (!span) return
  const word = span.textContent?.trim()
  if (!word) return
  speakWord(word, settingsStore.speechAccent)
}

function scrollToText(keyword: string): boolean {
  if (!contentRef.value || !keyword) return false
  try {
    const walker = document.createTreeWalker(
      contentRef.value,
      NodeFilter.SHOW_TEXT,
      null,
    )
    let node: Node | null
    while ((node = walker.nextNode())) {
      const text = node.textContent || ''
      if (text.includes(keyword)) {
        let el: HTMLElement | null = node.parentElement as HTMLElement | null
        while (el && el !== contentRef.value) {
          const tag = el.tagName.toLowerCase()
          if (tag === 'p' || /^h[1-6]$/.test(tag) || tag === 'div') break
          el = el.parentElement
        }
        const target = el || (node.parentElement as HTMLElement | null)
        if (target) {
          target.scrollIntoView({ behavior: 'smooth', block: 'start' })
          return true
        }
      }
    }
  } catch (e) {
    console.warn('[PreviewPanel] scrollToText failed:', e)
  }
  return false
}

defineExpose({ scrollToText })
</script>

<style scoped>
.preview-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--border-color, #ebeef5);
  background: var(--bg-color, #fff);
}

.preview-panel.fullscreen {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 9999;
  border-left: none;
  box-shadow: 0 0 40px rgba(0, 0, 0, 0.2);
}

.panel-header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.panel-header h4 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.panel-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.chapter-progress {
  color: var(--text-secondary, #909399);
  font-size: 12px;
  white-space: nowrap;
}

.fullscreen-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary, #909399);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.fullscreen-btn:hover {
  background: var(--accent-light, #ecf5ff);
  color: var(--accent-color, #409eff);
}

.preview-content {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
  font-size: 15px;
  line-height: 1.8;
  color: var(--text-regular, #303133);
}

.preview-chapter-chunk + .preview-chapter-chunk {
  border-top: 1px dashed var(--border-color, #dcdfe6);
  margin-top: 18px;
  padding-top: 8px;
}

.load-more-btn {
  display: block;
  margin: 20px auto;
  padding: 8px 18px;
  border: 1px solid var(--accent-color, #409eff);
  border-radius: 6px;
  background: transparent;
  color: var(--accent-color, #409eff);
  cursor: pointer;
}

.all-loaded {
  color: var(--text-secondary, #909399);
  text-align: center;
  font-size: 12px;
  margin: 20px 0;
}

.preview-panel.fullscreen .preview-content {
  padding: 32px 15%;
}

:deep(.preview-content h1) {
  font-size: 22px;
  font-weight: 700;
  margin: 16px 0 8px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
}
:deep(.preview-content h2) { font-size: 19px; font-weight: 600; margin: 14px 0 6px; }
:deep(.preview-content h3) { font-size: 17px; font-weight: 600; margin: 12px 0 4px; }
:deep(.preview-content p) { margin: 0 0 8px; }
:deep(.preview-content ul), :deep(.preview-content ol) { padding-left: 24px; margin: 4px 0 8px; }

:deep(.preview-content blockquote) {
  border-left: 3px solid var(--accent-color, #409eff);
  padding-left: 12px;
  margin: 8px 0;
  color: var(--text-secondary);
}

/* PDF preview: highlighted vocab words and their inline sup annotations. */
:deep(.preview-content .vocab-word) {
  display: inline-block;
  line-height: 1.4;
}
:deep(.preview-content .vocab-en) {
  cursor: pointer;
}
:deep(.preview-content .vocab-en:hover) {
  text-decoration: underline;
}
:deep(.preview-content .vocab-word sup) {
  margin-left: 2px;
  opacity: 0.85;
}
:deep(.preview-content .pdf-preview-body h1.title) {
  font-size: 26px;
  text-align: center;
  margin: 24px 0 12px;
  font-weight: 700;
}
:deep(.preview-content .pdf-preview-body h2.chapter) {
  font-size: 20px;
  margin: 20px 0 8px;
  padding-bottom: 4px;
  border-bottom: 1px solid #e0e0e0;
  font-weight: 600;
}
:deep(.preview-content .pdf-preview-body .vocab-heading) {
  font-size: 16px;
  margin: 18px 0 6px;
  font-weight: 600;
  color: #555;
}
:deep(.preview-content .pdf-preview-body .vocab-table) {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
  margin: 0 0 16px;
}
:deep(.preview-content .pdf-preview-body .vocab-table th),
:deep(.preview-content .pdf-preview-body .vocab-table td) {
  padding: 4px 8px;
  border: 1px solid #ddd;
  text-align: left;
  vertical-align: top;
}
:deep(.preview-content .pdf-preview-body .vocab-table th) {
  background: #f5f5f5;
}

@media (max-width: 900px) {
  .preview-panel {
    border-left: none;
  }
  .panel-header {
    padding: 8px 12px;
  }
  .preview-content {
    padding: 14px 12px;
  }
  .preview-panel.fullscreen .preview-content {
    padding: 24px clamp(14px, 6vw, 72px);
  }
  :deep(.preview-content .pdf-preview-body .vocab-table) {
    display: block;
    overflow-x: auto;
  }
}
</style>
