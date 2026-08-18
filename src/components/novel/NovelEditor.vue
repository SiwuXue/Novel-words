<template>
  <div class="novel-editor-wrapper">
    <!-- Toolbar -->
    <div class="editor-toolbar" v-if="editor">
      <el-select
        :model-value="highlightBookId"
        placeholder="选择词汇本高亮"
        clearable
        size="small"
        style="width: 160px; margin-right: 8px;"
        @update:model-value="emit('update:highlightBookId', $event as number | null)"
      >
        <el-option
          v-for="b in vocabBookStore.books"
          :key="b.id"
          :label="b.name"
          :value="b.id"
        />
      </el-select>
      <el-button-group class="toolbar-group">
        <el-button size="small" :type="editor.isActive('bold') ? 'primary' : 'default'" @click="editor.chain().focus().toggleBold().run()">
          <strong>B</strong>
        </el-button>
        <el-button size="small" :type="editor.isActive('italic') ? 'primary' : 'default'" @click="editor.chain().focus().toggleItalic().run()">
          <em>I</em>
        </el-button>
        <el-button size="small" :type="editor.isActive('underline') ? 'primary' : 'default'" @click="editor.chain().focus().toggleUnderline().run()">
          <u>U</u>
        </el-button>
      </el-button-group>

      <el-button-group class="toolbar-group">
        <el-button size="small" :type="editor.isActive('heading', { level: 1 }) ? 'primary' : 'default'" @click="editor.chain().focus().toggleHeading({ level: 1 }).run()">
          H1
        </el-button>
        <el-button size="small" :type="editor.isActive('heading', { level: 2 }) ? 'primary' : 'default'" @click="editor.chain().focus().toggleHeading({ level: 2 }).run()">
          H2
        </el-button>
        <el-button size="small" :type="editor.isActive('heading', { level: 3 }) ? 'primary' : 'default'" @click="editor.chain().focus().toggleHeading({ level: 3 }).run()">
          H3
        </el-button>
      </el-button-group>

      <el-button-group class="toolbar-group">
        <el-button size="small" :type="editor.isActive('bulletList') ? 'primary' : 'default'" @click="editor.chain().focus().toggleBulletList().run()">
          <el-icon><List /></el-icon>
        </el-button>
        <el-button size="small" :type="editor.isActive('orderedList') ? 'primary' : 'default'" @click="editor.chain().focus().toggleOrderedList().run()">
          <el-icon><Tickets /></el-icon>
        </el-button>
        <el-button size="small" :type="editor.isActive('blockquote') ? 'primary' : 'default'" @click="editor.chain().focus().toggleBlockquote().run()">
❝
        </el-button>
      </el-button-group>

      <el-button-group class="toolbar-group">
        <el-button size="small" @click="editor.chain().focus().undo().run()" :disabled="!editor.can().undo()">
          <el-icon><RefreshLeft /></el-icon>
        </el-button>
        <el-button size="small" @click="editor.chain().focus().redo().run()" :disabled="!editor.can().redo()">
          <el-icon><RefreshRight /></el-icon>
        </el-button>
      </el-button-group>

      <span class="save-indicator" v-if="store.saving">保存中...</span>
      <span class="save-indicator saved" v-else-if="!store.isDirty">已保存</span>
      <span class="save-indicator unsaved" v-else>未保存</span>
    </div>

    <!-- Editor -->
    <editor-content ref="editorContentRef" :editor="editor" class="tiptap-editor" />

    <!-- Dict Lookup Popover -->
    <DictLookupPopover
      v-if="popoverVisible"
      :text="popoverText"
      :position="popoverPosition"
      :novel-id="props.novelId"
      @close="closePopover"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { useEditor, EditorContent } from '@tiptap/vue-3'
import type { EditorView } from '@tiptap/pm/view'
import StarterKit from '@tiptap/starter-kit'
import Placeholder from '@tiptap/extension-placeholder'
import { List, Tickets, RefreshLeft, RefreshRight } from '@element-plus/icons-vue'
import { useEditorStore } from '@/stores/editorStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { useDictionaryStore } from '@/stores/dictionaryStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { speakWord } from '@/utils/speech'
import { plainTextToHtml } from '@/utils/editorHtml'
import { cleanText } from '@/utils/textCleaner'
import { VocabHighlight, setVocabHighlightWords, refreshVocabHighlight } from '@/extensions/VocabHighlight'
import type { HighlightWord } from '@/types/vocabWord'
import DictLookupPopover from './DictLookupPopover.vue'

const props = defineProps<{
  novelId: number
  content: string
  highlightWords: HighlightWord[]
  highlightBookId: number | null
}>()

const emit = defineEmits<{
  (e: 'update:content', html: string): void
  (e: 'update:highlightBookId', id: number | null): void
}>()

const store = useEditorStore()
const vocabBookStore = useVocabBookStore()
const dictStore = useDictionaryStore()
const settingsStore = useSettingsStore()
vocabBookStore.fetchAll()

// ===== Dict lookup state =====
const editorContentRef = ref<InstanceType<typeof EditorContent> | null>(null)
const popoverVisible = ref(false)
const popoverPosition = ref({ x: 0, y: 0 })
const popoverText = ref('')
let mouseupTimer: number | null = null
// Flag to prevent mouseup handler from re-firing after a dblclick
let suppressNextMouseup = false

function getSelectionText(): string | null {
  const sel = window.getSelection()
  if (!sel || sel.isCollapsed) return null
  const text = sel.toString().trim()
  return text || null
}

function isEnglishWord(s: string): boolean {
  return /^[A-Za-z][A-Za-z'-]*$/.test(s)
}

function hasChinese(s: string): boolean {
  return /[\u4e00-\u9fa5]/.test(s)
}

function showPopover(x: number, y: number, text: string) {
  popoverText.value = text
  popoverPosition.value = { x, y }
  popoverVisible.value = true
  void dictStore.lookupAuto(text)
  // 选中英文单词时自动朗读（按用户 speech_accent 偏好）
  if (isEnglishWord(text)) {
    speakWord(text, settingsStore.speechAccent)
  }
}

function closePopover() {
  popoverVisible.value = false
  dictStore.clear()
}

function handleDblClick(event: MouseEvent) {
  const text = getSelectionText()
  if (!text) return
  // 双击只处理英文单词
  if (!isEnglishWord(text)) return
  suppressNextMouseup = true
  showPopover(event.clientX, event.clientY, text)
}

function handleMouseup(event: MouseEvent) {
  if (suppressNextMouseup) {
    suppressNextMouseup = false
    return
  }
  if (popoverVisible.value) return
  // 延迟 50ms 让 selection 同步稳定
  if (mouseupTimer) clearTimeout(mouseupTimer)
  mouseupTimer = window.setTimeout(() => {
    const text = getSelectionText()
    if (!text) return
    // 划选英文单词（无空格，限 50 字符）
    if (isEnglishWord(text) && text.length <= 50) {
      showPopover(event.clientX, event.clientY, text)
      return
    }
    // 中文（限 30 字符，避免误选整段）
    if (hasChinese(text) && text.length <= 30) {
      showPopover(event.clientX, event.clientY, text)
    }
    // 含空格的英文短语忽略
  }, 50)
}

function onExternalClick(e: MouseEvent) {
  if (!popoverVisible.value) return
  const target = e.target as HTMLElement
  if (!target.closest('.dict-lookup-popover')) {
    closePopover()
  }
}

onMounted(() => {
  const el = (editorContentRef.value as any)?.$el as HTMLElement | undefined
  el?.addEventListener('dblclick', handleDblClick)
  el?.addEventListener('mouseup', handleMouseup)
  document.addEventListener('mousedown', onExternalClick)
})
onBeforeUnmount(() => {
  const el = (editorContentRef.value as any)?.$el as HTMLElement | undefined
  el?.removeEventListener('dblclick', handleDblClick)
  el?.removeEventListener('mouseup', handleMouseup)
  document.removeEventListener('mousedown', onExternalClick)
  if (mouseupTimer) clearTimeout(mouseupTimer)
})

// Initialize Tiptap with empty content; load the novel body asynchronously
// once the editor is mounted so the initial parse doesn't block the main
// thread on multi-megabyte HTML.
const editor = useEditor({
  content: '',
  extensions: [
    StarterKit.configure({
      heading: { levels: [1, 2, 3] },
    }),
    Placeholder.configure({
      placeholder: '开始编辑小说正文...',
    }),
    VocabHighlight.configure({
      words: props.highlightWords,
    }),
  ],
  editorProps: {
    handlePaste: (_view, event) => {
      // HTML paste: let Tiptap handle rich text natively
      const html = event.clipboardData?.getData('text/html')
      if (html) return false

      // Plain-text paste: clean then insert as paragraphs
      const text = event.clipboardData?.getData('text/plain')
      if (!text) return false

      const cleaned = cleanText(text)
      const result = plainTextToHtml(cleaned)
      editor.value?.commands.insertContent(result)
      return true
    },
  },
  onUpdate({ editor }) {
    try {
      const html = editor.getHTML()
      emit('update:content', html)
      store.scheduleAutosave(props.novelId, html)
    } catch (e) {
      console.warn('[NovelEditor] onUpdate failed:', e)
    }
  },
})

// Explicit "have I loaded this novel's content?" flag — replaces the old
// brittle 50-char length heuristic. Set true once we call setContent; reset
// when novelId changes so route navigation reloads the body.
const loadedNovelId = ref<number | null>(null)

function loadContent(raw: string) {
  if (!editor.value) return
  try {
    const html = plainTextToHtml(raw)
    editor.value.commands.setContent(html, { emitUpdate: false })
  } catch (e) {
    console.error('[NovelEditor] setContent failed:', e)
  }
}

// First load: when editor mounts, defer one macrotask to let Tiptap's
// schema finish wiring up, then push the current content.
watch(
  editor,
  (ed) => {
    if (!ed || loadedNovelId.value === props.novelId) return
    if (!props.content) return
    setTimeout(() => {
      if (!editor.value) return
      loadedNovelId.value = props.novelId
      loadContent(props.content)
    }, 0)
  },
  { immediate: true },
)

// Route navigation: when novelId changes, reset the flag and reload.
watch(
  () => props.novelId,
  (id, prev) => {
    if (id === prev) return
    if (!editor.value || !props.content) return
    loadedNovelId.value = null
    setTimeout(() => {
      if (!editor.value) return
      loadedNovelId.value = id
      loadContent(props.content)
    }, 0)
  },
)

onBeforeUnmount(() => {
  if (!editor.value) return
  // HMR may trigger unmount after the editor has already been partially torn
  // down (schema destroyed). Guard isDestroyed to avoid getHTML failures.
  if (!editor.value.isDestroyed) {
    try {
      const html = editor.value.getHTML()
      if (html) {
        store.flushSave(props.novelId, html)
      }
    } catch (e) {
      console.warn('[NovelEditor] flushSave failed (non-fatal):', e)
    }
  }
  try {
    editor.value.destroy()
  } catch (e) {
    console.warn('[NovelEditor] destroy failed (non-fatal):', e)
  }
})

/**
 * Scroll the editor to the first text node containing `keyword`.
 * Uses ProseMirror's doc descendants to find a real position, then Tiptap's
 * native scrollIntoView command (handles <p>/<br>/<h*> correctly).
 */
function scrollToText(keyword: string): boolean {
  if (!editor.value || !keyword) return false
  try {
    const doc = editor.value.state.doc
    let foundPos: number | null = null
    doc.descendants((node, pos) => {
      if (foundPos !== null) return false
      if (node.isText && node.text && node.text.includes(keyword)) {
        foundPos = pos + node.text.indexOf(keyword)
        return false
      }
      return true
    })
    if (foundPos === null) return false
    editor.value
      .chain()
      .focus()
      .setTextSelection(foundPos)
      .scrollIntoView()
      .run()
    return true
  } catch (e) {
    console.warn('[NovelEditor] scrollToText failed:', e)
    return false
  }
}

// Keep VocabHighlight extension in sync when highlightWords change externally
watch(
  () => props.highlightWords,
  (words) => {
    setVocabHighlightWords(words)
    const view = (editor.value as any)?.view as EditorView | undefined
    if (view) {
      refreshVocabHighlight(view)
    }
  },
)

defineExpose({ scrollToText })
</script>

<style scoped>
.novel-editor-wrapper {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.editor-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
  background: var(--bg-secondary, #fafafa);
  flex-wrap: wrap;
}

.toolbar-group {
  margin-right: 4px;
}

.save-indicator {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-secondary);
}
.save-indicator.saved { color: var(--success-color, #67c23a); }
.save-indicator.unsaved { color: var(--warning-color, #e6a23c); }

:deep(.tiptap-editor) {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}

:deep(.tiptap-editor .ProseMirror) {
  outline: none;
  min-height: 100%;
  font-size: 15px;
  line-height: 1.8;
  color: var(--text-regular, #303133);
}

:deep(.tiptap-editor .ProseMirror h1) {
  font-size: 22px;
  font-weight: 700;
  margin: 16px 0 8px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
}

:deep(.tiptap-editor .ProseMirror h2) {
  font-size: 19px;
  font-weight: 600;
  margin: 14px 0 6px;
}

:deep(.tiptap-editor .ProseMirror h3) {
  font-size: 17px;
  font-weight: 600;
  margin: 12px 0 4px;
}

:deep(.tiptap-editor .ProseMirror p) {
  margin: 0 0 8px;
}

:deep(.tiptap-editor .ProseMirror ul),
:deep(.tiptap-editor .ProseMirror ol) {
  padding-left: 24px;
  margin: 4px 0 8px;
}

:deep(.tiptap-editor .ProseMirror blockquote) {
  border-left: 3px solid var(--accent-color, #409eff);
  padding-left: 12px;
  margin: 8px 0;
  color: var(--text-secondary);
}

:deep(.tiptap-editor .ProseMirror p.is-editor-empty:first-child::before) {
  content: attr(data-placeholder);
  color: var(--text-placeholder, #c0c4cc);
  float: left;
  height: 0;
  pointer-events: none;
}
</style>