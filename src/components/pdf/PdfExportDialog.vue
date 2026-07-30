<template>
  <el-dialog
    v-model="visible"
    title="导出 PDF"
    width="560px"
    :close-on-click-modal="false"
  >
    <el-form label-width="80px">
      <el-form-item label="排版模板">
        <div class="template-selector">
          <!-- Built-in templates -->
          <div class="template-group">
            <div class="group-label">内置模板</div>
            <div
              v-for="tpl in builtinTemplates"
              :key="'b-' + tpl.id"
              class="template-card"
              :class="{ selected: selectedTemplateId === tpl.id }"
              @click="selectTemplate(tpl)"
            >
              <div class="tpl-name">{{ tpl.name }}</div>
              <div class="tpl-desc">{{ templateDesc(tpl) }}</div>
              <el-tag size="small" type="info">内置</el-tag>
            </div>
          </div>

          <!-- User templates -->
          <div class="template-group">
            <div class="group-label">我的模板</div>
            <div
              v-for="tpl in userTemplates"
              :key="'u-' + tpl.id"
              class="template-card"
              :class="{ selected: selectedTemplateId === tpl.id }"
              @click="selectTemplate(tpl)"
            >
              <div class="tpl-name">{{ tpl.name }}</div>
              <div class="tpl-desc">{{ tpl.paperSize }} · {{ tpl.fontSize }}px · 行距{{ tpl.lineSpacing }}</div>
            </div>
            <el-button v-if="selectedBuiltin" text type="primary" size="small" @click="saveAsCustom">
              从「{{ selectedBuiltin.name }}」另存为我的模板
            </el-button>
          </div>
        </div>
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

      <template v-if="selectedTemplate">
        <el-divider content-position="left">模板参数</el-divider>
        <el-form-item label="纸张">{{ selectedTemplate.paperSize }}</el-form-item>
        <el-form-item label="字体">{{ selectedTemplate.fontFamily }}</el-form-item>
        <el-form-item label="字号">{{ selectedTemplate.fontSize }}px</el-form-item>
        <el-form-item label="行距">{{ selectedTemplate.lineSpacing }}</el-form-item>
      </template>
      <el-alert
        v-else
        title="选择一个模板后显示参数"
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
import { TEMPLATE_TYPE_LABELS } from '@/types/pdf'
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

const builtinTemplates = computed(() => templateStore.builtinTemplates)
const userTemplates = computed(() => templateStore.templates.filter(t => !t.isBuiltin))

const selectedTemplate = ref<PdfTemplate | null>(null)
const selectedBuiltin = computed(() =>
  selectedTemplate.value?.isBuiltin ? selectedTemplate.value : null
)

function templateDesc(tpl: PdfTemplate): string {
  const label = TEMPLATE_TYPE_LABELS[tpl.templateType] || tpl.templateType
  return label.split(' — ')[1] || label
}

function selectTemplate(tpl: PdfTemplate) {
  selectedTemplate.value = tpl
  // Use negative id for builtin → Rust skips DB lookup
  selectedTemplateId.value = tpl.isBuiltin ? null : tpl.id
}

onMounted(async () => {
  await templateStore.fetchBuiltin()
  await templateStore.fetchAll()
  if (bookStore.books.length === 0) {
    await bookStore.fetchAll()
  }
  // Default select first builtin
  if (builtinTemplates.value.length > 0) {
    selectTemplate(builtinTemplates.value[0])
  }
  if (
    settingsStore.defaultVocabBookId &&
    bookStore.books.some((b) => b.id === settingsStore.defaultVocabBookId)
  ) {
    selectedVocabBookId.value = settingsStore.defaultVocabBookId
  }
})

function saveAsCustom() {
  // Emit event for parent to open PdfTemplateFormDialog
  ElMessage.info('请前往设置页从内置模板创建自定义模板')
}

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
