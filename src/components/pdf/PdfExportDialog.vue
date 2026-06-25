<template>
  <el-dialog
    v-model="visible"
    title="导出 PDF"
    width="480px"
    :close-on-click-modal="false"
  >
    <el-form label-width="90px">
      <el-form-item label="PDF 模板">
        <el-select v-model="selectedTemplateId" placeholder="选择模板" style="width:100%">
          <el-option
            v-for="tpl in templateStore.templates"
            :key="tpl.id"
            :label="tpl.name"
            :value="tpl.id"
          />
        </el-select>
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

      <template v-if="currentTemplate">
        <el-divider content-position="left">模板设置</el-divider>
        <el-form-item label="纸张大小">
          <span class="setting-value">{{ currentTemplate.paperSize }}</span>
        </el-form-item>
        <el-form-item label="字体">
          <span class="setting-value">{{ currentTemplate.fontFamily }}</span>
        </el-form-item>
        <el-form-item label="字号">
          <span class="setting-value">{{ currentTemplate.fontSize }}px</span>
        </el-form-item>
        <el-form-item label="行距">
          <span class="setting-value">{{ currentTemplate.lineSpacing }}</span>
        </el-form-item>
        <el-form-item label="注释模式">
          <el-tag size="small">{{
            annotationModeLabel(currentTemplate.annotationMode)
          }}</el-tag>
        </el-form-item>
      </template>
      <el-alert
        v-else
        title="尚未创建 PDF 模板，将使用默认设置导出"
        type="info"
        :closable="false"
        show-icon
      />
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
import { useSettingsStore } from '@/stores/settingsStore'
import type { PdfTemplate } from '@/types/pdf'

const props = defineProps<{
  modelValue: boolean
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
const settingsStore = useSettingsStore()

const selectedTemplateId = ref<number | null>(null)
const selectedVocabBookId = ref<number | null>(null)
const exporting = ref(false)

const currentTemplate = computed<PdfTemplate | null>(() => {
  if (!selectedTemplateId.value) return null
  return templateStore.templates.find((t) => t.id === selectedTemplateId.value) || null
})

function annotationModeLabel(mode: string): string {
  switch (mode) {
    case 'inline': return '行内标注'
    case 'sidebar': return '侧边栏'
    case 'appendix': return '文末附录'
    case 'none': return '无注释'
    default: return mode
  }
}

onMounted(async () => {
  if (templateStore.templates.length === 0) {
    await templateStore.fetchAll()
  }
  if (bookStore.books.length === 0) {
    await bookStore.fetchAll()
  }
  // Pre-select first template if available
  if (templateStore.templates.length > 0) {
    selectedTemplateId.value = templateStore.templates[0].id
  }
  // Pre-select default vocab book from settings
  if (
    settingsStore.defaultVocabBookId &&
    bookStore.books.some((b) => b.id === settingsStore.defaultVocabBookId)
  ) {
    selectedVocabBookId.value = settingsStore.defaultVocabBookId
  }
})

async function handleExport() {
  const novel = novelStore.currentNovel
  if (!novel) {
    ElMessage.error('请先打开小说')
    return
  }

  // Ask user where to save the PDF
  const filePath = await save({
    defaultPath: `${novel.title || 'export'}.pdf`,
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
  })

  if (!filePath) return // User cancelled

  exporting.value = true
  try {
    await invoke<string>('export_pdf', {
      novelId: novel.id,
      templateId: selectedTemplateId.value,
      vocabBookId: selectedVocabBookId.value,
      outputPath: filePath,
    })
    ElMessage.success('PDF 已导出')
    visible.value = false
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '导出失败'))
  } finally {
    exporting.value = false
  }
}
</script>

<style scoped>
.setting-value {
  color: var(--text-secondary, #909399);
  font-size: 14px;
}
</style>
