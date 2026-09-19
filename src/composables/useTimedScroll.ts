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
    if (active.value || !deps.canStart()) return false
    active.value = true
    startTimer()
    return true
  }

  /** 返回 'started' | 'stopped' | 'blocked'（canStart 未通过时 blocked，便于调用方提示） */
  function toggleTimedScroll(): 'started' | 'stopped' | 'blocked' {
    if (active.value) {
      stopTimedScroll()
      return 'stopped'
    }
    return startTimedScroll() ? 'started' : 'blocked'
  }

  function viewportAtBottom(el: HTMLElement): boolean {
    return el.scrollTop + el.clientHeight >= el.scrollHeight - 2
  }

  function lineHeightOf(el: HTMLElement): number {
    const lh = Number.parseFloat(window.getComputedStyle(el).lineHeight)
    return Number.isFinite(lh) && lh > 0 ? lh : 24
  }

  let stallCount = 0

  function tick() {
    if (!active.value) return
    const el = deps.getScrollEl()
    // 容器丢失/无滚动空间：直接停，避免「按钮亮着却永远不动」的假活状态
    if (!el || el.scrollHeight <= el.clientHeight) {
      stopTimedScroll()
      return
    }
    if (viewportAtBottom(el)) {
      stopTimedScroll()
      return
    }
    const before = el.scrollTop
    const delta =
      deps.settings.value.range === 'line'
        ? lineHeightOf(el)
        : Math.max(160, el.clientHeight * 0.82)
    // 直接赋值（即时滚动，与 ColorTxt scrollByDeltaY 一致；scrollBy smooth 在部分容器内会被打断）
    el.scrollTop = before + delta
    if (el.scrollTop === before) {
      // 赋值无效（容器实际不可滚）连续 2 次即停
      stallCount += 1
      if (stallCount >= 2) stopTimedScroll()
    } else {
      stallCount = 0
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
