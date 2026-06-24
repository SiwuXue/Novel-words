import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import type { HighlightWord } from '@/types/vocabWord'

const PLUGIN_KEY = new PluginKey('vocabHighlight')

const PROFICIENCY_COLORS: Record<string, string> = {
  unknown: '#e8e8e8',
  familiar: '#fdf2e9',
  mastered: '#e8f8e8',
}

const PROFICIENCY_TEXTS: Record<string, string> = {
  unknown: '生疏',
  familiar: '熟悉',
  mastered: '已掌握',
}

interface PluginState {
  wordsMap: Map<string, HighlightWord>
  decorations: DecorationSet
}

/**
 * Iterate every text node in the document, find all occurrences of `word`,
 * and return document positions for each match.
 */
function findWordPositions(
  doc: { descendants: (fn: (node: { isText: boolean; text?: string }, pos: number) => boolean | void) => void },
  word: string,
): Array<{ from: number; to: number }> {
  const results: Array<{ from: number; to: number }> = []
  if (!word) return results

  doc.descendants((node, pos) => {
    if (!node.isText) return
    const text: string = node.text || ''
    let idx = 0
    while (true) {
      const found = text.indexOf(word, idx)
      if (found === -1) break
      results.push({ from: pos + found, to: pos + found + word.length })
      idx = found + 1
    }
  })
  return results
}

function buildDecorations(
  doc: { descendants: (fn: (node: { isText: boolean; text?: string }, pos: number) => boolean | void) => void },
  words: HighlightWord[],
): DecorationSet {
  const decorations: Decoration[] = []

  for (const hw of words) {
    const color = PROFICIENCY_COLORS[hw.proficiency] || PROFICIENCY_COLORS.unknown
    const positions = findWordPositions(doc, hw.word)

    for (const { from, to } of positions) {
      decorations.push(
        Decoration.inline(from, to, {
          class: 'vocab-highlight',
          style: `background-color: ${color}; border-radius: 2px; cursor: pointer;`,
          nodeName: 'span',
        }),
      )
    }
  }

  return DecorationSet.create(doc as any, decorations)
}

function buildWordsMap(words: HighlightWord[]): Map<string, HighlightWord> {
  const map = new Map<string, HighlightWord>()
  for (const hw of words) {
    if (!map.has(hw.word)) {
      map.set(hw.word, hw)
    }
  }
  return map
}

// ---- Tooltip singleton ----

let tooltipEl: HTMLDivElement | null = null

function getTooltip(): HTMLDivElement {
  if (!tooltipEl) {
    tooltipEl = document.createElement('div')
    tooltipEl.className = 'vocab-highlight-tooltip'
    tooltipEl.style.cssText =
      'position:fixed;z-index:9999;display:none;max-width:300px;padding:10px 14px;' +
      'background:#fff;border:1px solid #e4e7ed;border-radius:6px;box-shadow:0 2px 12px rgba(0,0,0,.12);' +
      'font-size:13px;line-height:1.6;pointer-events:none;'
    document.body.appendChild(tooltipEl)
  }
  return tooltipEl
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function showTooltip(rect: DOMRect, hw: HighlightWord) {
  const tip = getTooltip()
  tip.innerHTML = `
    <div style="font-weight:600;margin-bottom:4px;">
      ${escapeHtml(hw.word)}
      ${hw.phonetic ? `<span style="color:#909399;font-weight:400;"> ${escapeHtml(hw.phonetic)}</span>` : ''}
      <span style="margin-left:6px;font-size:12px;color:#909399;">${PROFICIENCY_TEXTS[hw.proficiency] || ''}</span>
    </div>
    ${hw.definition ? `<div style="color:#303133;">${escapeHtml(hw.definition)}</div>` : ''}
    ${hw.exampleSentence ? `<div style="color:#909399;font-size:12px;margin-top:2px;">例句：${escapeHtml(hw.exampleSentence)}</div>` : ''}
  `

  // Position above the target element
  const top = rect.top - tip.offsetHeight - 6
  const left = rect.left + rect.width / 2 - tip.offsetWidth / 2

  const clampedTop = Math.max(4, top)
  const clampedLeft = Math.max(4, Math.min(left, window.innerWidth - tip.offsetWidth - 4))

  tip.style.top = `${clampedTop}px`
  tip.style.left = `${clampedLeft}px`
  tip.style.display = 'block'
}

function hideTooltip() {
  if (tooltipEl) {
    tooltipEl.style.display = 'none'
  }
}

// ---- Extension ----

export interface VocabHighlightOptions {
  words: HighlightWord[]
}

export const VocabHighlight = Extension.create<VocabHighlightOptions>({
  name: 'vocabHighlight',

  addOptions() {
    return { words: [] }
  },

  addProseMirrorPlugins() {
    const extension = this

    return [
      new Plugin<PluginState>({
        key: PLUGIN_KEY,

        state: {
          init(_config, _editorState) {
            const wordsMap = buildWordsMap(extension.options.words)
            return {
              wordsMap,
              decorations: DecorationSet.empty,
            }
          },

          apply(tr, oldState, _oldEditorState, newEditorState) {
            const wordsMap = buildWordsMap(extension.options.words)
            const wordsChanged = !mapsEqual(oldState.wordsMap, wordsMap)

            if (!tr.docChanged && !wordsChanged) {
              return {
                wordsMap,
                decorations: oldState.decorations.map(tr.mapping, tr.doc),
              }
            }

            const decorations = buildDecorations(newEditorState.doc, extension.options.words)
            return { wordsMap, decorations }
          },
        },

        props: {
          decorations(state) {
            const ps = PLUGIN_KEY.getState(state)
            return ps?.decorations ?? DecorationSet.empty
          },

          handleDOMEvents: {
            mouseover(_view, event) {
              const target = event.target as HTMLElement
              const span = target.closest('.vocab-highlight') as HTMLElement | null
              if (!span) {
                hideTooltip()
                return false
              }
              // Read the text content of the span as the word key
              const word = span.textContent?.trim()
              if (!word) return false

              // Get the HighlightWord from the plugin state via the editor view
              const state = (_view as any).state
              const ps = PLUGIN_KEY.getState(state) as PluginState | undefined
              const hw = ps?.wordsMap.get(word)
              if (!hw) return false

              showTooltip(span.getBoundingClientRect(), hw)
              return false
            },

            mouseout(_view, event) {
              const target = event.target as HTMLElement
              const related = event.relatedTarget as HTMLElement | null
              if (target.closest('.vocab-highlight') && !related?.closest('.vocab-highlight')) {
                hideTooltip()
              }
              return false
            },
          },
        },

        view() {
          // Ensure tooltip DOM is created
          getTooltip()
          return {
            destroy() {
              if (tooltipEl?.parentElement) {
                tooltipEl.parentElement.removeChild(tooltipEl)
              }
              tooltipEl = null
            },
          }
        },
      }),
    ]
  },
})

/** Shallow compare two maps by size and keys (words changed check). */
function mapsEqual(a: Map<string, HighlightWord>, b: Map<string, HighlightWord>): boolean {
  if (a.size !== b.size) return false
  for (const key of a.keys()) {
    if (!b.has(key)) return false
  }
  return true
}
