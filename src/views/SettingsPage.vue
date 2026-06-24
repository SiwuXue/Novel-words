<template>
  <div class="settings-page">
    <h2>设置</h2>

    <el-tabs v-model="activeTab" class="settings-tabs">
      <!-- PDF Templates tab -->
      <el-tab-pane label="PDF 模板" name="templates">
        <div class="tab-header">
          <p class="tab-desc">管理 PDF 导出模板：纸张大小、字体、注释模式等</p>
          <el-button type="primary" size="small" @click="showCreate">
            <el-icon><Plus /></el-icon> 新建模板
          </el-button>
        </div>

        <el-table
          v-loading="templateStore.loading"
          :data="templateStore.templates"
          stripe
          empty-text="暂无 PDF 模板"
          style="width:100%"
        >
          <el-table-column prop="name" label="名称" min-width="120" />
          <el-table-column prop="paperSize" label="纸张" width="80" />
          <el-table-column prop="fontFamily" label="字体" width="100" />
          <el-table-column prop="fontSize" label="字号" width="70">
            <template #default="{ row }">{{ row.fontSize }}px</template>
          </el-table-column>
          <el-table-column prop="lineSpacing" label="行距" width="70" />
          <el-table-column prop="annotationMode" label="注释模式" width="110">
            <template #default="{ row }">
              {{ annotationModeLabel(row.annotationMode) }}
            </template>
          </el-table-column>
          <el-table-column label="操作" width="140">
            <template #default="{ row }">
              <el-button size="small" link type="primary" @click="editTemplate(row)">
                编辑
              </el-button>
              <el-button size="small" link type="danger" @click="confirmDelete(row)">
                删除
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- General tab (placeholder for Step 14) -->
      <el-tab-pane label="通用" name="general">
        <p class="tab-desc">通用设置将在后续版本中提供。</p>
      </el-tab-pane>
    </el-tabs>

    <!-- Template form dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑 PDF 模板' : '新建 PDF 模板'"
      width="480px"
      :close-on-click-modal="false"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item label="名称" prop="name">
          <el-input v-model="form.name" placeholder="如：默认模板" maxlength="100" />
        </el-form-item>
        <el-form-item label="纸张大小" prop="paperSize">
          <el-select v-model="form.paperSize" style="width:100%">
            <el-option label="A4" value="A4" />
            <el-option label="A5" value="A5" />
            <el-option label="自定义" value="Custom" />
          </el-select>
        </el-form-item>
        <el-form-item label="字体" prop="fontFamily">
          <el-select v-model="form.fontFamily" style="width:100%">
            <el-option label="宋体 (SimSun)" value="SimSun" />
            <el-option label="黑体 (SimHei)" value="SimHei" />
            <el-option label="楷体 (KaiTi)" value="KaiTi" />
            <el-option label="微软雅黑" value="Microsoft YaHei" />
            <el-option label="Arial" value="Arial" />
            <el-option label="Times New Roman" value="Times New Roman" />
          </el-select>
        </el-form-item>
        <el-form-item label="字号" prop="fontSize">
          <el-input-number v-model="form.fontSize" :min="8" :max="24" />
          <span style="margin-left:8px;color:#909399;font-size:13px;">px</span>
        </el-form-item>
        <el-form-item label="行距" prop="lineSpacing">
          <el-input-number v-model="form.lineSpacing" :min="1.0" :max="3.0" :step="0.1" :precision="1" />
        </el-form-item>
        <el-form-item label="注释模式" prop="annotationMode">
          <el-select v-model="form.annotationMode" style="width:100%">
            <el-option label="行内标注 — 生词后附释义" value="inline" />
            <el-option label="侧边栏 — 释义显示在侧边" value="sidebar" />
            <el-option label="文末附录 — 文末生成词汇表" value="appendix" />
            <el-option label="无注释 — 仅原文" value="none" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit" :loading="submitting">
          {{ isEdit ? '保存' : '创建' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { Plus } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { usePdfTemplateStore } from '@/stores/pdfTemplateStore'
import type { PdfTemplate } from '@/types/pdf'

const templateStore = usePdfTemplateStore()

const activeTab = ref('templates')
const dialogVisible = ref(false)
const isEdit = ref(false)
const editingId = ref<number | null>(null)
const submitting = ref(false)
const formRef = ref<FormInstance>()

const defaultMargins = JSON.stringify({ top: 25, bottom: 25, left: 20, right: 20 })

const form = reactive({
  name: '',
  paperSize: 'A4' as 'A4' | 'A5' | 'Custom',
  fontFamily: 'SimSun',
  fontSize: 14,
  lineSpacing: 1.5,
  annotationMode: 'appendix' as 'inline' | 'sidebar' | 'appendix' | 'none',
  margins: JSON.stringify({ top: 25, bottom: 25, left: 20, right: 20 }),
})

const rules: FormRules = {
  name: [{ required: true, message: '请输入模板名称', trigger: 'blur' }],
}

function annotationModeLabel(mode: string): string {
  switch (mode) {
    case 'inline': return '行内标注'
    case 'sidebar': return '侧边栏'
    case 'appendix': return '文末附录'
    case 'none': return '无注释'
    default: return mode
  }
}

function showCreate() {
  isEdit.value = false
  editingId.value = null
  form.name = ''
  form.paperSize = 'A4'
  form.fontFamily = 'SimSun'
  form.fontSize = 14
  form.lineSpacing = 1.5
  form.annotationMode = 'appendix'
  form.margins = defaultMargins
  dialogVisible.value = true
}

function editTemplate(tpl: PdfTemplate) {
  isEdit.value = true
  editingId.value = tpl.id
  form.name = tpl.name
  form.paperSize = tpl.paperSize
  form.fontFamily = tpl.fontFamily
  form.fontSize = tpl.fontSize
  form.lineSpacing = tpl.lineSpacing
  form.annotationMode = tpl.annotationMode
  form.margins = tpl.margins
  dialogVisible.value = true
}

async function handleSubmit() {
  if (!formRef.value) return
  const valid = await formRef.value.validate().catch(() => false)
  if (!valid) return

  submitting.value = true
  try {
    if (isEdit.value && editingId.value) {
      await templateStore.update(editingId.value, {
        name: form.name,
        paperSize: form.paperSize,
        fontFamily: form.fontFamily,
        fontSize: form.fontSize,
        lineSpacing: form.lineSpacing,
        margins: form.margins,
        annotationMode: form.annotationMode,
      })
      ElMessage.success('模板已更新')
    } else {
      await templateStore.create({
        name: form.name,
        paperSize: form.paperSize,
        fontFamily: form.fontFamily,
        fontSize: form.fontSize,
        lineSpacing: form.lineSpacing,
        margins: form.margins,
        annotationMode: form.annotationMode,
      })
      ElMessage.success('模板已创建')
    }
    dialogVisible.value = false
  } catch (e: any) {
    ElMessage.error(String(e?.message || e || '操作失败'))
  } finally {
    submitting.value = false
  }
}

async function confirmDelete(tpl: PdfTemplate) {
  try {
    await ElMessageBox.confirm(
      `确定删除模板「${tpl.name}」吗？`,
      '确认删除',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' },
    )
    await templateStore.remove(tpl.id)
    ElMessage.success('已删除')
  } catch {
    // user cancelled
  }
}

onMounted(() => {
  templateStore.fetchAll()
})
</script>

<style scoped>
.settings-page {
  padding: 24px;
}
.settings-page h2 {
  margin: 0 0 16px 0;
  font-size: 20px;
}
.settings-tabs {
  margin-top: 8px;
}
.tab-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 16px;
}
.tab-desc {
  margin: 0 0 16px 0;
  color: var(--text-secondary, #909399);
  font-size: 14px;
}
</style>
