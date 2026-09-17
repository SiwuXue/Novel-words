/**
 * 英文逐词阅读的分词工具。
 *
 * 将 Tiptap HTML（<p>/<h1>-<h3>/<blockquote>/<li> 等块级元素）解析为
 * 块级 token 序列：每个块保留原始标签，内部文本按"可点击单词 / 普通文本"拆分。
 * 英文分词用规则实现：字母串（含撇号、连字符连接）为一个词；纯数字不作为词。
 */

export interface WordToken {
  type: 'word'
  /** 原文（保留大小写，用于查词） */
  text: string
  /** 归一化 key（与后端 word_key 一致：弯引号转直引号、压缩空白、小写） */
  key: string
}

export interface TextToken {
  type: 'text'
  text: string
}

/** 已保存短语：相邻若干单词合并后的整体 */
export interface PhraseToken {
  type: 'phrase'
  text: string
  key: string
}

export type Token = WordToken | TextToken | PhraseToken

export interface WordTapBlock {
  /** 渲染标签（p / h2 / blockquote / li …） */
  tag: string
  tokens: Token[]
}

/** 与后端 word_key 一致的归一化 */
export function wordKey(word: string): string {
  return word
    .replace(/[‘’]/g, "'")
    .split(/\s+/)
    .filter(Boolean)
    .join(' ')
    .toLowerCase()
}

/** 英文单词：字母串，允许内部撇号/弯引号/连字符连接（don't, mother-in-law） */
const WORD_RE = /[A-Za-z]+(?:['’-][A-Za-z]+)*/g

/** 将一段纯文本拆分为 token 序列 */
export function tokenizeText(text: string): Token[] {
  const tokens: Token[] = []
  let last = 0
  for (const match of text.matchAll(WORD_RE)) {
    const idx = match.index ?? 0
    if (idx > last) {
      tokens.push({ type: 'text', text: text.slice(last, idx) })
    }
    tokens.push({ type: 'word', text: match[0], key: wordKey(match[0]) })
    last = idx + match[0].length
  }
  if (last < text.length) {
    tokens.push({ type: 'text', text: text.slice(last) })
  }
  return tokens
}

/** 块级元素标签白名单 */
const BLOCK_TAGS = new Set(['P', 'H1', 'H2', 'H3', 'H4', 'BLOCKQUOTE', 'LI', 'UL', 'OL', 'PRE', 'DIV'])

function isElement(node: Node): node is Element {
  return node.nodeType === Node.ELEMENT_NODE
}

function isBlock(el: Element): boolean {
  return BLOCK_TAGS.has(el.tagName)
}

/** 递归收集元素内的文本（深度优先，与渲染顺序一致） */
function collectTextNodes(el: Element, out: Text[]): void {
  for (const node of el.childNodes) {
    if (node.nodeType === Node.TEXT_NODE) {
      out.push(node as Text)
    } else if (isElement(node)) {
      // 跳过脚本与样式
      const tag = (node as Element).tagName
      if (tag === 'SCRIPT' || tag === 'STYLE') continue
      collectTextNodes(node, out)
    }
  }
}

/**
 * 将 Tiptap HTML 解析为逐词渲染的块序列。
 * 每个块级元素一个 block；顶层无块级包裹的散文本归入一个 p 块。
 */
export function parseWordTapBlocks(html: string): WordTapBlock[] {
  const doc = new DOMParser().parseFromString(html, 'text/html')
  const blocks: WordTapBlock[] = []

  const pushBlock = (tag: string, textNodes: Text[]) => {
    const text = textNodes.map((n) => n.textContent ?? '').join('')
    if (!text.trim()) return
    const tokens = tokenizeText(text)
    if (tokens.some((t) => t.type === 'word')) {
      blocks.push({ tag: tag.toLowerCase(), tokens })
    }
  }

  const walk = (container: Element): void => {
    let looseText: Text[] = []
    for (const node of container.childNodes) {
      if (node.nodeType === Node.TEXT_NODE) {
        looseText.push(node as Text)
      } else if (isElement(node)) {
        const el = node as Element
        if (el.tagName === 'SCRIPT' || el.tagName === 'STYLE') continue
        if (isBlock(el)) {
          if (looseText.length) {
            pushBlock('p', looseText)
            looseText = []
          }
          // 含嵌套块（ul > li）则继续下钻，否则当前块收口
          if (Array.from(el.children).some((c) => isBlock(c))) {
            walk(el)
          } else {
            const inner: Text[] = []
            collectTextNodes(el, inner)
            pushBlock(el.tagName, inner)
          }
        } else {
          // 行内元素（strong/em 等）：并入散文本流
          collectTextNodes(el, looseText)
        }
      }
    }
    if (looseText.length) {
      pushBlock('p', looseText)
    }
  }

  walk(doc.body)
  return blocks
}

/** 收集 blocks 中的唯一词 key（用于批量查询状态） */
export function collectWordKeys(blocks: WordTapBlock[]): string[] {
  const set = new Set<string>()
  for (const block of blocks) {
    for (const token of block.tokens) {
      if (token.type === 'word') set.add(token.key)
    }
  }
  return [...set]
}

/** 逐词状态色类名 */
export function stateClass(proficiency: string | undefined): string {
  switch (proficiency) {
    case 'unknown':
      return 'wt-st-unknown'
    case 'familiar':
      return 'wt-st-familiar'
    case 'mastered':
      return 'wt-st-mastered'
    case 'ignore':
      return 'wt-st-ignore'
    default:
      // 未收录 → 新词
      return 'wt-st-new'
  }
}

/**
 * 将相邻单词序列合并为已保存短语（贪心最长匹配）。
 * 仅当词与词之间是纯空白分隔时才算相邻（"apple, pie" 不会合并）。
 */
export function mergePhrases(blocks: WordTapBlock[], phraseKeys: Set<string>): WordTapBlock[] {
  if (phraseKeys.size === 0) return blocks
  return blocks.map((block) => {
    const tokens = block.tokens
    const out: Token[] = []
    let merged = false
    let i = 0
    while (i < tokens.length) {
      const token = tokens[i]
      if (token.type !== 'word') {
        out.push(token)
        i += 1
        continue
      }
      // 从 i 开始向前扩展，记录命中的最长短语
      const keys: string[] = []
      const words: string[] = []
      let matched: { end: number; key: string; text: string } | null = null
      let j = i
      while (j < tokens.length) {
        const current = tokens[j]
        if (current.type === 'word') {
          keys.push(current.key)
          words.push(current.text)
          j += 1
          const key = keys.join(' ')
          if (phraseKeys.has(key)) {
            matched = { end: j, key, text: words.join(' ') }
          }
        } else if (
          current.type === 'text' &&
          /^\s+$/.test(current.text) &&
          keys.length > 0
        ) {
          j += 1
        } else {
          break
        }
      }
      if (matched && matched.end - i >= 2) {
        out.push({ type: 'phrase', text: matched.text, key: matched.key })
        merged = true
        i = matched.end
      } else {
        out.push(token)
        i += 1
      }
    }
    return merged ? { ...block, tokens: out } : block
  })
}
