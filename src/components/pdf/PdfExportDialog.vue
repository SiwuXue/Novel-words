<template>
  <el-dialog
    v-model="visible"
    title="导出 PDF"
    width="560px"
    :close-on-click-modal="false"
  >
    <el-form label-width="80px">
      <el-form-item label="排版模板" v-if="selectedTemplate">
        <el-tag>{{ selectedTemplate.name }}</el-tag>
        <span style="margin-left:8px;font-size:12px;color:var(--text-secondary)">
          {{ selectedTemplate.paperSize }} · {{ selectedTemplate.fontSize }}px · 行距{{ selectedTemplate.lineSpacing }}
        </span>
      </el-form-item>

      <el-form-item label="词汇本">
        <el-select
          v-model="selectedVocabBookId"
          placeholder="选择词汇本（可选）"
          clearable
          style="width:100%"
        >
          <el-option
            v-for="book in bookStore.books"
            :key="book.id"
            :label="book.name"
            :value="book.id"
          />
        </el-select>
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" @click="handleExport" :loading="exporting">
        导出
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { usePdfTemplateStore } from '@/stores/pdfTemplateStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { useNovelStore } from '@/stores/novelStore'
import type { PdfTemplate } from '@/types/pdf'

const props = defineProps<{
  modelValue: boolean
  templateType?: string
  vocabBookId?: number | null
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
}>()

const visible = ref(props.modelValue)
watch(() => props.modelValue, (v) => { visible.value = v })
watch(visible, (v) => { emit('update:modelValue', v) })

const templateStore = usePdfTemplateStore()
const bookStore = useVocabBookStore()
const novelStore = useNovelStore()

const selectedTemplateId = ref<number | null>(null)
const selectedVocabBookId = ref<number | null>(null)
const exporting = ref(false)

const builtinTemplates = computed(() => templateStore.builtinTemplates)

const selectedTemplate = ref<PdfTemplate | null>(null)

function selectTemplate(tpl: PdfTemplate) {
  selectedTemplate.value = tpl
  selectedTemplateId.value = tpl.isBuiltin ? null : tpl.id
}

onMounted(async () => {
  await templateStore.fetchBuiltin()
  await templateStore.fetchAll()
  if (bookStore.books.length === 0) {
    await bookStore.fetchAll()
  }
  // Pre-select from parent props
  selectedVocabBookId.value = props.vocabBookId ?? null
  // Find the builtin template matching the given type
  const tpl = builtinTemplates.value.find(
    (t) => t.templateType === (props.templateType || 'intensive'),
  ) ?? builtinTemplates.value[0]
  if (tpl) selectTemplate(tpl)
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

async function handleExport() {
  const novel = novelStore.currentNovel
  if (!novel) {
    ElMessage.error('请先打开小说')
    return
  }

  console.log('[PdfExport] opening save dialog...')
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
  if (!filePath) {
    console.log('[PdfExport] user cancelled save dialog')
    return
  }

  console.log('[PdfExport] invoking export_pdf...', {
    novelId: novel.id,
    templateId: selectedTemplateId.value,
    templateType: selectedTemplate.value?.templateType,
    outputPath: filePath,
  })
  exporting.value = true
  try {
    const result = await invoke<string>('export_pdf', {
      novelId: novel.id,
      templateId: selectedTemplateId.value,
      templateType: selectedTemplate.value?.templateType || 'intensive',
      vocabBookId: selectedVocabBookId.value,
      outputPath: filePath,
    })
    console.log('[PdfExport] export_pdf succeeded:', result)
    ElMessage.success('PDF 已导出')
    visible.value = false
  } catch (e: any) {
    console.error('[PdfExport] export_pdf failed:', e)
    ElMessage.error(String(e?.message || e || '导出失败'))
  } finally {
    exporting.value = false
  }
}
</script>

<style scoped>
.template-selector { width: 100%; }
.template-group { margin-bottom: 16px; }
.group-label {
  font-size: 13px;
  color: var(--text-secondary, #909399);
  margin-bottom: 8px;
  font-weight: 500;
}
.template-card {
  border: 1px solid var(--border-color, #dcdfe6);
  border-radius: 6px;
  padding: 10px 12px;
  margin-bottom: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 10px;
  transition: border-color 0.2s;
}
.template-card:hover { border-color: var(--accent-color, #409eff); }
.template-card.selected {
  border-color: var(--accent-color, #409eff);
  background: var(--accent-light, #ecf5ff);
}
.tpl-name { font-weight: 600; white-space: nowrap; }
.tpl-desc { flex: 1; font-size: 12px; color: var(--text-secondary, #909399); }
</style>
