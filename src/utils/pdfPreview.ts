/**
 * Frontend PDF preview renderer.
 *
 * Produces the same look as the Rust printpdf backend (which is the real
 * exporter): per-paragraph inline annotations in two-pass (Step 1/Step 2)
 * structure for the intensive template.
 *
 * This module does NOT call the printer — it just emits HTML the preview
 * panel can render and that the user can sanity-check before clicking
 * "Export".
 */
import type { Chapter } from '@/types/novel'
import type { VocabWord } from '@/types/vocabWord'
import type { PdfTemplate } from '@/types/pdf'
import { highlightFor, textColorFor } from './proficiencyColors'
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

/**
 * Step 1 paragraph: matched Chinese → English (by proficiency) + （definition purple）.
 * The original Chinese term is dropped and replaced.
 */
function renderParagraphStep1(para: string, words: VocabWord[]): string {
  const matches = findMatchesInLine(para, words)
  if (matches.length === 0) {
    return escapeHtml(para)
  }
  const out: string[] = []
  let last = 0
  for (const m of matches) {
    if (m.start > last) out.push(escapeHtml(para.slice(last, m.start)))
    // Skip the original matched Chinese term (m.start..m.end). Replace it:
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

/**
 * Step 2 paragraph: matched Chinese → English (by proficiency) + （blank bracket）.
 */
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
    const blank = Array(defLen + 1).join('\u3000') // ideographic space
    const enColor = textColorFor(m.word.proficiency)
    out.push(
      `<span class="vocab-en" style="color:${enColor}">${en}</span><span class="vocab-blank">（${blank}）</span>`,
    )
    last = m.end
  }
  if (last < para.length) out.push(escapeHtml(para.slice(last)))
  return out.join('')
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
    .pdf-preview-body .vocab-en { color: #CC0000; font-weight: 500; }
    .pdf-preview-body .vocab-def { color: #990099; }
    .pdf-preview-body .vocab-paren { color: #222; }
    .pdf-preview-body .vocab-blank { color: #999; letter-spacing: 0; }
    .pdf-preview-body h1.title { font-size: ${fontSize + 8}px; text-align: center; margin: 32px 0 8px; }
    .pdf-preview-body h2.chapter { font-size: ${fontSize + 4}px; margin: 24px 0 8px; padding-bottom: 4px; border-bottom: 1px solid #ddd; }
    .pdf-preview-body .intensive-chapter { margin-bottom: 20px; page-break-after: always; }
    .pdf-preview-body .intensive-chapter .ch-en { font-size: ${fontSize + 6}px; text-align: center; margin: 20px 0 6px; font-weight: 600; }
    .pdf-preview-body .intensive-chapter .ch-cn { font-size: ${fontSize + 2}px; text-align: center; margin: 0 0 4px; font-weight: 600; }
    .pdf-preview-body .intensive-chapter .ch-sub { font-size: ${fontSize - 1}px; text-align: center; margin: 0 0 3px; color: #CC0000; }
    .pdf-preview-body .intensive-chapter .ch-wc { font-size: ${fontSize - 2}px; text-align: center; margin: 0 0 14px; color: #999; }
    .pdf-preview-body .step-title { font-size: ${fontSize + 1}px; margin: 14px 0 4px; font-weight: 600; }
    .pdf-preview-body .step-desc { font-size: ${fontSize - 2}px; margin: 0 0 10px; color: #999; }
    .pdf-preview-body .step1-end { font-size: ${fontSize - 1}px; text-align: center; margin: 14px 0 18px; color: #999; letter-spacing: 2px; }
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

/** Intensive: two-pass (Step 1 / Step 2) chapter rendering without word tables. */
function buildIntensive(
  chapters: Chapter[], words: VocabWord[], _novelTitle?: string,
): string {
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

    // Step 1
    parts.push(`<div class="step-title">Step 1：在语境中背单词</div>`)
    parts.push(`<div class="step-desc">请仔细阅读下文，注意英文单词及其对应的中文释义。红色=生疏，橙色=熟悉，灰色=已掌握。</div>`)
    for (const para of splitParagraphs(body)) {
      parts.push(`<p>${renderParagraphStep1(para, words)}</p>`)
    }
    parts.push(`<div class="step1-end">—— Step 1 完 ——</div>`)

    // Step 2
    parts.push(`<div class="step-title">Step 2：看单词回忆词义</div>`)
    parts.push(`<div class="step-desc">请再次阅读下文，尝试回忆英文单词对应的中文意思。</div>`)
    for (const para of splitParagraphs(body)) {
      parts.push(`<p>${renderParagraphStep2(para, words)}</p>`)
    }
    parts.push(`</div>`)
  }
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
        return `<span class="dict-answer"><span class="vocab-word" style="background:${c.bg};color:${c.text};border-radius:3px;padding:0 3px;">${escapeHtml(w.word)}</span> ${w.phonetic ? `/${escapeHtml(w.phonetic)}/ ` : ''
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
