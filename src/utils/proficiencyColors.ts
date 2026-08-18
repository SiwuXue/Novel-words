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
  unknown:  { bg: '#f9c7c7', text: '#222222' }, // 浅红
  familiar: { bg: '#fceba6', text: '#222222' }, // 浅黄
  mastered: { bg: '#c7ebc7', text: '#222222' }, // 浅绿
}

/** Return the color for an arbitrary proficiency string, defaulting to unknown. */
export function highlightFor(proficiency: string | undefined): ProficiencyColor {
  if (proficiency === 'mastered' || proficiency === 'familiar') {
    return PROFICIENCY_HIGHLIGHT[proficiency]
  }
  return PROFICIENCY_HIGHLIGHT.unknown
}
