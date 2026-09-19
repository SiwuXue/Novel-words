<script setup lang="ts">
/**
 * 圆形图标按钮（移植自 ColorTxt IconButton.vue，裁剪掉本组件未用的变体）。
 * 胶囊内的层按钮为透明底，中央主控键与左右半悬圆钮由父组件配色。
 */
withDefaults(
  defineProps<{
    /** SVG 字符串（icons.xxx，?raw 导入） */
    iconHtml?: string
    title?: string
    /** 图标按钮无文字时建议设置，便于读屏 */
    ariaLabel?: string
    disabled?: boolean
  }>(),
  { disabled: false },
)

defineEmits<{ click: [e: MouseEvent] }>()
</script>

<template>
  <button
    type="button"
    class="ttsIconBtn"
    :title="title"
    :aria-label="ariaLabel"
    :disabled="disabled"
    @click="$emit('click', $event)"
  >
    <span class="icon" v-html="iconHtml"></span>
  </button>
</template>

<style scoped>
.ttsIconBtn {
  background: transparent;
  border: none;
  border-radius: 50%;
  width: 30px;
  height: 30px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition:
    background 0.16s ease,
    color 0.16s ease;
}

.ttsIconBtn:hover:not(:disabled) {
  background: var(--tts-icon-hover-bg);
}

.icon {
  width: 16px;
  height: 16px;
  display: inline-flex;
  color: var(--tts-icon-fg);
}

.icon :deep(svg) {
  width: 16px;
  height: 16px;
  display: block;
}

.icon :deep(svg path) {
  fill: currentColor;
}

.ttsIconBtn:focus {
  outline: none;
}

.ttsIconBtn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
