import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'
import type { EditorView } from '@tiptap/pm/view'
import type { HighlightWord } from '@/types/vocabWord'
import { speakWord } from '@/utils/speech'
import { useSettingsStore } from '@/stores/settingsStore'

const PLUGIN_KEY = new PluginKey('vocabHighlight')

// Theme-aware highlight colors — red / orange / gray to match the PDF preview
// and the exported PDF (see PROFICIENCY_TEXT in utils/proficiencyColors.ts).
const PROFICIENCY_COLORS: Record<string, string> = {
  unknown: 'var(--editor-hl-unknown)',
  familiar: 'var(--editor-hl-familiar)',
  mastered: 'var(--editor-hl-mastered)',
}

const PROFICIENCY_BG: Record<string, string> = {
  unknown: 'var(--editor-hl-unknown-bg)',
  familiar: 'var(--editor-hl-familiar-bg)',
  mastered: 'var(--editor-hl-mastered-bg)',
}

const PROFICIENCY_TEXTS: Record<string, string> = {
  unknown: '生疏',
  familiar: '熟悉',
  mastered: '已掌握',
}

interface PluginState {
  wordsMap: Map<string, HighlightWord>
  decorations: DecorationSet
  focusDecorations: DecorationSet
  /** Doc changed but decorations not yet rebuilt (debounced). */
  dirty: boolean
}

const REBUILD_DEBOUNCE_MS = 220

// ---- Module-level words store ----
// Stored at module level so the plugin always reads the latest words
// regardless of Tiptap's extension instance lifecycle. This avoids
// object-identity issues between extensionManager.extensions and the
// closure captured in addProseMirrorPlugins().

let currentWords: HighlightWord[] = []
let cachedWordsMap: Map<string, HighlightWord> | null = null
let cachedTargetIndex: ReturnType<typeof buildTargetIndex> | null = null

function invalidateWordCaches() {
  cachedWordsMap = null
  cachedTargetIndex = null
}

export function setVocabHighlightWords(words: HighlightWord[]): void {
  currentWords = words
  invalidateWordCaches()
}

export function refreshVocabHighlight(view: EditorView): void {
  const newState = view.state.apply(
    view.state.tr.setMeta('vocabHighlightRefresh', Date.now()),
  )
  view.updateState(newState)
}

// ---- Position search ----

function trustedCnTerms(word: HighlightWord): string[] {
  if (word.novelId == null || !word.exampleSentence?.trim()) return []
  if (word.matchTerms) {
    try {
      const saved = JSON.parse(word.matchTerms)
      if (Array.isArray(saved)) {
        const terms = saved.filter(
          (term): term is string => typeof term === 'string' && Array.from(term).length >= 2,
        )
        if (terms.length > 0) return terms
      }
    } catch {
      // Legacy/manual rows fall back to the definition + example intersection.
    }
  }
  const primaryDefinition = (word.definition || '').split('【', 1)[0]
  const terms: string[] = []
  for (const match of primaryDefinition.matchAll(/[\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff]+/g)) {
    const term = match[0]
    if (
      Array.from(term).length >= 2 &&
      word.exampleSentence.includes(term) &&
      !terms.includes(term)
    ) {
      terms.push(term)
    }
  }
  return terms
}

let focusTimer: ReturnType<typeof setTimeout> | null = null
let focusView: EditorView | null = null

function buildFocusDecorations(
  doc: { descendants: (fn: (node: { isText: boolean; text?: string }, pos: number) => boolean | void) => void },
  keyword: string,
): { decorations: DecorationSet; found: boolean } {
  const target = keyword.trim()
  if (!target) return { decorations: DecorationSet.empty, found: false }
  const pattern = new RegExp(escapeRegExp(target), 'gi')
  const decorations: Decoration[] = []
  let found = false

  doc.descendants((node, pos) => {
    if (!node.isText || !node.text) return
    pattern.lastIndex = 0
    for (let match = pattern.exec(node.text); match; match = pattern.exec(node.text)) {
      const before = node.text[match.index - 1]
      const after = node.text[match.index + match[0].length]
      const isWordChar = (char: string | undefined) => Boolean(char && /[A-Za-z0-9_]/.test(char))
      if (isWordChar(before) || isWordChar(after)) continue
      found = true
      decorations.push(
        Decoration.inline(pos + match.index, pos + match.index + match[0].length, {
          class: 'source-focus-highlight',
          style: 'color: #422006 !important; background: #facc15 !important; border: 2px solid #f59e0b; border-radius: 4px; box-shadow: 0 0 0 4px rgba(250, 204, 21, 0.4), 0 3px 12px rgba(180, 83, 9, 0.35); font-weight: 700; padding: 1px 3px;',
          nodeName: 'span',
        }),
      )
    }
  })

  return { decorations: DecorationSet.create(doc as any, decorations), found }
}

/** Add a visible, temporary source-location highlight for a word. */
export function focusVocabText(view: EditorView, keyword: string, duration = 5000): boolean {
  const result = buildFocusDecorations(view.state.doc, keyword)
  if (!result.found) return false
  if (focusTimer != null) clearTimeout(focusTimer)
  focusView = view
  view.dispatch(view.state.tr.setMeta('vocabHighlightFocus', result.decorations))
  focusTimer = setTimeout(() => {
    if (focusView === view) {
      view.dispatch(view.state.tr.setMeta('vocabHighlightFocus', DecorationSet.empty))
      focusView = null
    }
    focusTimer = null
  }, Math.max(100, duration))
  return true
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function buildTargetIndex(words: HighlightWord[]): {
  byTerm: Map<string, HighlightWord>
  pattern: RegExp | null
} {
  const byTerm = new Map<string, HighlightWord>()
  for (const word of words) {
    const cnTerms = trustedCnTerms(word)
    const targets = cnTerms.length > 0 ? cnTerms : [word.word]
    for (const target of targets) {
      if (target && !byTerm.has(target)) byTerm.set(target, word)
    }
  }
  const alternatives = [...byTerm.keys()].sort(
    (a, b) => Array.from(b).length - Array.from(a).length,
  )
  return {
    byTerm,
    pattern: alternatives.length > 0
      ? new RegExp(alternatives.map(escapeRegExp).join('|'), 'g')
      : null,
  }
}

function getTargetIndex() {
  if (!cachedTargetIndex) cachedTargetIndex = buildTargetIndex(currentWords)
  return cachedTargetIndex
}

function buildDecorations(
  doc: { descendants: (fn: (node: { isText: boolean; text?: string }, pos: number) => boolean | void) => void },
  words: HighlightWord[],
): DecorationSet {
  const decorations: Decoration[] = []
  const { byTerm, pattern } = words === currentWords
    ? getTargetIndex()
    : buildTargetIndex(words)
  if (!pattern) return DecorationSet.empty

  doc.descendants((node, pos) => {
    if (!node.isText) return
    const text = node.text || ''
    pattern.lastIndex = 0
    for (let match = pattern.exec(text); match; match = pattern.exec(text)) {
      const hw = byTerm.get(match[0])
      if (!hw) continue
      const color = PROFICIENCY_COLORS[hw.proficiency] || PROFICIENCY_COLORS.unknown
      const bg = PROFICIENCY_BG[hw.proficiency] || PROFICIENCY_BG.unknown
      decorations.push(
        Decoration.inline(pos + match.index, pos + match.index + match[0].length, {
          class: 'vocab-highlight',
          style: `color: ${color}; background-color: ${bg}; border-radius: 2px; cursor: pointer; font-weight: 500;`,
          nodeName: 'span',
        }),
      )
    }
  })

  return DecorationSet.create(doc as any, decorations)
}

function buildWordsMap(words: HighlightWord[]): Map<string, HighlightWord> {
  const map = new Map<string, HighlightWord>()
  for (const hw of words) {
    if (!map.has(hw.word)) {
      map.set(hw.word, hw)
    }
    for (const term of trustedCnTerms(hw)) {
      if (!map.has(term)) map.set(term, hw)
    }
  }
  return map
}

function getWordsMap() {
  if (!cachedWordsMap) cachedWordsMap = buildWordsMap(currentWords)
  return cachedWordsMap
}

// ---- Tooltip singleton ----

let tooltipEl: HTMLDivElement | null = null

function getTooltip(): HTMLDivElement {
  if (!tooltipEl) {
    tooltipEl = document.createElement('div')
    tooltipEl.className = 'vocab-highlight-tooltip'
    tooltipEl.style.cssText =
      'position:fixed;z-index:9999;display:none;max-width:300px;padding:10px 14px;' +
      'border-radius:6px;box-shadow:0 2px 12px rgba(0,0,0,.12);' +
      'font-size:13px;line-height:1.6;pointer-events:auto;'
    // Hide when the mouse leaves the tooltip itself (e.g. after clicking 朗读).
    tooltipEl.addEventListener('mouseleave', () => hideTooltip())
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
      ${hw.phonetic ? `<span style="color:var(--text-secondary);font-weight:400;"> ${escapeHtml(hw.phonetic)}</span>` : ''}
      <span style="margin-left:6px;font-size:12px;color:var(--text-secondary);">${PROFICIENCY_TEXTS[hw.proficiency] || ''}</span>
    </div>
    ${hw.definition ? `<div style="color:var(--text-primary);">${escapeHtml(hw.definition)}</div>` : ''}
    ${hw.exampleSentence ? `<div style="color:var(--text-secondary);font-size:12px;margin-top:2px;">例句：${escapeHtml(hw.exampleSentence)}</div>` : ''}
    <div style="margin-top:6px;"><span data-speak="${escapeHtml(hw.word)}" style="cursor:pointer;color:var(--accent-color);">朗读</span></div>
  `

  // 朗读按钮点击
  const speakBtn = tip.querySelector('[data-speak]')
  if (speakBtn) {
    speakBtn.addEventListener('click', (e: Event) => {
      e.stopPropagation()
      let accent: 'us' | 'uk' = 'us'
      try {
        accent = useSettingsStore().speechAccent
      } catch {
        /* Pinia not ready, use default */
      }
      speakWord(hw.word, accent)
    })
  }

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
    // Seed the module-level store from initial options
    currentWords = this.options.words
    invalidateWordCaches()

    // Captured in `view()` so we can schedule debounced rebuilds.
    let editorView: EditorView | null = null
    let rebuildTimer: ReturnType<typeof setTimeout> | null = null
    let rebuildGeneration = 0

    function isLargeDocument(view: EditorView): boolean {
      return view.state.doc.content.size > 200_000 && currentWords.length > 200
    }

    function startAsyncRebuild(generation: number) {
      const view = editorView
      if (!view || generation !== rebuildGeneration) return
      const doc = view.state.doc
      const { byTerm, pattern } = getTargetIndex()
      view.dispatch(view.state.tr.setMeta('vocabHighlightClear', true))
      if (!pattern) return

      const textNodes: Array<{ text: string; pos: number }> = []
      doc.descendants((node, pos) => {
        if (node.isText && node.text) textNodes.push({ text: node.text, pos })
      })
      let nodeIndex = 0

      const processChunk = () => {
        const currentView = editorView
        if (!currentView || generation !== rebuildGeneration || currentView.state.doc !== doc) return
        const decorations: Decoration[] = []
        const startedAt = performance.now()
        while (nodeIndex < textNodes.length && performance.now() - startedAt < 8) {
          const node = textNodes[nodeIndex++]
          pattern.lastIndex = 0
          for (let match = pattern.exec(node.text); match; match = pattern.exec(node.text)) {
            const hw = byTerm.get(match[0])
            if (!hw) continue
            const color = PROFICIENCY_COLORS[hw.proficiency] || PROFICIENCY_COLORS.unknown
            const bg = PROFICIENCY_BG[hw.proficiency] || PROFICIENCY_BG.unknown
            decorations.push(
              Decoration.inline(node.pos + match.index, node.pos + match.index + match[0].length, {
                class: 'vocab-highlight',
                style: `color: ${color}; background-color: ${bg}; border-radius: 2px; cursor: pointer; font-weight: 500;`,
                nodeName: 'span',
              }),
            )
          }
        }
        if (decorations.length > 0) {
          currentView.dispatch(
            currentView.state.tr.setMeta('vocabHighlightPartial', decorations),
          )
        }
        if (nodeIndex < textNodes.length) setTimeout(processChunk, 0)
      }
      setTimeout(processChunk, 0)
    }

    function scheduleRebuild(delay = REBUILD_DEBOUNCE_MS) {
      if (rebuildTimer != null) clearTimeout(rebuildTimer)
      const generation = ++rebuildGeneration
      rebuildTimer = setTimeout(() => {
        rebuildTimer = null
        const view = editorView
        if (!view || generation !== rebuildGeneration) return
        if (isLargeDocument(view)) startAsyncRebuild(generation)
        else view.dispatch(view.state.tr.setMeta('vocabHighlightSync', true))
      }, delay)
    }

    return [
      new Plugin<PluginState>({
        key: PLUGIN_KEY,

        state: {
          init(_config, _editorState) {
            const wordsMap = buildWordsMap(currentWords)
            return {
              wordsMap,
              decorations: DecorationSet.empty,
              focusDecorations: DecorationSet.empty,
              dirty: false,
            }
          },

          apply(tr, oldState, _oldEditorState, newEditorState) {
            const focus = tr.getMeta('vocabHighlightFocus') as DecorationSet | undefined
            if (focus) {
              return { ...oldState, focusDecorations: focus }
            }
            if (tr.getMeta('vocabHighlightClear')) {
              return {
                ...oldState,
                decorations: DecorationSet.empty,
                focusDecorations: oldState.focusDecorations.map(tr.mapping, tr.doc),
                dirty: true,
              }
            }
            const partial = tr.getMeta('vocabHighlightPartial') as Decoration[] | undefined
            if (partial) {
              return {
                ...oldState,
                decorations: oldState.decorations.add(newEditorState.doc, partial),
                focusDecorations: oldState.focusDecorations.map(tr.mapping, tr.doc),
                dirty: true,
              }
            }

            const wordsMap = getWordsMap()
            const wordsChanged = !mapsEqual(oldState.wordsMap, wordsMap)

            if (wordsChanged) {
              if (rebuildTimer != null) {
                clearTimeout(rebuildTimer)
                rebuildTimer = null
              }
              if (newEditorState.doc.content.size > 200_000 && currentWords.length > 200) {
                scheduleRebuild(0)
                return {
                  wordsMap,
                  decorations: DecorationSet.empty,
                  focusDecorations: oldState.focusDecorations.map(tr.mapping, tr.doc),
                  dirty: true,
                }
              }
              return {
                wordsMap,
                decorations: buildDecorations(newEditorState.doc, currentWords),
                focusDecorations: oldState.focusDecorations.map(tr.mapping, tr.doc),
                dirty: false,
              }
            }

            // No doc change: keep existing decorations (mapped).
            if (!tr.docChanged) {
              return {
                wordsMap,
                decorations: oldState.decorations.map(tr.mapping, tr.doc),
                focusDecorations: oldState.focusDecorations.map(tr.mapping, tr.doc),
                dirty: oldState.dirty,
              }
            }

            if (tr.getMeta('vocabHighlightSync')) {
              return {
                wordsMap,
                decorations: buildDecorations(newEditorState.doc, currentWords),
                focusDecorations: oldState.focusDecorations.map(tr.mapping, tr.doc),
                dirty: false,
              }
            }

            // Typing: keep stale decorations, schedule debounced rebuild.
            scheduleRebuild()
            return {
              wordsMap,
              decorations: oldState.decorations.map(tr.mapping, tr.doc),
              focusDecorations: oldState.focusDecorations.map(tr.mapping, tr.doc),
              dirty: true,
            }
          },
        },

        props: {
          decorations(state) {
            const ps = PLUGIN_KEY.getState(state)
            if (!ps) return DecorationSet.empty
            return DecorationSet.create(state.doc, [
              ...ps.decorations.find(),
              ...ps.focusDecorations.find(),
            ])
          },

          handleDOMEvents: {
            mouseover(_view, event) {
              const target = event.target as HTMLElement
              const span = target.closest('.vocab-highlight') as HTMLElement | null
              if (!span) {
                hideTooltip()
                return false
              }
              const word = span.textContent?.trim()
              if (!word) return false

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
                // 鼠标移到 tooltip 内（如点击朗读）时保持显示
                if (related && tooltipEl && tooltipEl.contains(related)) return false
                hideTooltip()
              }
              return false
            },
          },
        },

        view(view) {
          editorView = view
          getTooltip()
          return {
            destroy() {
              if (rebuildTimer != null) {
                clearTimeout(rebuildTimer)
                rebuildTimer = null
              }
              if (focusTimer != null) {
                clearTimeout(focusTimer)
                focusTimer = null
              }
              focusView = null
              rebuildGeneration++
              editorView = null
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
