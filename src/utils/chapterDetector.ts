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
  const chapters: Chapter[] = []
  let lastPos = 0
  let lastTitle = ''
  let foundFirst = false

  for (const [lineStart, line] of lineStarts(text)) {
    if (!isHeading(line)) continue
    const title = line.trim()
    if (foundFirst) {
      chapters.push({
        id: 0,
        novelId: 0,
        title: lastTitle,
        content: '',
        sortOrder: chapters.length,
        startIndex: lastPos,
        createdAt: '',
      })
    } else {
      foundFirst = true
    }
    lastTitle = title
    lastPos = lineStart + line.length
  }

  if (foundFirst) {
    chapters.push({
      id: 0,
      novelId: 0,
      title: lastTitle,
      content: '',
      sortOrder: chapters.length,
      startIndex: lastPos,
      createdAt: '',
    })
  } else if (text.trim()) {
    chapters.push({
      id: 0,
      novelId: 0,
      title: '全文',
      content: '',
      sortOrder: 0,
      startIndex: 0,
      createdAt: '',
    })
  }

  return chapters
}
