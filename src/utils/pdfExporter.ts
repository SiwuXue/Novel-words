import html2pdf from 'html2pdf.js'
import type { Novel } from '@/types/novel'
import type { PdfTemplate } from '@/types/pdf'
import type { VocabWord } from '@/types/vocabWord'

interface Margins {
  top: number
  bottom: number
  left: number
  right: number
}

function parseMargins(raw: string): Margins {
  try {
    const parsed = JSON.parse(raw)
    return {
      top: parsed.top ?? 25,
      bottom: parsed.bottom ?? 25,
      left: parsed.left ?? 20,
      right: parsed.right ?? 20,
    }
  } catch {
    return { top: 25, bottom: 25, left: 20, right: 20 }
  }
}

function formatDate(dateStr: string): string {
  if (!dateStr) return ''
  // "2026-06-24T12:00:00" → "2026-06-24"
  return dateStr.slice(0, 10)
}

/**
 * Escape HTML special characters in text content.
 */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

/**
 * Build inline-annotated HTML body.
 * Wraps each vocab word occurrence in a <span> with definition shown in smaller text.
 */
function buildInlineBody(text: string, vocabs: VocabWord[], lineHeight: number): string {
  // Sort by word length descending so longer matches take priority
  const sorted = [...vocabs].sort((a, b) => b.word.length - a.word.length)
  const wordMap = new Map(sorted.map((v) => [v.word.toLowerCase(), v]))

  // Split text into paragraphs
  const paragraphs = text.split(/\n\s*\n/).filter((p) => p.trim())

  return paragraphs
    .map((para) => {
      const escaped = escapeHtml(para.trim())
      // Replace newlines with <br>
      const lines = escaped
        .split('\n')
        .map((line) => annotateLine(line, wordMap))
        .join('<br>')
      return `<p style="text-indent:2em;margin:0 0 0.5em 0;line-height:${lineHeight};">${lines}</p>`
    })
    .join('\n')
}

/**
 * Find and annotate vocab words in a single line of text.
 */
function annotateLine(
  line: string,
  wordMap: Map<string, VocabWord>,
): string {
  // Build a regex from all vocabulary words (case-insensitive)
  const escaped = [...wordMap.keys()].map((w) =>
    w.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'),
  )
  if (escaped.length === 0) return line

  const regex = new RegExp(`\\b(${escaped.join('|')})\\b`, 'gi')

  return line.replace(regex, (match) => {
    const vocab = wordMap.get(match.toLowerCase())
    if (!vocab) return match

    const def = escapeHtml(vocab.definition || '')
    const phonetic = escapeHtml(vocab.phonetic || '')
    const defText = phonetic ? `/${phonetic}/ ${def}` : def

    const color =
      vocab.proficiency === 'mastered'
        ? '#67c23a'
        : vocab.proficiency === 'familiar'
          ? '#e6a23c'
          : '#909399'

    return `<span style="color:${color};font-weight:500;" title="${defText}">${match}<sup style="font-size:0.7em;color:${color};">${defText}</sup></span>`
  })
}

/**
 * Build appendix-annotated HTML body.
 * Clean text + vocab table appended at the end.
 */
function buildAppendixBody(text: string, vocabs: VocabWord[]): string {
  const paragraphs = text
    .split(/\n\s*\n/)
    .filter((p) => p.trim())
    .map((para) => {
      const escaped = escapeHtml(para.trim())
      const lines = escaped.split('\n').join('<br>')
      return `<p style="text-indent:2em;margin:0 0 0.5em 0;">${lines}</p>`
    })
    .join('\n')

  // Sort vocabs alphabetically for appendix
  const sorted = [...vocabs].sort((a, b) =>
    a.word.toLowerCase().localeCompare(b.word.toLowerCase()),
  )

  // Deduplicate
  const seen = new Set<string>()
  const unique = sorted.filter((v) => {
    const key = v.word.toLowerCase()
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })

  const tableRows = unique
    .map((v) => {
      const word = escapeHtml(v.word)
      const phonetic = escapeHtml(v.phonetic || '—')
      const def = escapeHtml(v.definition || '—')
      const proficiency =
        v.proficiency === 'mastered'
          ? '已掌握'
          : v.proficiency === 'familiar'
            ? '熟悉'
            : '生疏'
      return `<tr>
        <td style="padding:4px 8px;border:1px solid #ddd;">${word}</td>
        <td style="padding:4px 8px;border:1px solid #ddd;">${phonetic}</td>
        <td style="padding:4px 8px;border:1px solid #ddd;">${def}</td>
        <td style="padding:4px 8px;border:1px solid #ddd;">${proficiency}</td>
      </tr>`
    })
    .join('\n')

  const table = `
    <h2 style="margin-top:2em;">词汇附录</h2>
    <table style="width:100%;border-collapse:collapse;margin-top:1em;font-size:0.85em;">
      <thead>
        <tr style="background:#f5f5f5;">
          <th style="padding:6px 8px;border:1px solid #ddd;text-align:left;">单词</th>
          <th style="padding:6px 8px;border:1px solid #ddd;text-align:left;">音标</th>
          <th style="padding:6px 8px;border:1px solid #ddd;text-align:left;">释义</th>
          <th style="padding:6px 8px;border:1px solid #ddd;text-align:left;">熟练度</th>
        </tr>
      </thead>
      <tbody>
        ${tableRows}
      </tbody>
    </table>`

  return paragraphs + table
}

/**
 * Build plain text body (no annotation). Used for 'none' mode.
 */
function buildPlainBody(text: string): string {
  return text
    .split(/\n\s*\n/)
    .filter((p) => p.trim())
    .map((para) => {
      const escaped = escapeHtml(para.trim())
      const lines = escaped.split('\n').join('<br>')
      return `<p style="text-indent:2em;margin:0 0 0.5em 0;">${lines}</p>`
    })
    .join('\n')
}

/**
 * Dispatch body builder based on annotation mode.
 *
 * - inline: vocab annotated inline after each word
 * - appendix: clean text + vocab table at end
 * - none: clean text only, no vocab
 * - sidebar: falls back to appendix (true sidebar requires two-column
 *   layout which html2pdf renders unreliably; TODO if needed)
 */
function buildBody(
  text: string,
  vocabs: VocabWord[],
  mode: string,
  lineHeight: number,
): string {
  switch (mode) {
    case 'inline':
      return buildInlineBody(text, vocabs, lineHeight)
    case 'none':
      return buildPlainBody(text)
    case 'sidebar':
      // Fallback: sidebar layout is unreliable with html2pdf
      return buildAppendixBody(text, vocabs)
    case 'appendix':
    default:
      return buildAppendixBody(text, vocabs)
  }
}

/**
 * Build the complete HTML document for PDF generation.
 */
function buildHtml(
  novel: Novel,
  template: PdfTemplate,
  vocabs: VocabWord[],
): string {
  const title = escapeHtml(novel.title || '未命名')
  const author = escapeHtml(novel.author || '')
  const date = formatDate(novel.updatedAt || novel.createdAt || '')
  const text = novel.cleanedText || novel.rawText || ''

  const lineHeight = template.lineSpacing || 1.5
  const fontFamily = template.fontFamily || 'SimSun'
  const fontSize = template.fontSize || 14

  const body = buildBody(text, vocabs, template.annotationMode, lineHeight)

  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: ${fontFamily}, serif;
    font-size: ${fontSize}px;
    line-height: ${lineHeight};
    color: #222;
    padding: 10px;
  }
  .title-page {
    text-align: center;
    padding-top: 120px;
    margin-bottom: 60px;
  }
  .title-page h1 {
    font-size: ${fontSize + 8}px;
    margin-bottom: 16px;
  }
  .title-page .meta {
    font-size: ${fontSize}px;
    color: #555;
  }
  .page-break {
    page-break-after: always;
  }
</style>
</head>
<body>
<div class="title-page">
  <h1>${title}</h1>
  <p class="meta">${author}</p>
  <p class="meta">${date}</p>
</div>
<div class="page-break"></div>
${body}
</body>
</html>`
}

/**
 * Export novel content as PDF using html2pdf.js.
 */
export async function exportPdf(
  novel: Novel,
  template: PdfTemplate,
  vocabs: VocabWord[],
): Promise<void> {
  const html = buildHtml(novel, template, vocabs)
  const margins = parseMargins(template.margins)

  const opt = {
    margin: [margins.top, margins.right, margins.bottom, margins.left] as [number, number, number, number],
    filename: `${novel.title || 'export'}.pdf`,
    image: { type: 'jpeg' as const, quality: 0.98 },
    html2canvas: {
      scale: 2,
      useCORS: true,
      letterRendering: true,
    },
    jsPDF: {
      unit: 'mm' as const,
      format: template.paperSize === 'A4' ? 'a4' : template.paperSize === 'A5' ? 'a5' : 'a4',
      orientation: 'portrait' as const,
    },
    pagebreak: { mode: ['css', 'legacy'] },
  }

  const container = document.createElement('div')
  container.innerHTML = html
  container.style.position = 'absolute'
  container.style.left = '-9999px'
  container.style.top = '0'
  document.body.appendChild(container)

  try {
    await html2pdf().set(opt).from(container).save()
  } finally {
    document.body.removeChild(container)
  }
}
