/**
 * Frontend PDF preview renderer.
 *
 * Produces the same look as the Rust printpdf backend (which is the real
 * exporter): per-paragraph inline annotations + per-chapter word table +
 * full-book word table, with each vocab word colored by proficiency.
 *
 * This module does NOT call the printer — it just emits HTML the preview
 * panel can render and that the user can sanity-check before clicking
 * "Export".
 */
import type { Chapter } from '@/types/novel'
import type { VocabWord } from '@/types/vocabWord'
import type { PdfTemplate } from '@/types/pdf'
import { highlightFor } from './proficiencyColors'
import { looksLikeHtml } from './editorHtml'

/* ------------------------------------------------------------------ *
 * Matching: locate English words in Chinese text via their (chosen) *
 * Chinese definition — same approach as the Rust pdf::matcher.      *
 * ------------------------------------------------------------------ */

const CJK_RANGE = /[\u{4E00}-\u{9FFF}\u{3400}-\u{4DBF}\u{F900}-\u{FAFF}]/u

function isCjk(ch: string): boolean {
  return CJK_RANGE.test(ch)
}

/** Extract candidate Chinese terms from a definition. */
function extractCnTerms(definition: string): string[] {
  if (!definition) return []
  const segments = definition.split(/[;；,，、/|~～()（）【】\[\] \t\n.""'“”]+/)
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

/** Find all non-overlapping occurrences of vocab words in `line`. */
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

/** Return the subset of words whose Chinese meaning appears in `text`. */
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

/** Split a chapter's body into paragraphs, matching the backend behavior. */
export function splitParagraphs(content: string): string[] {
  const hasBlank = content.includes('\n\n') || content.includes('\r\n\r\n')
  const parts: string[] = hasBlank
    ? content.split('\n\n').map((p) => p.replace(/\r/g, '').replace(/\n/g, ' '))
    : content.split('\n')
  return parts.map((p) => p.trim()).filter((p) => p.length > 0)
}

function renderParagraph(para: string, words: VocabWord[]): string {
  const matches = findMatchesInLine(para, words)
  if (matches.length === 0) {
    return escapeHtml(para)
  }
  const out: string[] = []
  let last = 0
  for (const m of matches) {
    if (m.start > last) out.push(escapeHtml(para.slice(last, m.start)))
    const matched = para.slice(m.start, m.end)
    const c = highlightFor(m.word.proficiency)
    const sup = m.word.phonetic
      ? `${escapeHtml(m.word.phonetic)} ${escapeHtml(m.word.definition || '')}`
      : escapeHtml(m.word.definition || '')
    out.push(
      `<span class="vocab-word" data-prof="${m.word.proficiency}" style="background:${c.bg};color:${c.text};border-radius:3px;padding:0 3px;font-weight:500;">${escapeHtml(matched)}<sup style="font-size:0.65em;margin-left:2px;color:${c.text};">${sup}</sup></span>`,
    )
    last = m.end
  }
  if (last < para.length) out.push(escapeHtml(para.slice(last)))
  return out.join('')
}

function renderChapterWordTable(chapterTitle: string, words: VocabWord[]): string {
  if (words.length === 0) return ''
  const rows = words
    .map((w) => {
      const c = highlightFor(w.proficiency)
      return `<tr>
        <td><span class="vocab-word" data-prof="${w.proficiency}" style="background:${c.bg};color:${c.text};border-radius:3px;padding:1px 4px;">${escapeHtml(w.word)}</span></td>
        <td>${escapeHtml(w.phonetic || '—')}</td>
        <td>${escapeHtml(w.definition || '—')}</td>
      </tr>`
    })
    .join('')
  return `<h3 class="vocab-heading">${escapeHtml(chapterTitle)} — 生词表</h3>
    <table class="vocab-table">
      <thead><tr><th>单词</th><th>音标</th><th>释义</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>`
}

function renderFullTable(words: VocabWord[]): string {
  if (words.length === 0) return ''
  const seen = new Set<string>()
  const unique = words.filter((w) => {
    const k = w.word.toLowerCase()
    if (seen.has(k)) return false
    seen.add(k)
    return true
  })
  const rows = unique
    .map((w) => {
      const c = highlightFor(w.proficiency)
      return `<tr>
        <td><span class="vocab-word" data-prof="${w.proficiency}" style="background:${c.bg};color:${c.text};border-radius:3px;padding:1px 4px;">${escapeHtml(w.word)}</span></td>
        <td>${escapeHtml(w.phonetic || '—')}</td>
        <td>${escapeHtml(w.definition || '—')}</td>
      </tr>`
    })
    .join('')
  return `<h3 class="vocab-heading">全文总词汇表</h3>
    <table class="vocab-table">
      <thead><tr><th>单词</th><th>音标</th><th>释义</th></tr></thead>
      <tbody>${rows}</tbody>
    </table>`
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
  templateType?: string
}

function baseCss(fontSize: number, lineHeight: number): string {
  return `
    .pdf-preview-body { font-size: ${fontSize}px; line-height: ${lineHeight}; color: #222; }
    .pdf-preview-body .vocab-word { display: inline-block; }
    .pdf-preview-body h1.title { font-size: ${fontSize + 8}px; text-align: center; margin: 32px 0 8px; }
    .pdf-preview-body h2.chapter { font-size: ${fontSize + 4}px; margin: 24px 0 8px; padding-bottom: 4px; border-bottom: 1px solid #ddd; }
    .pdf-preview-body p { margin: 0 0 8px; text-indent: 2em; }
    .pdf-preview-body .vocab-heading { font-size: ${fontSize + 2}px; margin: 18px 0 6px; }
    .pdf-preview-body .vocab-table { width: 100%; border-collapse: collapse; font-size: ${fontSize - 1}px; margin: 0 0 16px; }
    .pdf-preview-body .vocab-table th, .pdf-preview-body .vocab-table td { padding: 4px 8px; border: 1px solid #ddd; text-align: left; vertical-align: top; }
    .pdf-preview-body .vocab-table th { background: #f5f5f5; }
    .pdf-preview-body .sidebar-row { display: flex; gap: 12px; margin-bottom: 12px; }
    .pdf-preview-body .sidebar-body { flex: 0 0 65%; }
    .pdf-preview-body .sidebar-words { flex: 1; font-size: ${fontSize - 2}px; background: #f9f9f9; padding: 6px 8px; border-radius: 4px; }
    .pdf-preview-body .sidebar-words .sw { margin-bottom: 3px; }
    .pdf-preview-body .dict-blank { display: inline-block; border-bottom: 1px solid #000; min-width: 2em; margin: 0 2px; }
    .pdf-preview-body .dict-answer { color: #888; font-size: ${fontSize - 2}px; }
  `
}

/** Intensive: inline annotations + word tables (current behavior). */
function buildIntensive(
  chapters: Chapter[], words: VocabWord[], novelTitle?: string,
): string {
  const parts: string[] = []
  if (novelTitle) parts.push(`<h1 class="title">${escapeHtml(novelTitle)}</h1>`)
  for (const ch of chapters) {
    if (ch.title) parts.push(`<h2 class="chapter">${escapeHtml(ch.title)}</h2>`)
    const body = looksLikeHtml(ch.content) ? stripHtml(ch.content) : ch.content
    for (const para of splitParagraphs(body)) {
      parts.push(`<p>${renderParagraph(para, words)}</p>`)
    }
    const chWords = wordsFoundInText(body, words)
    parts.push(renderChapterWordTable(ch.title, chWords))
  }
  parts.push(renderFullTable(words))
  return parts.join('\n')
}

/** Sidebar: left ~65% body, right word list per paragraph. */
function buildSidebar(
  chapters: Chapter[], words: VocabWord[], _novelTitle?: string,
): string {
  const parts: string[] = []
  for (const ch of chapters) {
    if (ch.title) parts.push(`<h2 class="chapter">${escapeHtml(ch.title)}</h2>`)
    const body = looksLikeHtml(ch.content) ? stripHtml(ch.content) : ch.content
    for (const para of splitParagraphs(body)) {
      const found = wordsFoundInText(para, words)
      const wordList = found.length > 0
        ? found.map((w) => {
            const c = highlightFor(w.proficiency)
            return `<div class="sw"><span class="vocab-word" style="background:${c.bg};color:${c.text};border-radius:3px;padding:0 3px;">${escapeHtml(w.word)}</span> ${escapeHtml(w.definition || '')}</div>`
          }).join('')
        : '<span style="color:#ccc">—</span>'
      parts.push(
        `<div class="sidebar-row"><div class="sidebar-body"><p>${escapeHtml(para)}</p></div><div class="sidebar-words">${wordList}</div></div>`,
      )
    }
  }
  parts.push(renderFullTable(words))
  return parts.join('\n')
}

/** Recitation: left body, right word+definition per paragraph. */
function buildRecitation(
  chapters: Chapter[], words: VocabWord[], _novelTitle?: string,
): string {
  const parts: string[] = []
  for (const ch of chapters) {
    if (ch.title) parts.push(`<h2 class="chapter">${escapeHtml(ch.title)}</h2>`)
    const body = looksLikeHtml(ch.content) ? stripHtml(ch.content) : ch.content
    for (const para of splitParagraphs(body)) {
      const found = wordsFoundInText(para, words)
      const wordList = found.length > 0
        ? found.map((w) => {
            const c = highlightFor(w.proficiency)
            return `<div class="sw"><span class="vocab-word" style="background:${c.bg};color:${c.text};border-radius:3px;padding:0 3px;">${escapeHtml(w.word)} ${escapeHtml(w.definition || '')}</span></div>`
          }).join('')
        : '<span style="color:#ccc">—</span>'
      parts.push(
        `<div class="sidebar-row"><div class="sidebar-body"><p>${escapeHtml(para)}</p></div><div class="sidebar-words">${wordList}</div></div>`,
      )
    }
  }
  parts.push(renderFullTable(words))
  return parts.join('\n')
}

/** Dictation: matched terms → blanks; answer key at end. */
function buildDictation(
  chapters: Chapter[], words: VocabWord[], _novelTitle?: string,
): string {
  const parts: string[] = []
  for (const ch of chapters) {
    if (ch.title) parts.push(`<h2 class="chapter">${escapeHtml(ch.title)}</h2>`)
    const body = looksLikeHtml(ch.content) ? stripHtml(ch.content) : ch.content
    for (let para of splitParagraphs(body)) {
      const matches = findMatchesInLine(para, words)
      if (matches.length > 0) {
        let out = ''
        let last = 0
        for (const m of matches) {
          if (m.start > last) out += escapeHtml(para.slice(last, m.start))
          const blankLen = Math.max(m.end - m.start, 2) * 2
          out += `<span class="dict-blank" style="width:${blankLen}ch;">${'_'.repeat(blankLen)}</span>`
          last = m.end
        }
        if (last < para.length) out += escapeHtml(para.slice(last))
        para = out
      } else {
        para = escapeHtml(para)
      }
      parts.push(`<p>${para}</p>`)
    }
  }
  // Answer key
  const seen = new Set<string>()
  const unique = words.filter((w) => {
    const k = w.word.toLowerCase()
    if (seen.has(k)) return false
    seen.add(k)
    return true
  })
  if (unique.length > 0) {
    const answers = unique
      .map((w) => {
        const c = highlightFor(w.proficiency)
        return `<span class="dict-answer"><span class="vocab-word" style="background:${c.bg};color:${c.text};border-radius:3px;padding:0 3px;">${escapeHtml(w.word)}</span> ${
          w.phonetic ? `/${escapeHtml(w.phonetic)}/ ` : ''
        }${escapeHtml(w.definition || '—')}</span>`
      })
      .join(' · ')
    parts.push(
      `<h3 class="vocab-heading">参考答案</h3><p class="dict-answer" style="line-height:1.8;">${answers}</p>`,
    )
  }
  return parts.join('\n')
}

export function buildHtml(input: BuildPreviewInput): string {
  const { chapters, words, template, novelTitle, templateType } = input
  const lineHeight = template?.lineSpacing ?? 1.5
  const fontSize = template?.fontSize ?? 14
  const css = baseCss(fontSize, lineHeight)

  const bodyContent = (() => {
    switch (templateType) {
      case 'sidebar':
        return buildSidebar(chapters, words, novelTitle)
      case 'recitation':
        return buildRecitation(chapters, words, novelTitle)
      case 'dictation':
        return buildDictation(chapters, words, novelTitle)
      default:
        return buildIntensive(chapters, words, novelTitle)
    }
  })()

  return `<style>${css}</style><div class="pdf-preview-body">${bodyContent}</div>`
}
