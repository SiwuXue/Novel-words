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

/** 目录驱动识别：适配无 "Chapter N" 前缀、以 CONTENTS/目录 + 标题行分章的英文小说
 *  （如 Flipped）。目录首条目在目录块之后再次出现即为正文起点，按目录顺序切章；
 *  任一候选未按序命中则返回 null（回退到前缀模式匹配）。 */
export function detectChaptersViaToc(text: string): Chapter[] | null {
  // 按行切分（记录起始偏移，兼容 \n / \r\n / \r）
  const lines: Array<{ start: number; text: string }> = []
  let lineStart = 0
  for (let i = 0; i <= text.length; i++) {
    const ch = i < text.length ? text[i] : '\n'
    if (ch === '\n' || ch === '\r') {
      lines.push({ start: lineStart, text: text.slice(lineStart, i) })
      if (ch === '\r' && text[i + 1] === '\n') i++
      lineStart = i + 1
    }
  }

  // 1. 目录标记行
  const markerIdx = lines.findIndex((l) => {
    const t = l.text.trim()
    return t.toLowerCase() === 'contents' || t === '目录'
  })
  if (markerIdx === -1) return null

  // 2. 收集目录条目；首条目重复出现 → 正文起点
  const candidates: string[] = []
  let bodyStartIdx = -1
  let i = markerIdx + 1
  while (i < lines.length) {
    const trimmed = lines[i].text.trim()
    if (!trimmed) {
      i++
      continue
    }
    if (candidates.length >= 500 || isHeading(trimmed) || trimmed.length > 60) break
    if (candidates.length > 0 && trimmed === candidates[0]) {
      bodyStartIdx = i
      break
    }
    candidates.push(trimmed)
    i++
  }
  if (candidates.length < 2 || bodyStartIdx === -1) return null

  // 3. 从正文起点按目录顺序切章
  const chapters: Chapter[] = []
  const preamble = text.slice(0, lines[bodyStartIdx].start).trim()
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
  let expected = 0
  let lastTitle = ''
  let lastPos = 0
  let inBody = false
  for (let j = bodyStartIdx; j < lines.length; j++) {
    const trimmed = lines[j].text.trim()
    if (expected < candidates.length && trimmed === candidates[expected]) {
      if (inBody) {
        chapters.push({
          id: 0,
          novelId: 0,
          title: lastTitle,
          content: text.slice(lastPos, lines[j].start).trim(),
          sortOrder: chapters.length,
          startIndex: lastPos,
          createdAt: '',
        })
      }
      lastTitle = trimmed
      lastPos = lines[j].start + lines[j].text.length
      inBody = true
      expected++
    }
  }
  if (expected !== candidates.length || !inBody) return null
  chapters.push({
    id: 0,
    novelId: 0,
    title: lastTitle,
    content: text.slice(lastPos).trim(),
    sortOrder: chapters.length,
    startIndex: lastPos,
    createdAt: '',
  })
  return chapters
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
  const toc = detectChaptersViaToc(text)
  if (toc) return toc
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
  // 目录驱动策略需要全文视野，命中即免分批扫描
  const toc = detectChaptersViaToc(text)
  if (toc) {
    onProgress?.(1)
    return toc
  }
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
