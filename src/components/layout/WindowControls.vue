<template>
  <div class="window-controls">
    <button class="ctrl-btn" @click="minimize" title="最小化">
      <svg width="12" height="12" viewBox="0 0 12 12"><rect y="5" width="12" height="1.5" fill="currentColor"/></svg>
    </button>
    <button class="ctrl-btn" @click="toggleMaximize" title="最大化">
      <svg width="12" height="12" viewBox="0 0 12 12"><rect x="1" y="1" width="10" height="10" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
    </button>
    <button class="ctrl-btn ctrl-close" @click="closeWindow" title="关闭">
      <svg width="12" height="12" viewBox="0 0 12 12">
        <line x1="1" y1="1" x2="11" y2="11" stroke="currentColor" stroke-width="1.5"/>
        <line x1="11" y1="1" x2="1" y2="11" stroke="currentColor" stroke-width="1.5"/>
      </svg>
    </button>
  </div>
</template>

<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'

const appWindow = getCurrentWindow()

function minimize() { appWindow.minimize() }
function toggleMaximize() { appWindow.toggleMaximize() }
function closeWindow() { appWindow.close() }
</script>

<style scoped>
.window-controls {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-left: auto;
}
.ctrl-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.15s;
}
.ctrl-btn:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
}
.ctrl-close:hover {
  background: var(--danger-color);
  color: #fff;
}
</style>
