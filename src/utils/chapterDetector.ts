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

/** Yield (byte_offset, line_content) pairs for each line in the text. */
function lineStarts(text: string): Array<[number, string]> {
  const result: Array<[number, string]> = []
  const lines = text.split(/\r?\n/)
  let pos = 0
  for (const line of lines) {
    while (pos < text.length && (text[pos] === '\n' || text[pos] === '\r')) {
      pos++
    }
    result.push([pos, line])
    pos += line.length
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
