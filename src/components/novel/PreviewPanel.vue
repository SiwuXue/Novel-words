<template>
  <div class="preview-panel" :class="{ fullscreen, 'reading-mode': readingMode }" :style="readingStyle">
    <div class="panel-header">
      <h4>{{ readingMode ? (chapterTitle || t('reading.preview')) : '实时预览' }}</h4>
      <div class="panel-actions">
        <template v-if="!readingMode">
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
        <button
          class="fullscreen-btn reading-btn"
          :title="t('reading.enter')"
          :aria-label="t('reading.enter')"
          @click="emit('enter-reading-mode')"
        >
          <el-icon><Reading /></el-icon>
        </button>
        </template>
        <template v-else>
          <span class="reading-meta">
            {{ Math.round(readingOverallProgress * 100) }}% · {{ readingRemainingLabel }}
          </span>
          <el-popover
            v-model:visible="readingSettingsOpen"
            placement="bottom-end"
            :width="320"
            trigger="click"
            popper-class="reading-settings-popper"
          >
            <template #reference>
              <button class="fullscreen-btn" :title="t('reading.settings')" :aria-label="t('reading.settings')">
                <el-icon><Setting /></el-icon>
              </button>
            </template>
            <div class="reading-settings-panel">
              <div class="reading-settings-title">{{ t('reading.settings') }}</div>
              <label class="reading-setting-row">
                <span>{{ t('reading.font') }}</span>
                <el-select v-model="readingFont" size="small" style="width: 180px">
                  <el-option value="system" :label="t('reading.fontSystem')" />
                  <el-option value="serif" :label="t('reading.fontSerif')" />
                  <el-option value="mono" :label="t('reading.fontMono')" />
                </el-select>
              </label>
              <label class="reading-setting-row">
                <span>{{ t('reading.fontSize') }} {{ readingFontSize }}px</span>
                <el-slider v-model="readingFontSize" :min="15" :max="30" :step="1" style="width: 150px" />
              </label>
              <label class="reading-setting-row">
                <span>{{ t('reading.lineHeight') }} {{ readingLineHeight.toFixed(1) }}</span>
                <el-slider v-model="readingLineHeight" :min="1.4" :max="2.4" :step="0.1" style="width: 150px" />
              </label>
              <label class="reading-setting-row">
                <span>{{ t('reading.width') }}</span>
                <el-radio-group v-model="readingWidth" size="small">
                  <el-radio-button :value="620">{{ t('reading.widthNarrow') }}</el-radio-button>
                  <el-radio-button :value="760">{{ t('reading.widthMedium') }}</el-radio-button>
                  <el-radio-button :value="920">{{ t('reading.widthWide') }}</el-radio-button>
                </el-radio-group>
              </label>
            </div>
          </el-popover>
          <button class="fullscreen-btn" :title="t('reading.exit')" :aria-label="t('reading.exit')" @click="emit('exit-reading-mode')">
            <el-icon><Close /></el-icon>
          </button>
        </template>
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
    <div v-if="readingMode" class="preview-reading-bottom" role="navigation" :aria-label="t('reading.navigation')">
      <button class="chapter-nav-btn" :disabled="!hasPreviousChapter" @click="emit('previous-chapter')">
        <el-icon><ArrowLeft /></el-icon> {{ t('reading.previousChapter') }}
      </button>
      <div class="preview-reading-progress" :title="t('reading.shortcutHint')">
        <el-progress :percentage="readingPercent" :stroke-width="5" :show-text="false" />
        <span>{{ t('reading.chapterProgress', { n: readingPercent }) }}</span>
      </div>
      <button class="chapter-nav-btn" :disabled="!hasNextChapter" @click="emit('next-chapter')">
        {{ t('reading.nextChapter') }} <el-icon><ArrowRight /></el-icon>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { ArrowLeft, ArrowRight, Aim, Close, FullScreen, Reading, Setting } from '@element-plus/icons-vue'
import { looksLikeHtml } from '@/utils/editorHtml'
import { speakWord } from '@/utils/speech'
import { useSettingsStore } from '@/stores/settingsStore'
import { t } from '@/i18n'

const settingsStore = useSettingsStore()
const contentRef = ref<HTMLElement | null>(null)

const props = defineProps<{
  html: string
  htmlChunks?: string[]
  previewScope?: 'current' | 'all'
  loadedChapters?: number
  totalChapters?: number
  fullscreen?: boolean
  readingMode?: boolean
  chapterTitle?: string
  chapterIndex?: number
  readingOverallProgress?: number
  readingRemainingMinutes?: number
}>()

const emit = defineEmits<{
  (e: 'toggle-fullscreen'): void
  (e: 'update:previewScope', scope: 'current' | 'all'): void
  (e: 'load-more'): void
  (e: 'enter-reading-mode'): void
  (e: 'exit-reading-mode'): void
  (e: 'previous-chapter'): void
  (e: 'next-chapter'): void
  (e: 'reading-progress', percent: number): void
}>()

type ReadingFont = 'system' | 'serif' | 'mono'
const readingSettingsOpen = ref(false)
const readingFont = ref<ReadingFont>('system')
const readingFontSize = ref(18)
const readingLineHeight = ref(1.9)
const readingWidth = ref(760)

const readingStyle = computed(() => ({
  '--reading-font-family':
    readingFont.value === 'serif'
      ? 'Georgia, "Songti SC", "STSong", serif'
      : readingFont.value === 'mono'
        ? 'ui-monospace, SFMono-Regular, Consolas, monospace'
        : 'system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  '--reading-font-size': `${readingFontSize.value}px`,
  '--reading-line-height': String(readingLineHeight.value),
  '--reading-width': `${readingWidth.value}px`,
}))

const readingProgress = ref(0)
const readingPercent = computed(() => Math.round(readingProgress.value * 100))
const readingOverallProgress = computed(() => Math.min(1, Math.max(0, props.readingOverallProgress ?? readingProgress.value)))
const readingRemainingLabel = computed(() => {
  const n = props.readingRemainingMinutes ?? 0
  return n > 0 ? t('reading.remaining', { n }) : t('reading.completed')
})
const hasPreviousChapter = computed(() => (props.chapterIndex ?? 0) > 0)
const hasNextChapter = computed(() => (props.chapterIndex ?? 0) < (props.totalChapters ?? 1) - 1)

function loadReadingPreferences() {
  try {
    const raw = localStorage.getItem('reading-preferences')
    if (!raw) return
    const saved = JSON.parse(raw) as Partial<{ font: ReadingFont; fontSize: number; lineHeight: number; width: number }>
    if (saved.font === 'system' || saved.font === 'serif' || saved.font === 'mono') readingFont.value = saved.font
    if (typeof saved.fontSize === 'number') readingFontSize.value = Math.min(30, Math.max(15, saved.fontSize))
    if (typeof saved.lineHeight === 'number') readingLineHeight.value = Math.min(2.4, Math.max(1.4, saved.lineHeight))
    if (saved.width === 620 || saved.width === 760 || saved.width === 920) readingWidth.value = saved.width
  } catch {
    /* ignore malformed local preferences */
  }
}

watch(
  [readingFont, readingFontSize, readingLineHeight, readingWidth],
  () => {
    localStorage.setItem('reading-preferences', JSON.stringify({
      font: readingFont.value,
      fontSize: readingFontSize.value,
      lineHeight: readingLineHeight.value,
      width: readingWidth.value,
    }))
  },
)

onMounted(() => {
  loadReadingPreferences()
  window.addEventListener('keydown', onKeyDown)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeyDown)
})

watch(
  () => [props.readingMode, props.html, props.chapterIndex] as const,
  async ([active]) => {
    if (!active) return
    await nextTick()
    if (contentRef.value) contentRef.value.scrollTop = 0
    readingProgress.value = 0
    emit('reading-progress', 0)
  },
)

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
  if (props.readingMode && contentRef.value) {
    const max = contentRef.value.scrollHeight - contentRef.value.clientHeight
    readingProgress.value = max > 0 ? contentRef.value.scrollTop / max : 0
    emit('reading-progress', readingProgress.value)
  }
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

function scrollBy(direction: 1 | -1) {
  const el = contentRef.value
  if (!el) return
  el.scrollBy({ top: direction * Math.max(160, el.clientHeight * 0.82), behavior: 'smooth' })
}

function onKeyDown(e: KeyboardEvent) {
  if (!props.readingMode) return
  const target = e.target as HTMLElement | null
  if (target?.closest('input, textarea, select, [contenteditable="true"]')) return
  if (e.altKey && e.key === 'ArrowLeft') {
    e.preventDefault()
    emit('previous-chapter')
    return
  }
  if (e.altKey && e.key === 'ArrowRight') {
    e.preventDefault()
    emit('next-chapter')
    return
  }
  if (e.key === ' ' || e.key === 'PageDown' || e.key === 'ArrowDown' || e.key === 'ArrowRight') {
    e.preventDefault()
    scrollBy(1)
  } else if (e.key === 'PageUp' || e.key === 'ArrowUp' || e.key === 'ArrowLeft') {
    e.preventDefault()
    scrollBy(-1)
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

.preview-panel.reading-mode {
  position: fixed;
  inset: 0;
  z-index: 10000;
  border: none;
  background: var(--bg-primary, #fff);
  box-shadow: 0 0 40px rgba(0, 0, 0, 0.2);
}
.preview-panel.reading-mode .panel-header {
  padding: 10px clamp(14px, 4vw, 48px);
  background: var(--bg-primary, #fff);
}
:global(.reading-settings-popper) {
  z-index: 10001 !important;
}
.preview-panel.reading-mode .preview-content {
  padding: clamp(28px, 5vw, 72px) 18px 110px;
  font-family: var(--reading-font-family);
  font-size: var(--reading-font-size);
  line-height: var(--reading-line-height);
  color: var(--text-primary, #1f2937);
}
.preview-panel.reading-mode :deep(.preview-content > *) {
  max-width: var(--reading-width, 760px);
  margin-right: auto;
  margin-left: auto;
}
.preview-panel.reading-mode :deep(.preview-content p) {
  margin-bottom: 1.1em;
  text-indent: 2em;
}
.preview-panel.reading-mode :deep(.preview-content h1),
.preview-panel.reading-mode :deep(.preview-content h2),
.preview-panel.reading-mode :deep(.preview-content h3) {
  text-indent: 0;
  margin-top: 2em;
  margin-bottom: 1em;
}
.preview-panel.reading-mode :deep(.preview-content h1) {
  font-size: calc(var(--reading-font-size) * 1.65);
}
.preview-panel.reading-mode :deep(.preview-content h2) {
  font-size: calc(var(--reading-font-size) * 1.4);
}
.preview-panel.reading-mode :deep(.preview-content h3) {
  font-size: calc(var(--reading-font-size) * 1.2);
}
.reading-meta {
  color: var(--text-secondary, #909399);
  font-size: 12px;
}
.reading-settings-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.reading-settings-title {
  color: var(--text-primary, #303133);
  font-size: 14px;
  font-weight: 600;
}
.reading-setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  color: var(--text-secondary, #606266);
  font-size: 12px;
}
.preview-reading-bottom {
  position: absolute;
  right: 0;
  bottom: 16px;
  left: 0;
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: clamp(8px, 2vw, 24px);
  pointer-events: none;
}
.preview-reading-bottom > * {
  pointer-events: auto;
}
.chapter-nav-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 8px 12px;
  border: 1px solid var(--border-color, #ebeef5);
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-primary, #fff) 92%, transparent);
  color: var(--text-regular, #303133);
  cursor: pointer;
  backdrop-filter: blur(8px);
}
.chapter-nav-btn:hover:not(:disabled) {
  border-color: var(--accent-color, #409eff);
  color: var(--accent-color, #409eff);
}
.chapter-nav-btn:disabled {
  cursor: not-allowed;
  opacity: 0.45;
}
.preview-reading-progress {
  width: min(36vw, 420px);
  padding: 8px 14px;
  border: 1px solid var(--border-color, #ebeef5);
  border-radius: 999px;
  background: color-mix(in srgb, var(--bg-primary, #fff) 92%, transparent);
  box-shadow: 0 6px 20px rgba(15, 23, 42, 0.08);
  backdrop-filter: blur(8px);
}
.preview-reading-progress span {
  display: block;
  margin-top: 4px;
  color: var(--text-secondary, #909399);
  font-size: 11px;
  text-align: center;
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
