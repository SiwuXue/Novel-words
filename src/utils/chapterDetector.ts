import type { Chapter } from '@/types/novel'

/**
 * Frontend port of `src-tauri/src/utils/chapter_detector.rs`.
 */
const PATTERNS = [
  '第',
  'Chapter ',
  'CHAPTER ',
  'chaper ',
  '楔子',
  '序章',
  '序言',
  '终章',
  '尾声',
  '后记',
  '番外',
  '番外篇',
  '尾声·',
  '卷',
]

function isHeading(line: string): boolean {
  const trimmed = line.trim()
  if (!trimmed) return false
  if (trimmed.length > 30) return false
  return PATTERNS.some((p) => trimmed.startsWith(p))
}

/**
 * Yield (char_offset, line_content) pairs for each line in the text.
 * Handles \n, \r\n, and standalone \r line endings.
 * Offsets are JavaScript string indices (UTF-16 code units).
 */
type Heading = { lineStart: number; title: string; contentStart: number }

function collectHeadings(text: string): Heading[] {
  const result: Heading[] = []
  let lineStart = 0
  for (let i = 0; i <= text.length; i++) {
    const ch = i < text.length ? text[i] : '\n' // treat EOF as newline
    if (ch === '\n' || ch === '\r') {
      const line = text.slice(lineStart, i)
      if (isHeading(line)) {
        result.push({
          lineStart,
          title: line.trim(),
          contentStart: lineStart + line.length,
        })
      }
      // Skip \r\n sequence
      if (ch === '\r' && i + 1 < text.length && text[i + 1] === '\n') {
        i++ // skip \n
      }
      lineStart = i + 1
    }
  }
  return result
}

function buildChapters(text: string, headings: Heading[]): Chapter[] {
  // No headings at all → whole text as one chapter.
  if (headings.length === 0) {
    if (!text.trim()) return []
    return [
      {
        id: 0,
        novelId: 0,
        title: '全文',
        content: text.trim(),
        sortOrder: 0,
        startIndex: 0,
        createdAt: '',
      },
    ]
  }

  const chapters: Chapter[] = []

  // Preamble: any content before the first heading (e.g. book title / author).
  const preamble = text.slice(0, headings[0].lineStart).trim()
  if (preamble) {
    chapters.push({
      id: 0,
      novelId: 0,
      title: '前言',
      content: preamble,
      sortOrder: 0,
      startIndex: 0,
      createdAt: '',
    })
  }

  // Each heading's content runs until the next heading's line start.
  for (let i = 0; i < headings.length; i++) {
    const h = headings[i]
    const end = i + 1 < headings.length ? headings[i + 1].lineStart : text.length
    const content = text.slice(h.contentStart, end).trim()
    chapters.push({
      id: 0,
      novelId: 0,
      title: h.title,
      content,
      sortOrder: chapters.length,
      startIndex: h.lineStart,
      createdAt: '',
    })
  }

  return chapters
}

export function detectChapters(text: string): Chapter[] {
  return buildChapters(text, collectHeadings(text))
}

/**
 * Detect chapters in batches so a large novel yields to the browser between
 * chunks. The synchronous detector is kept for small edits/autosaves.
 */
export async function detectChaptersInBatches(
  text: string,
  onProgress?: (progress: number) => void,
): Promise<Chapter[]> {
  const headings: Heading[] = []
  const batchSize = 64 * 1024
  let lineStart = 0
  let lastYieldAt = 0

  for (let i = 0; i <= text.length; i++) {
    const ch = i < text.length ? text[i] : '\n'
    if (ch !== '\n' && ch !== '\r') continue

    const line = text.slice(lineStart, i)
    if (isHeading(line)) {
      headings.push({
        lineStart,
        title: line.trim(),
        contentStart: lineStart + line.length,
      })
    }

    if (i - lastYieldAt >= batchSize && i < text.length) {
      onProgress?.(i / Math.max(1, text.length))
      lastYieldAt = i
      await new Promise<void>((resolve) => {
        if (typeof requestAnimationFrame === 'function') {
          requestAnimationFrame(() => resolve())
        } else {
          setTimeout(resolve, 0)
        }
      })
    }

    if (ch === '\r' && i + 1 < text.length && text[i + 1] === '\n') {
      i++
    }
    lineStart = i + 1
  }

  onProgress?.(1)
  return buildChapters(text, headings)
}
