/**
 * SRS (spaced repetition) state stored inside the `memory_tag` column, so no
 * schema change is required. Keep the JSON shape in sync with
 * `src-tauri/src/utils/srs.rs`.
 */

export interface SrsState {
  ease: number
  interval: number // days
  reps: number
  lapses: number
  due: string // YYYY-MM-DD (local)
}

export interface MemoryTag {
  tag: string
  srs: SrsState | null
}

/** Parse the `memory_tag` column into a user tag + SRS state. */
export function parseMemoryTag(raw: string): MemoryTag {
  if (!raw) return { tag: '', srs: null }
  try {
    const obj = JSON.parse(raw)
    if (obj && typeof obj === 'object' && 'srs' in obj) {
      return {
        tag: typeof obj.tag === 'string' ? obj.tag : '',
        srs: (obj.srs as SrsState) ?? null,
      }
    }
  } catch {
    /* legacy plain-text tag */
  }
  return { tag: raw, srs: null }
}

/** Serialize (tag, srs) back into the memory_tag column. */
export function encodeMemoryTag(tag: string, srs: SrsState | null): string {
  if (!srs) return tag || ''
  return JSON.stringify({ tag: tag || '', srs })
}

/** Local date as YYYY-MM-DD. */
export function todayStr(): string {
  const d = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

/** A card is due when it has no SRS state or its due date is today/past. */
export function isDue(srs: SrsState | null): boolean {
  if (!srs || !srs.due) return true
  return srs.due <= todayStr()
}
