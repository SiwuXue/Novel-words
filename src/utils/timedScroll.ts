/**
 * 定时自动滚动（参考 ColorTxt constants/timedScroll.ts）：
 * 按设定间隔每次向下滚动一行或一屏，到底自动停止；
 * 与朗读互斥，手动滚动后重新计满间隔。
 */
export type TimedScrollRange = 'screen' | 'line'

export interface TimedScrollSettings {
  range: TimedScrollRange
  intervalMs: number
}

export const defaultTimedScrollRange: TimedScrollRange = 'screen'
export const defaultTimedScrollIntervalMs = 3000
export const minTimedScrollIntervalMs = 200
export const maxTimedScrollIntervalMs = 600_000

/** 底部条下拉的速度档位（毫秒） */
export const TIMED_SCROLL_SPEED_PRESETS: { labelKey: string; intervalMs: number }[] = [
  { labelKey: 'reading.autoScrollSpeedVeryFast', intervalMs: 800 },
  { labelKey: 'reading.autoScrollSpeedFast', intervalMs: 2000 },
  { labelKey: 'reading.autoScrollSpeedNormal', intervalMs: 3000 },
  { labelKey: 'reading.autoScrollSpeedSlow', intervalMs: 5000 },
  { labelKey: 'reading.autoScrollSpeedVerySlow', intervalMs: 8000 },
]

export function clampTimedScrollIntervalMs(v: number): number {
  if (!Number.isFinite(v)) return defaultTimedScrollIntervalMs
  return Math.max(
    minTimedScrollIntervalMs,
    Math.min(maxTimedScrollIntervalMs, Math.floor(v)),
  )
}

export function mergeTimedScrollSettings(
  partial: Partial<TimedScrollSettings> | null | undefined,
): TimedScrollSettings {
  const range =
    partial?.range === 'line' || partial?.range === 'screen'
      ? partial.range
      : defaultTimedScrollRange
  return {
    range,
    intervalMs: clampTimedScrollIntervalMs(
      partial?.intervalMs ?? defaultTimedScrollIntervalMs,
    ),
  }
}
