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
function lineStarts(text: string): Array<[number, string]> {
  const result: Array<[number, string]> = []
  let lineStart = 0
  for (let i = 0; i <= text.length; i++) {
    const ch = i < text.length ? text[i] : '\n' // treat EOF as newline
    if (ch === '\n' || ch === '\r') {
      const line = text.slice(lineStart, i)
      result.push([lineStart, line])
      // Skip \r\n sequence
      if (ch === '\r' && i + 1 < text.length && text[i + 1] === '\n') {
        i++ // skip \n
      }
      lineStart = i + 1
    }
  }
  return result
}

export function detectChapters(text: string): Chapter[] {
  // Collect all heading positions first: (charOffsetOfLineStart, title, contentStart)
  const headings: Array<{ lineStart: number; title: string; contentStart: number }> = []
  for (const [lineStart, line] of lineStarts(text)) {
    if (!isHeading(line)) continue
    headings.push({
      lineStart,
      title: line.trim(),
      contentStart: lineStart + line.length,
    })
  }

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
