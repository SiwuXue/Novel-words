import { onBeforeUnmount, ref, watch, type Ref } from 'vue'
import {
  clampTimedScrollIntervalMs,
  type TimedScrollSettings,
} from '@/utils/timedScroll'

/**
 * 定时自动滚动（参考 ColorTxt useAppTimedScroll.ts）：
 * - 按间隔每次向下滚一行/一屏，到底自动停止；
 * - 手动滚动后调用 nudgeTimedScrollTimer 重新计满间隔；
 * - 朗读/切章等场景由调用方在互斥时机调用 stopTimedScroll。
 */
export function useTimedScroll(deps: {
  /** 滚动容器 getter（NovelEditor.getScrollEl），不存在时 tick 空转 */
  getScrollEl: () => HTMLElement | null
  settings: Ref<TimedScrollSettings>
  /** 是否允许启动（如内容未就绪/编辑模式则禁用） */
  canStart: () => boolean
}) {
  const active = ref(false)
  let timerId: ReturnType<typeof setInterval> | null = null

  function clearTimer() {
    if (timerId !== null) {
      clearInterval(timerId)
      timerId = null
    }
  }

  function stopTimedScroll() {
    active.value = false
    clearTimer()
  }

  function startTimer() {
    clearTimer()
    const ms = clampTimedScrollIntervalMs(deps.settings.value.intervalMs)
    timerId = setInterval(tick, ms)
  }

  /** 手动滚动后重新计满间隔，避免刚操作完立刻再自动滚。 */
  function nudgeTimedScrollTimer() {
    if (!active.value) return
    startTimer()
  }

  function startTimedScroll() {
    if (active.value || !deps.canStart()) return
    active.value = true
    startTimer()
  }

  function toggleTimedScroll() {
    if (active.value) {
      stopTimedScroll()
      return
    }
    startTimedScroll()
  }

  function viewportAtBottom(el: HTMLElement): boolean {
    return el.scrollTop + el.clientHeight >= el.scrollHeight - 2
  }

  function lineHeightOf(el: HTMLElement): number {
    const lh = Number.parseFloat(window.getComputedStyle(el).lineHeight)
    return Number.isFinite(lh) && lh > 0 ? lh : 24
  }

  function tick() {
    if (!active.value) return
    const el = deps.getScrollEl()
    if (!el) return
    if (viewportAtBottom(el)) {
      stopTimedScroll()
      return
    }
    if (deps.settings.value.range === 'line') {
      el.scrollBy({ top: lineHeightOf(el), behavior: 'auto' })
    } else {
      el.scrollBy({ top: el.clientHeight * 0.82, behavior: 'smooth' })
    }
  }

  watch(
    () => deps.settings.value,
    () => {
      if (active.value) startTimer()
    },
    { deep: true },
  )

  onBeforeUnmount(() => stopTimedScroll())

  return {
    isTimedScrollActive: active,
    toggleTimedScroll,
    startTimedScroll,
    stopTimedScroll,
    nudgeTimedScrollTimer,
  }
}
