<template>
  <el-dialog
    v-model="visible"
    title="关于"
    width="420px"
    :close-on-click-modal="false"
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
import { ref, watch } from 'vue'
import { Document, Collection, Printer } from '@element-plus/icons-vue'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
}>()

const visible = ref(props.modelValue)
watch(() => props.modelValue, (v) => { visible.value = v })
watch(visible, (v) => { emit('update:modelValue', v) })

const version = '0.1.0'
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

.about-tech {
  font-size: 12px;
  color: var(--text-placeholder, #c0c4cc);
  margin: 0;
}
</style>
