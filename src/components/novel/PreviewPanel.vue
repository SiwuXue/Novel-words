<template>
  <div class="preview-panel">
    <div class="panel-header">
      <h4>实时预览</h4>
    </div>
    <div class="preview-content" v-html="displayHtml" ref="contentRef"></div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { looksLikeHtml } from '@/utils/editorHtml'

const props = defineProps<{
  html: string
}>()

const contentRef = ref<HTMLElement | null>(null)

/**
 * For HTML input (Tiptap output after any edit, or wrapped paragraphs after
 * the first load) the content is already safe — no sanitization needed.
 * For raw plain text (e.g. a freshly imported novel body that arrives as
 * a single string with newlines), wrap in <p> blocks. Sanitization only
 * runs on the rare plain-text path, so the regex cost is paid once per
 * novel, not once per keystroke.
 */
const displayHtml = computed(() => {
  if (!props.html) return '<p style="color:#909399">暂无内容</p>'
  if (looksLikeHtml(props.html)) return props.html
  return sanitizeAndWrap(props.html)
})

function sanitizeAndWrap(raw: string): string {
  return raw
    .replace(/<script[\s\S]*?<\/script>/gi, '')
    .replace(/\son\w+\s*=\s*"[^"]*"/gi, '')
    .replace(/\son\w+\s*=\s*'[^']*'/gi, '')
    .split(/\n{2,}/)
    .map((p) => '<p>' + p.replace(/\n/g, '<br>') + '</p>')
    .join('')
}

/**
 * Scroll the preview to the first text node containing `keyword`.
 * Walks text nodes via TreeWalker, then scrolls the nearest block ancestor
 * (p / h1-h6 / div) into view.
 */
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
        // Walk up to the nearest block-level element
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

.panel-header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
}

.panel-header h4 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.preview-content {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
  font-size: 15px;
  line-height: 1.8;
  color: var(--text-regular, #303133);
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
</style>