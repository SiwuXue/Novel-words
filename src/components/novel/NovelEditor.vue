<template>
  <div class="novel-editor-wrapper">
    <!-- Toolbar -->
    <div class="editor-toolbar" v-if="editor">
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
    <editor-content :editor="editor" class="tiptap-editor" />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onBeforeUnmount } from 'vue'
import { useEditor, EditorContent } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import Placeholder from '@tiptap/extension-placeholder'
import { List, Tickets, RefreshLeft, RefreshRight } from '@element-plus/icons-vue'
import { useEditorStore } from '@/stores/editorStore'
import { plainTextToHtml } from '@/utils/editorHtml'

const props = defineProps<{
  novelId: number
  content: string
}>()

const emit = defineEmits<{
  (e: 'update:content', html: string): void
}>()

const store = useEditorStore()

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
  ],
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
    console.log('[NovelEditor] content loaded, length =', raw.length)
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
  try {
    const html = editor.value.getHTML()
    if (html) {
      store.flushSave(props.novelId, html)
    }
  } catch (e) {
    console.warn('[NovelEditor] flushSave failed (non-fatal):', e)
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