<template>
  <el-dialog
    v-model="visible"
    title="关于"
    width="420px"
    :close-on-click-modal="false"
    :append-to-body="true"
    center
  >
    <div class="about-body">
      <div class="about-icon">📖</div>
      <h2 class="about-title">词阅</h2>
      <p class="about-version">v{{ version }}</p>
      <p class="about-desc">本地小说阅读与词汇管理，助力外语学习</p>

      <el-divider />

      <div class="about-features">
        <div class="feature-row">
          <el-icon><Document /></el-icon>
          <span>导入 TXT 小说，自动编码检测与排版清洗</span>
        </div>
        <div class="feature-row">
          <el-icon><Collection /></el-icon>
          <span>创建词汇本，阅读中高亮生词、悬浮释义</span>
        </div>
        <div class="feature-row">
          <el-icon><Printer /></el-icon>
          <span>导出带注释的 PDF，支持行内标注/文末附录</span>
        </div>
      </div>

      <el-divider />

      <div class="about-data">
        <div class="about-data-label">数据存储路径</div>
        <div class="about-data-path">{{ dataDir || '加载中…' }}</div>
        <div class="about-data-actions">
          <el-button size="small" @click="copyDataDir">复制</el-button>
          <el-button size="small" @click="openDataDir">打开目录</el-button>
        </div>
      </div>

      <el-divider />

      <p class="about-tech">
        Tauri v2 · Vue 3 · TypeScript · Element Plus · Tiptap · Rust · SQLite
      </p>
    </div>

    <template #footer>
      <el-button @click="visible = false">关闭</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { Document, Collection, Printer } from '@element-plus/icons-vue'
import { invoke } from '@tauri-apps/api/core'
import { openPath } from '@tauri-apps/plugin-opener'
import { ElMessage } from 'element-plus'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
}>()

const visible = ref(props.modelValue)
watch(() => props.modelValue, (v) => { visible.value = v })
watch(visible, (v) => { emit('update:modelValue', v) })

const version = ref('')
const dataDir = ref('')

onMounted(async () => {
  try {
    const info = await invoke<{ version: string; dataDir: string }>('get_app_info')
    version.value = info.version
    dataDir.value = info.dataDir
  } catch (e) {
    console.error('[AboutDialog] get_app_info failed:', e)
  }
})

async function copyDataDir() {
  if (!dataDir.value) return
  try {
    await navigator.clipboard.writeText(dataDir.value)
    ElMessage.success('已复制路径')
  } catch {
    ElMessage.error('复制失败，请手动选择路径复制')
  }
}

async function openDataDir() {
  if (!dataDir.value) return
  try {
    await openPath(dataDir.value)
  } catch (e: any) {
    ElMessage.error('打开目录失败: ' + String(e?.message || e))
  }
}
</script>

<style scoped>
.about-body {
  text-align: center;
}

.about-icon {
  font-size: 48px;
  margin-bottom: 12px;
}

.about-title {
  font-size: 20px;
  font-weight: 700;
  margin: 0 0 4px 0;
  color: var(--text-regular, #303133);
}

.about-version {
  font-size: 13px;
  color: var(--text-secondary, #909399);
  margin: 0 0 8px 0;
}

.about-desc {
  font-size: 14px;
  color: var(--text-secondary, #909399);
  margin: 0;
}

.about-features {
  text-align: left;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.feature-row {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  color: var(--text-regular, #303133);
}

.feature-row .el-icon {
  color: var(--accent-color, #409eff);
  flex-shrink: 0;
}

.about-data {
  text-align: left;
}

.about-data-label {
  font-size: 12px;
  color: var(--text-secondary, #909399);
  margin-bottom: 6px;
}

.about-data-path {
  font-size: 12px;
  color: var(--text-regular, #303133);
  background: var(--bg-secondary, #f5f7fa);
  border-radius: 6px;
  padding: 8px 10px;
  word-break: break-all;
  user-select: text;
  margin-bottom: 8px;
}

.about-data-actions {
  display: flex;
  gap: 8px;
}

.about-tech {
  font-size: 12px;
  color: var(--text-placeholder, #c0c4cc);
  margin: 0;
}
</style>
