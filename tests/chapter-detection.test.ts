import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { detectChapters, detectChaptersViaToc } from '@/utils/chapterDetector'

const SAMPLE =
  'Flipped\n\nby Wendelin Van Draanen\n\nCONTENTS\n\nDiving Under\n\nFlipped\n\nBuddy, Beware!\n\nDiving Under\n\nAll I\'ve ever wanted is for Juli Baker to leave me alone.\n\nFlipped\n\nThe first day I met Juli Baker, I lost my mind.\n\nBuddy, Beware!\n\nThe third chapter body.\n'

describe('toc-driven chapter detection', () => {
  it('splits named chapters from a CONTENTS listing', () => {
    const chapters = detectChapters(SAMPLE)
    expect(chapters.map((c) => c.title)).toEqual([
      '前言',
      'Diving Under',
      'Flipped',
      'Buddy, Beware!',
    ])
    expect(chapters[1].content).toContain('Juli Baker to leave me alone')
    expect(chapters[3].content).toBe('The third chapter body.')
    expect(chapters[0].content).toContain('CONTENTS')
  })

  it('falls back to pattern detection without a toc marker', () => {
    const chapters = detectChapters('序章\nprologue body\n第一章 开端\nchapter body\n')
    expect(chapters.map((c) => c.title)).toEqual(['序章', '第一章 开端'])
  })

  it('keeps the whole text as one chapter when nothing matches', () => {
    const chapters = detectChapters('just some text without chapters')
    expect(chapters.map((c) => c.title)).toEqual(['全文'])
  })

  it('returns null from the toc strategy when validation fails', () => {
    // 目录只有一条 + 正文首条目从未再次出现
    expect(detectChaptersViaToc('CONTENTS\n\nDiving Under\n\nsome body without the title\n')).toBeNull()
  })
})

// 用真实文件做端到端验证（文件在本机时）
const REAL_FILE = 'E:/Dsektop/Flipped (怦然心动).txt'
const itReal = existsSync(REAL_FILE) ? it : it.skip

describe('real Flipped txt file', () => {
  itReal('detects the toc and all named chapters', () => {
    const text = readFileSync(REAL_FILE, 'utf-8')
    const chapters = detectChapters(text)
    const titles = chapters.map((c) => c.title)
    // 目录含 14 个条目 → 前言 + 14 章
    expect(titles[0]).toBe('前言')
    expect(chapters.length).toBe(15)
    expect(titles).toContain('Diving Under')
    expect(titles).toContain('The Basket Boys')
    expect(titles[titles.length - 1]).toBe('The Basket Boys')
    // 每章都有正文
    for (const chapter of chapters.slice(1)) {
      expect(chapter.content.length).toBeGreaterThan(0)
    }
  })
})
