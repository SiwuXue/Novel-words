/**
 * Single source of truth for proficiency highlight colors, shared between:
 * - the on-page PDF preview (CSS classes, here in TS)
 * - the Rust printpdf backend (Rgb 0-1 in pdf::mod::highlight_color_for)
 *
 * Whenever you change a color here, mirror the change in
 * `src-tauri/src/pdf/mod.rs::highlight_color_for`.
 */
export type Proficiency = 'unknown' | 'familiar' | 'mastered'

export interface ProficiencyColor {
  /** Background color for the word's highlight band. */
  bg: string
  /** Foreground color for the word's text. */
  text: string
}

export const PROFICIENCY_HIGHLIGHT: Record<Proficiency, ProficiencyColor> = {
  unknown: { bg: '#f9c7c7', text: '#222222' }, // 浅红
  familiar: { bg: '#fceba6', text: '#222222' }, // 浅黄
  mastered: { bg: '#c7ebc7', text: '#222222' }, // 浅绿
}

/** Text color for intensive reading mode (inline annotations). */
export const PROFICIENCY_TEXT: Record<Proficiency, string> = {
  unknown: '#CC0000', // 红色 — 生疏，需重点学习
  familiar: '#E67E22', // 橙色 — 熟悉，次重点
  mastered: '#666666', // 灰色 — 已掌握，可略过
}

/** Return the highlight color for an arbitrary proficiency string. */
export function highlightFor(proficiency: string | undefined): ProficiencyColor {
  if (proficiency === 'mastered' || proficiency === 'familiar') {
    return PROFICIENCY_HIGHLIGHT[proficiency]
  }
  return PROFICIENCY_HIGHLIGHT.unknown
}

/** Return the text color for intensive reading annotations. */
export function textColorFor(proficiency: string | undefined): string {
  if (proficiency === 'mastered' || proficiency === 'familiar') {
    return PROFICIENCY_TEXT[proficiency]
  }
  return PROFICIENCY_TEXT.unknown
}
