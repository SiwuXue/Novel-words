<script setup lang="ts">
/**
 * 轻量滑杆（移植自 ColorTxt RangeSlider.vue，裁剪版：保留渐变轨道与悬停值提示）。
 * 数值域由父组件决定（语速 0.5–2 / 音量 0–100），组件不做单位假设。
 */
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

const modelValue = defineModel<number>({ required: true })

const props = withDefaults(
  defineProps<{
    min?: number
    max?: number
    step?: number
    ariaLabel?: string
    disabled?: boolean
  }>(),
  {
    min: 0,
    max: 100,
    step: 1,
    ariaLabel: '',
    disabled: false,
  },
)

const wrapRef = ref<HTMLElement | null>(null)
const hovering = ref(false)
const tipPos = ref({ x: 0, y: 0 })

const safeRange = computed(() => Math.max(1e-9, props.max - props.min))

const progressPercent = computed(() => {
  const ratio = (modelValue.value - props.min) / safeRange.value
  return Math.max(0, Math.min(100, ratio * 100))
})

function formatValue(v: number, step: number): string {
  const stepText = String(step)
  const dot = stepText.indexOf('.')
  const decimals = dot === -1 ? 0 : stepText.length - dot - 1
  return Number(v.toFixed(decimals)).toString()
}

const displayText = computed(() => formatValue(modelValue.value, props.step))

const showTip = computed(() => hovering.value && !props.disabled)

function normalize(raw: number): number {
  let v = Number.isFinite(raw) ? raw : props.min
  if (props.step > 0) {
    v = Math.round((v - props.min) / props.step) * props.step + props.min
  }
  v = Math.max(props.min, Math.min(props.max, v))
  return Number(v.toFixed(4))
}

function updateTipPos(): void {
  const wrap = wrapRef.value
  if (!wrap) return
  const input = wrap.querySelector<HTMLInputElement>('.ttsRangeInput')
  if (!input) return
  const rect = input.getBoundingClientRect()
  const thumbRadius = 8
  const travel = Math.max(0, rect.width - thumbRadius * 2)
  const ratio = progressPercent.value / 100
  tipPos.value = {
    x: rect.left + thumbRadius + travel * ratio,
    y: rect.top,
  }
}

function onPointerEnter(): void {
  hovering.value = true
  updateTipPos()
}

function onPointerMove(): void {
  if (!hovering.value) return
  updateTipPos()
}

function onPointerLeave(): void {
  hovering.value = false
}

function onInput(ev: Event): void {
  const el = ev.target as HTMLInputElement
  modelValue.value = normalize(el.valueAsNumber)
  updateTipPos()
}

function onChange(ev: Event): void {
  const el = ev.target as HTMLInputElement
  const next = normalize(el.valueAsNumber)
  modelValue.value = next
  el.value = String(next)
  updateTipPos()
}

watch(progressPercent, () => {
  if (hovering.value) updateTipPos()
})

onMounted(() => {
  window.addEventListener('scroll', updateTipPos, true)
  window.addEventListener('resize', updateTipPos)
})

onUnmounted(() => {
  window.removeEventListener('scroll', updateTipPos, true)
  window.removeEventListener('resize', updateTipPos)
})
</script>

<template>
  <div
    ref="wrapRef"
    class="ttsRangeSlider"
    :class="{ 'ttsRangeSlider--disabled': disabled }"
    @pointerenter="onPointerEnter"
    @pointermove="onPointerMove"
    @pointerleave="onPointerLeave"
  >
    <input
      class="ttsRangeInput"
      type="range"
      :value="modelValue"
      :min="min"
      :max="max"
      :step="step"
      :disabled="disabled"
      :aria-label="ariaLabel || undefined"
      :style="{ '--range-progress': `${progressPercent}%` }"
      @input="onInput"
      @change="onChange"
    />

    <Teleport to="body">
      <span
        v-if="showTip"
        class="ttsRangeHoverValue"
        :style="{ left: `${tipPos.x}px`, top: `${tipPos.y}px` }"
        aria-hidden="true"
      >
        {{ displayText }}
      </span>
    </Teleport>
  </div>
</template>

<style scoped>
.ttsRangeSlider {
  width: 100%;
  display: inline-flex;
  align-items: center;
}

.ttsRangeSlider--disabled {
  opacity: 0.55;
}

.ttsRangeInput {
  width: 100%;
  height: 20px;
  margin: 0;
  appearance: none;
  background: transparent;
  cursor: pointer;
}

.ttsRangeSlider--disabled .ttsRangeInput {
  cursor: not-allowed;
}

.ttsRangeInput::-webkit-slider-runnable-track {
  height: 6px;
  border-radius: 999px;
  background: linear-gradient(
    to right,
    var(--tts-primary) 0%,
    var(--tts-primary) var(--range-progress),
    var(--tts-border) var(--range-progress),
    var(--tts-border) 100%
  );
}

.ttsRangeInput::-webkit-slider-thumb {
  appearance: none;
  margin-top: -5px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 2px solid var(--tts-primary);
  background: var(--tts-thumb-bg);
}

.ttsRangeInput::-moz-range-track {
  height: 6px;
  border-radius: 999px;
  background: var(--tts-border);
}

.ttsRangeInput::-moz-range-progress {
  height: 6px;
  border-radius: 999px;
  background: var(--tts-primary);
}

.ttsRangeInput::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 1px solid var(--tts-primary);
  background: var(--tts-thumb-bg);
}
</style>

<style>
/* 悬停值提示：Teleport 到 body，需全局样式 */
.ttsRangeHoverValue {
  position: fixed;
  z-index: 10000;
  transform: translate(-50%, calc(-100% - 2px));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 20px;
  min-width: 30px;
  padding: 4px 6px;
  font-size: 11px;
  line-height: 1;
  border-radius: 4px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  color: var(--text-primary);
  box-shadow: 0 2px 8px color-mix(in srgb, #000 14%, transparent);
  white-space: nowrap;
  pointer-events: none;
  font-variant-numeric: tabular-nums;
}
</style>
