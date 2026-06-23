<template>
  <el-dialog
    v-model="visible"
    title="导入文本文件"
    width="680px"
    :close-on-click-modal="false"
    destroy-on-close
    @closed="emit('close')"
  >
    <!-- Step 1: choose file -->
    <div v-show="step === 1" class="import-step">
      <div class="drop-zone" @click="selectFile">
        <el-icon :size="48" color="var(--text-secondary)"><FolderOpened /></el-icon>
        <p>点击选择 .txt / .md 文件</p>
        <p class="hint">支持 UTF-8 / GBK 编码</p>
      </div>
      <div v-if="filePath" class="selected-file">
        <el-tag type="info" size="small">{{ fileName }}</el-tag>
      </div>
      <div class="step-footer">
        <el-button @click="visible = false">取消</el-button>
        <el-button type="primary" :disabled="!filePath" :loading="analyzing" @click="analyzeFile">
          分析文件
        </el-button>
      </div>
    </div>

    <!-- Step 2: preview chapters & confirm -->
    <div v-show="step === 2" class="import-step">
      <el-descriptions :column="2" border size="small">
        <el-descriptions-item label="检测书名">
          {{ result?.detectedTitle || '未检测到' }}
        </el-descriptions-item>
        <el-descriptions-item label="章节数">
          {{ result?.chapters.length || 0 }} 章
        </el-descriptions-item>
        <el-descriptions-item label="总字符数">
          {{ (result?.rawText.length || 0).toLocaleString() }}
        </el-descriptions-item>
        <el-descriptions-item label="清洗后字符数">
          {{ (result?.cleanedText.length || 0).toLocaleString() }}
        </el-descriptions-item>
      </el-descriptions>

      <div class="chapter-preview">
        <h4>章节预览（前 10 章）</h4>
        <ul>
          <li v-for="(ch, i) in result?.chapters.slice(0, 10)" :key="i">
            <span class="ch-title">{{ ch.title }}</span>
            <span class="ch-len">{{ ch.content.length.toLocaleString() }} 字</span>
          </li>
        </ul>
      </div>

      <div class="step-footer">
        <el-button @click="step = 1">返回重选</el-button>
        <el-button @click="visible = false">取消</el-button>
        <el-button type="primary" :loading="importing" @click="handleImport">
          确认识别结果 → 导入编辑器
        </el-button>
      </div>
    </div>

    <!-- Error state -->
    <div v-show="step === 3" class="import-step">
      <el-result icon="error" title="导入失败" :sub-title="errorMsg">
        <template #extra>
          <el-button type="primary" @click="step = 1">重试</el-button>
        </template>
      </el-result>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus'
import { FolderOpened } from '@element-plus/icons-vue'
import type { ImportResult } from '@/types/novel'

const emit = defineEmits<{
  (e: 'confirm', result: ImportResult, filePath: string): void
  (e: 'close'): void
}>()

const visible = ref(true)
const step = ref(1)
const filePath = ref('')
const analyzing = ref(false)
const importing = ref(false)
const result = ref<ImportResult | null>(null)
const errorMsg = ref('')

const fileName = computed(() => {
  if (!filePath.value) return ''
  const parts = filePath.value.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1]
})

async function selectFile() {
  const selected = await open({
    multiple: false,
    filters: [
      { name: '文本文件', extensions: ['txt', 'md', 'text'] },
      { name: '所有文件', extensions: ['*'] },
    ],
  })
  if (selected && typeof selected === 'string') {
    filePath.value = selected
  }
}

async function analyzeFile() {
  if (!filePath.value) return
  analyzing.value = true
  try {
    const r = await invoke<ImportResult>('import_text_file', { path: filePath.value })
    result.value = r
    step.value = 2
  } catch (e: any) {
    console.error('[import_text_file] failed:', e)
    errorMsg.value = String(e?.message || e || '未知错误')
    step.value = 3
  } finally {
    analyzing.value = false
  }
}

async function handleImport() {
  if (!result.value) return
  importing.value = true
  try {
    emit('confirm', result.value, filePath.value)
    ElMessage.success(`已导入 ${result.value.detectedTitle || '小说'}，${result.value.chapters.length} 章`)
  } finally {
    importing.value = false
  }
}

</script>

<style scoped>
.import-step {
  min-height: 320px;
  padding: 20px 0;
}
.drop-zone {
  border: 2px dashed var(--border-color, #dcdfe6);
  border-radius: 8px;
  padding: 48px;
  text-align: center;
  cursor: pointer;
  transition: border-color 0.2s;
}
.drop-zone:hover {
  border-color: var(--accent-color, #409eff);
}
.drop-zone p {
  margin: 8px 0 0;
  color: var(--text-regular);
}
.drop-zone .hint {
  color: var(--text-secondary);
  font-size: 13px;
}
.selected-file {
  margin-top: 12px;
  text-align: center;
}
.chapter-preview {
  margin-top: 16px;
  max-height: 280px;
  overflow-y: auto;
}
.chapter-preview h4 {
  margin: 0 0 8px;
  font-size: 14px;
  color: var(--text-secondary);
}
.chapter-preview ul {
  list-style: none;
  padding: 0;
  margin: 0;
}
.chapter-preview li {
  display: flex;
  justify-content: space-between;
  padding: 6px 8px;
  border-radius: 4px;
  font-size: 13px;
}
.chapter-preview li:nth-child(odd) {
  background: var(--bg-secondary);
}
.ch-title {
  color: var(--text-regular);
}
.ch-len {
  color: var(--text-secondary);
  flex-shrink: 0;
}
.import-error {
  text-align: center;
  padding: 32px;
}
.import-error p {
  margin: 16px 0;
  color: var(--danger-color, #f56c6c);
}
.step-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--border-color, #ebeef5);
}
</style>
