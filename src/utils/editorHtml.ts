/**
 * Shared helpers for converting between plain text and the HTML that
 * Tiptap / the preview panel render.
 *
 * Single source of truth so NovelEditor.vue and PreviewPanel.vue can't
 * drift on the "is this already HTML?" check or the paragraph-wrap rule.
 */

/** True if `s` looks like an HTML fragment (starts with `<` after trimming). */
export function looksLikeHtml(s: string): boolean {
  return s.trimStart().startsWith('<')
}

/**
 * Wrap plain text in `<p>...</p>` blocks (paragraph = separated by blank lines,
 * intra-paragraph newlines become `<br>`). If the input is already HTML it is
 * returned unchanged.
 */
export function plainTextToHtml(s: string): string {
  if (!s) return ''
  if (looksLikeHtml(s)) return s
  return (
    '<p>' +
    s.split(/\n{2,}/).map((p) => p.replace(/\n/g, '<br>')).join('</p><p>') +
    '</p>'
  )
}