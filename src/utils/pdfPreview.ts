/**
 * Frontend PDF preview renderer — intensive reading template only.
 *
 * Produces the same look as the Rust printpdf backend: per-paragraph
 * inline annotations in two-pass (Step 1/Step 2) structure.
 *
 * This module does NOT call the printer — it just emits HTML the preview
 * panel can render for sanity-check before exporting.
 */
import type { Chapter } from '@/types/novel'
import type { VocabWord } from '@/types/vocabWord'
import type { PdfTemplate } from '@/types/pdf'
import { textColorFor } from './proficiencyColors'
import { looksLikeHtml } from './editorHtml'

/* ------------------------------------------------------------------ *
 * Matching: locate English words in Chinese text via their (chosen) *
 * Chinese definition — same approach as the Rust pdf::matcher.      *
 * ------------------------------------------------------------------ */

const CJK_RANGE = /[\u{4E00}-\u{9FFF}\u{3400}-\u{4DBF}\u{F900}-\u{FAFF}]/u

function isCjk(ch: string): boolean {
  return CJK_RANGE.test(ch)
}

function extractCnTerms(definition: string): string[] {
  if (!definition) return []
  const segments = definition.split(/[;；,，、/|~～()（）【】\[\] \t\n.""'"""'']+/)
  const terms: string[] = []
  for (const seg of segments) {
    let run = ''
    for (const ch of seg) {
      if (isCjk(ch)) {
        run += ch
      } else if (run) {
        if (run.length >= 2 && !terms.includes(run)) terms.push(run)
        run = ''
      }
    }
    if (run.length >= 2 && !terms.includes(run)) terms.push(run)
  }
  return terms
}

interface Match {
  start: number
  end: number
  word: VocabWord
}

function findMatchesInLine(line: string, words: VocabWord[]): Match[] {
  const raw: Match[] = []
  for (const w of words) {
    for (const term of extractCnTerms(w.definition)) {
      let from = 0
      while (true) {
        const pos = line.indexOf(term, from)
        if (pos < 0) break
        raw.push({ start: pos, end: pos + term.length, word: w })
        from = pos + term.length
        if (from <= pos) break
      }
    }
  }
  raw.sort((a, b) => a.start - b.start || b.end - b.start - (a.end - a.start))
  const filtered: Match[] = []
  for (const m of raw) {
    if (!filtered.some((f) => m.start < f.end && f.start < m.end)) {
      filtered.push(m)
    }
  }
  return filtered
}

function wordsFoundInText(text: string, words: VocabWord[]): VocabWord[] {
  const found: VocabWord[] = []
  for (const w of words) {
    if (
      extractCnTerms(w.definition).some((t) => text.includes(t)) &&
      !found.some((f) => f.word.toLowerCase() === w.word.toLowerCase())
    ) {
      found.push(w)
    }
  }
  return found
}

/* ------------------------------------------------------------------ *
 * HTML helpers                                                       *
 * ------------------------------------------------------------------ */

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

export function splitParagraphs(content: string): string[] {
  const hasBlank = content.includes('\n\n') || content.includes('\r\n\r\n')
  const parts: string[] = hasBlank
    ? content.split('\n\n').map((p) => p.replace(/\r/g, '').replace(/\n/g, ' '))
    : content.split('\n')
  return parts.map((p) => p.trim()).filter((p) => p.length > 0)
}

/** Step 1: Chinese → English (by proficiency) + （definition）. */
function renderParagraphStep1(para: string, words: VocabWord[]): string {
  const matches = findMatchesInLine(para, words)
  if (matches.length === 0) {
    return escapeHtml(para)
  }
  const out: string[] = []
  let last = 0
  for (const m of matches) {
    if (m.start > last) out.push(escapeHtml(para.slice(last, m.start)))
    const en = escapeHtml(m.word.word)
    const def = escapeHtml(m.word.definition || '—')
    const enColor = textColorFor(m.word.proficiency)
    out.push(
      `<span class="vocab-en" style="color:${enColor}">${en}</span><span class="vocab-paren">（</span><span class="vocab-def">${def}</span><span class="vocab-paren">）</span>`,
    )
    last = m.end
  }
  if (last < para.length) out.push(escapeHtml(para.slice(last)))
  return out.join('')
}

/** Step 2: Chinese → English (by proficiency) + （blank）. */
function renderParagraphStep2(para: string, words: VocabWord[]): string {
  const matches = findMatchesInLine(para, words)
  if (matches.length === 0) {
    return escapeHtml(para)
  }
  const out: string[] = []
  let last = 0
  for (const m of matches) {
    if (m.start > last) out.push(escapeHtml(para.slice(last, m.start)))
    const en = escapeHtml(m.word.word)
    const defLen = Math.max((m.word.definition || '').length, 4)
    const blank = Array(defLen + 1).join('\u3000')
    const enColor = textColorFor(m.word.proficiency)
    out.push(
      `<span class="vocab-en" style="color:${enColor}">${en}</span><span class="vocab-blank">（${blank}）</span>`,
    )
    last = m.end
  }
  if (last < para.length) out.push(escapeHtml(para.slice(last)))
  return out.join('')
}

/** Step 3: Build a single column table (序号 / 单词 / 释义). */
function buildStep3ColumnTable(list: VocabWord[], startIdx: number): string {
  const rows = list
    .map((w, i) => {
      const idx = String(startIdx + i + 1).padStart(2, '0')
      const en = escapeHtml(w.word)
      const def = escapeHtml(w.definition || '—')
      const enColor = textColorFor(w.proficiency)
      return `<tr><td class="idx">${idx}</td><td class="word" style="color:${enColor}">${en}</td><td class="def" title="${def}">${def}</td></tr>`
    })
    .join('')
  return `<table><thead><tr><th>序号</th><th>单词</th><th>释义</th></tr></thead><tbody>${rows}</tbody></table>`
}

/** Step 3 block: title + description + two-column word table. */
function buildStep3Block(chWords: VocabWord[]): string {
  const n = chWords.length
  if (n === 0) {
    return [
      `<div class="step-title">Step 3：单词列表（本章 0 词）</div>`,
      `<div class="step-desc">本章没有匹配到词汇本中的单词。</div>`,
    ].join('')
  }
  const mid = Math.ceil(n / 2)
  const left = chWords.slice(0, mid)
  const right = chWords.slice(mid)
  return [
    `<div class="step-title">Step 3：单词列表（本章 ${n} 词）</div>`,
    `<div class="step-desc">复习本章出现的全部 ${n} 个单词，巩固记忆效果。</div>`,
    `<div class="step3-tables">`,
    buildStep3ColumnTable(left, 0),
    buildStep3ColumnTable(right, mid),
    `</div>`,
  ].join('')
}

function stripHtml(html: string): string {
  return html
    .replace(/<br\s*\/?>/gi, '\n')
    .replace(/<\/p>/gi, '\n\n')
    .replace(/<\/h[1-6]>/gi, '\n\n')
    .replace(/<\/li>/gi, '\n')
    .replace(/<\/div>/gi, '\n')
    .replace(/<[^>]*>/g, '')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}

/* ------------------------------------------------------------------ *
 * Public API                                                         *
 * ------------------------------------------------------------------ */

export interface BuildPreviewInput {
  chapters: Chapter[]
  words: VocabWord[]
  template?: PdfTemplate | null
  novelTitle?: string
}

function baseCss(fontSize: number, lineHeight: number): string {
  return `
    .pdf-preview-body { font-size: ${fontSize}px; line-height: ${lineHeight}; color: #222; }
    .pdf-preview-body .vocab-en { font-weight: 500; }
    .pdf-preview-body .vocab-def { color: #990099; }
    .pdf-preview-body .vocab-paren { color: #222; }
    .pdf-preview-body .vocab-blank { color: #999; letter-spacing: 0; }
    .pdf-preview-body .intensive-chapter { margin-bottom: 20px; page-break-after: always; }
    .pdf-preview-body .intensive-chapter .ch-en { font-size: ${fontSize + 6}px; text-align: center; margin: 20px 0 6px; font-weight: 600; }
    .pdf-preview-body .intensive-chapter .ch-cn { font-size: ${fontSize + 2}px; text-align: center; margin: 0 0 4px; font-weight: 600; }
    .pdf-preview-body .intensive-chapter .ch-sub { font-size: ${fontSize - 1}px; text-align: center; margin: 0 0 3px; color: #CC0000; }
    .pdf-preview-body .intensive-chapter .ch-wc { font-size: ${fontSize - 2}px; text-align: center; margin: 0 0 14px; color: #999; }
    .pdf-preview-body .step-title { font-size: ${fontSize + 1}px; margin: 14px 0 4px; font-weight: 600; }
    .pdf-preview-body .step-desc { font-size: ${fontSize - 2}px; margin: 0 0 10px; color: #999; }
    .pdf-preview-body .step1-end { font-size: ${fontSize - 1}px; text-align: center; margin: 14px 0 18px; color: #999; letter-spacing: 2px; }
    .pdf-preview-body .chapter-end { font-size: ${fontSize + 1}px; text-align: center; margin: 26px 0 10px; letter-spacing: 2px; }
    .pdf-preview-body .step3-tables { display: flex; justify-content: space-between; gap: 2%; margin-top: 10px; }
    .pdf-preview-body .step3-tables table { border-collapse: collapse; width: 49%; table-layout: fixed; }
    .pdf-preview-body .step3-tables th,
    .pdf-preview-body .step3-tables td { border: 1px solid #C8D1D9; padding: 4px 6px; font-size: ${fontSize - 1}px; }
    .pdf-preview-body .step3-tables thead th { background: #E0E8EF; color: #333; font-weight: 600; text-align: center; }
    .pdf-preview-body .step3-tables td.idx { text-align: center; color: #888; width: 20%; font-size: ${fontSize - 2}px; }
    .pdf-preview-body .step3-tables td.word { width: 34%; font-weight: 500; }
    .pdf-preview-body .step3-tables td.def { width: 46%; color: #222; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .pdf-preview-body p { margin: 0 0 8px; text-indent: 2em; }
  `
}

function buildIntensive(chapters: Chapter[], words: VocabWord[], _novelTitle?: string): string {
  const parts: string[] = []
  for (let ci = 0; ci < chapters.length; ci++) {
    const ch = chapters[ci]
    const num = ci + 1
    const body = looksLikeHtml(ch.content) ? stripHtml(ch.content) : ch.content
    const chWords = wordsFoundInText(body, words)

    parts.push(`<div class="intensive-chapter">`)
    parts.push(`<div class="ch-en">Chapter ${num}</div>`)
    if (ch.title) parts.push(`<div class="ch-cn">${escapeHtml(ch.title)}</div>`)
    parts.push(`<div class="ch-sub">【第 ${num} 章】</div>`)
    parts.push(`<div class="ch-wc">本章词汇：${chWords.length} 词</div>`)

    parts.push(`<div class="step-title">Step 1：在语境中背单词</div>`)
    parts.push(`<div class="step-desc">请仔细阅读下文，注意英文单词及其对应的中文释义。红色=生疏，橙色=熟悉，灰色=已掌握。</div>`)
    for (const para of splitParagraphs(body)) {
      parts.push(`<p>${renderParagraphStep1(para, words)}</p>`)
    }
    parts.push(`<div class="step1-end">—— Step 1 完 ——</div>`)

    parts.push(`<div class="step-title">Step 2：看单词回忆词义</div>`)
    parts.push(`<div class="step-desc">请再次阅读下文，尝试回忆英文单词对应的中文意思。</div>`)
    for (const para of splitParagraphs(body)) {
      parts.push(`<p>${renderParagraphStep2(para, words)}</p>`)
    }

    parts.push(buildStep3Block(chWords))
    parts.push(`<div class="chapter-end">—— 第 ${num} 章 完 ——</div>`)
    parts.push(`</div>`)
  }
  return parts.join('\n')
}

export function buildHtml(input: BuildPreviewInput): string {
  const { chapters, words, template, novelTitle } = input
  const lineHeight = template?.lineSpacing ?? 1.5
  const fontSize = template?.fontSize ?? 14
  const css = baseCss(fontSize, lineHeight)
  const bodyContent = buildIntensive(chapters, words, novelTitle)
  return `<style>${css}</style><div class="pdf-preview-body">${bodyContent}</div>`
}
