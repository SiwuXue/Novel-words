<template>
  <div class="chapter-list-panel">
    <div class="panel-header">
      <h4>章节目录</h4>
      <span class="ch-count">{{ chapters.length }} 章</span>
    </div>
    <div class="chapter-list" v-if="chapters.length > 0">
      <div
        v-for="(ch, i) in chapters"
        :key="i"
        class="chapter-item"
        :class="{ active: i === activeIndex }"
        @click="$emit('select', i)"
      >
        <span class="ch-index">{{ i + 1 }}</span>
        <span class="ch-label">{{ ch.title }}</span>
      </div>
    </div>
    <el-empty v-else description="暂无章节数据" :image-size="60" />
  </div>
</template>

<script setup lang="ts">
import type { Chapter } from '@/types/novel'

defineProps<{
  chapters: Chapter[]
  activeIndex: number
}>()

defineEmits<{
  (e: 'select', index: number): void
}>()
</script>

<style scoped>
.chapter-list-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border-color, #ebeef5);
  background: var(--bg-secondary, #fafafa);
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color, #ebeef5);
}

.panel-header h4 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.ch-count {
  font-size: 12px;
  color: var(--text-secondary);
}

.chapter-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.chapter-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 16px;
  cursor: pointer;
  font-size: 13px;
  transition: background 0.15s;
}

.chapter-item:hover {
  background: var(--hover-bg, #f0f2f5);
}

.chapter-item.active {
  background: var(--accent-light, #ecf5ff);
  color: var(--accent-color, #409eff);
}

.ch-index {
  flex-shrink: 0;
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  background: var(--bg-color, #fff);
  font-size: 11px;
  color: var(--text-secondary);
}

.chapter-item.active .ch-index {
  background: var(--accent-color, #409eff);
  color: #fff;
}

.ch-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
