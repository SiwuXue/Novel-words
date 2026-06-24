/**
 * Front-end text cleaner — TypeScript port of src-tauri/src/utils/text_cleaner.rs.
 *
 * Three-phase cleaning: remove ad lines → normalize whitespace → strip special chars.
 * Keep the two files in sync: when a new ad pattern is added, add it to both.
 */

type AdPattern = string

const AD_PATTERNS: AdPattern[] = [
  '请收藏',
  '本章未完',
  '求推荐',
  '求月票',
  '求订阅',
  '求打赏',
  '求收藏',
  '本章完',
  'www.',
  'http://',
  'https://',
  '.com',
  '笔趣阁',
  '顶点小说',
  '请记住',
  '永久免费',
  '最快更新',
  '手机阅读',
  '电脑阅读',
  '一秒记住',
  '记住本站',
  '域名',
  '首发',
  '唯一网址',
  '本站网址',
  '备用网址',
  '防失联',
  '网址',
  '新域名',
  '老域名',
  'org/',
  '.net',
]

/**
 * Remove lines that contain known ad/watermark patterns.
 * Empty lines are kept (they're handled by normalizeWhitespace).
 */
function removeAdLines(text: string): string {
  return text
    .split('\n')
    .filter((line) => {
      const trimmed = line.trim()
      if (trimmed === '') return true
      return !AD_PATTERNS.some((pat) => trimmed.includes(pat))
    })
    .join('\n')
}

/**
 * Collapse 3+ consecutive blank lines into at most 2.
 */
function normalizeWhitespace(text: string): string {
  const lines = text.split('\n')
  const result: string[] = []
  let emptyCount = 0

  for (const line of lines) {
    if (line.trim() === '') {
      emptyCount++
      if (emptyCount <= 2) {
        result.push('')
      }
    } else {
      emptyCount = 0
      result.push(line)
    }
  }

  return result.join('\n')
}

/**
 * Strip zero-width characters, BOM, soft hyphens, and control chars.
 * Keeps: newline (\n), carriage return (\r), tab (\t),
 * ASCII printable (0x20-0x7E), and all Unicode ≥ 0x80
 * (CJK, punctuation, fullwidth forms).
 */
function stripSpecialChars(text: string): string {
  // Remove: C0 control chars (0x00-0x1F except \n \r \t),
  //         zero-width chars, BOM, soft hyphen, DEL (0x7F).
  return text.replace(
    /[\x00-\x08\x0B\x0C\x0E-\x1F\x7F​‌‍﻿­]/g,
    '',
  )
}

/**
 * Clean raw text: remove ads → normalize whitespace → strip special chars.
 */
export function cleanText(raw: string): string {
  let cleaned = removeAdLines(raw)
  cleaned = normalizeWhitespace(cleaned)
  cleaned = stripSpecialChars(cleaned)
  return cleaned
}
