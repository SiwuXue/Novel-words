/**
 * Proficiency text colors for intensive reading mode.
 *
 * Keep in sync with `src-tauri/src/pdf/mod.rs::text_color_for_proficiency`.
 */
export type Proficiency = 'unknown' | 'familiar' | 'mastered'

/** Text color for intensive reading inline annotations. */
export const PROFICIENCY_TEXT: Record<Proficiency, string> = {
  unknown: '#CC0000',
  familiar: '#E67E22',
  mastered: '#666666',
}

/** Return the text color for intensive reading annotations. */
export function textColorFor(proficiency: string | undefined): string {
  if (proficiency === 'mastered' || proficiency === 'familiar') {
    return PROFICIENCY_TEXT[proficiency]
  }
  return PROFICIENCY_TEXT.unknown
}
