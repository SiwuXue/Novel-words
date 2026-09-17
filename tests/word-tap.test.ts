import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import WordTapReader from '@/components/novel/WordTapReader.vue'
import {
  tokenizeText,
  parseWordTapBlocks,
  collectWordKeys,
  wordKey,
  stateClass,
} from '@/utils/wordTap'

const invoke = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('@/utils/speech', () => ({ speakWord: vi.fn() }))
// 跳过"读完本章"的确认弹窗
vi.mock('element-plus', async importOriginal => {
  const actual = await importOriginal<typeof import('element-plus')>()
  return {
    ...actual,
    ElMessageBox: { ...actual.ElMessageBox, confirm: vi.fn(() => Promise.resolve()) },
  }
})

beforeEach(() => {
  setActivePinia(createPinia())
  invoke.mockReset()
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_all_vocab_books') return []
    if (cmd === 'dict_lookup_english') {
      return { word: 'garden', phonetic_uk: '', phonetic_us: '', translation: '花园', frequency: 1, difficulty: 1 }
    }
    if (cmd === 'lookup_word_tap_states') {
      return [
        { key: 'garden', word: 'garden', proficiency: 'unknown' },
        { key: 'apple', word: 'apple', proficiency: 'mastered' },
        { key: 'the', word: 'the', proficiency: 'ignore' },
      ]
    }
    if (cmd === 'mark_word_tap_proficiency') return 1
    return null
  })
})
afterEach(() => {
  document.body.innerHTML = ''
})

describe('word tokenizer', () => {
  it('splits words and keeps contractions and hyphens', () => {
    const tokens = tokenizeText("Don't stop — mother-in-law said, 42 apples!")
    const words = tokens.filter(t => t.type === 'word').map(t => (t as any).text)
    expect(words).toEqual(["Don't", 'stop', 'mother-in-law', 'said', 'apples'])
  })

  it('normalizes keys like the backend word_key', () => {
    expect(wordKey('Garden')).toBe('garden')
    expect(wordKey('don’t')).toBe("don't")
    expect(wordKey('  multi   word ')).toBe('multi word')
  })

  it('parses tiptap html into blocks with word tokens', () => {
    const blocks = parseWordTapBlocks('<h2>Chapter</h2><p>Hello <strong>brave</strong> world.</p>')
    expect(blocks).toHaveLength(2)
    expect(blocks[0].tag).toBe('h2')
    expect(blocks[1].tag).toBe('p')
    const words = blocks[1].tokens.filter(t => t.type === 'word').map(t => (t as any).text)
    expect(words).toEqual(['Hello', 'brave', 'world'])
  })

  it('skips script and style content', () => {
    const blocks = parseWordTapBlocks('<p>ok</p><style>.word{color:red}</style>')
    const keys = collectWordKeys(blocks)
    expect(keys).toEqual(['ok'])
  })

  it('collects unique keys and maps state classes', () => {
    expect(collectWordKeys(parseWordTapBlocks('<p>apple Apple banana</p>'))).toEqual([
      'apple',
      'banana',
    ])
    expect(stateClass(undefined)).toBe('wt-st-new')
    expect(stateClass('unknown')).toBe('wt-st-unknown')
    expect(stateClass('ignore')).toBe('wt-st-ignore')
  })
})

describe('WordTapReader', () => {
  const content = '<p>The <b>garden</b> was quiet. An <i>apple</i> fell.</p>'

  async function mountReader() {
    const wrapper = mount(WordTapReader, {
      props: { content, novelId: 3, chapterId: 9 },
      global: { plugins: [ElementPlus] },
      attachTo: document.body,
    })
    await flushPromises()
    return wrapper
  }

  it('renders every word as a clickable span with state colors', async () => {
    const wrapper = await mountReader()
    const words = wrapper.findAll('.wt-word')
    expect(words.map(w => w.text())).toEqual([
      'The',
      'garden',
      'was',
      'quiet',
      'An',
      'apple',
      'fell',
    ])
    const byText = (t: string) => words.find(w => w.text() === t)!
    expect(byText('The').classes()).toContain('wt-st-ignore')
    expect(byText('garden').classes()).toContain('wt-st-unknown')
    expect(byText('apple').classes()).toContain('wt-st-mastered')
    expect(byText('was').classes()).toContain('wt-st-new')
  })

  it('shows chapter counts for each status', async () => {
    const wrapper = await mountReader()
    const text = wrapper.find('.wt-counts').text()
    expect(text).toContain('新词 4')
    expect(text).toContain('不认识 1')
    expect(text).toContain('认识 1')
    expect(text).toContain('忽略 1')
  })

  it('opens the lookup popover when a word is clicked', async () => {
    const wrapper = await mountReader()
    await wrapper.findAll('.wt-word').find(w => w.text() === 'garden')!.trigger('click')
    await flushPromises()
    const popover = document.querySelector('.dict-lookup-popover')
    expect(popover).toBeTruthy()
    expect(popover!.textContent).toContain('花园')
  })

  it('marks proficiency from the popover quick buttons and recolors', async () => {
    const wrapper = await mountReader()
    await wrapper.findAll('.wt-word').find(w => w.text() === 'garden')!.trigger('click')
    await flushPromises()
    const unknownBtn = Array.from(
      document.querySelectorAll('.dict-quick-btn'),
    ).find(b => b.textContent === '不认识') as HTMLButtonElement
    expect(unknownBtn).toBeTruthy()
    unknownBtn.click()
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('mark_word_tap_proficiency', {
      words: ['garden'],
      proficiency: 'unknown',
    })
    // 点击后颜色立即刷新为 unknown（本来就是 unknown，改点 mastered 验证变化）
    const masteredBtn = Array.from(
      document.querySelectorAll('.dict-quick-btn'),
    ).find(b => b.textContent === '认识') as HTMLButtonElement
    masteredBtn.click()
    await flushPromises()
    expect(invoke).toHaveBeenLastCalledWith('mark_word_tap_proficiency', {
      words: ['garden'],
      proficiency: 'mastered',
    })
    const garden = wrapper.findAll('.wt-word').find(w => w.text() === 'garden')!
    expect(garden.classes()).toContain('wt-st-mastered')
  })

  it('finish chapter marks all new and unknown words as ignored', async () => {
    const wrapper = await mountReader()
    await wrapper.find('.wt-toolbar button').trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('mark_word_tap_proficiency', {
      words: expect.arrayContaining(['was', 'quiet', 'An', 'fell']),
      proficiency: 'ignore',
    })
    const was = wrapper.findAll('.wt-word').find(w => w.text() === 'was')!
    expect(was.classes()).toContain('wt-st-ignore')
  })
})
