import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import DictLookupPopover from '@/components/novel/DictLookupPopover.vue'
import { useDictionaryStore, type OnlineDictResult } from '@/stores/dictionaryStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'

const invoke = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('@/utils/speech', () => ({ speakWord: vi.fn() }))

const books = [{ id: 1, name: '阅读生词', isPreset: false }]
const offlineWord = { word: 'garden', phonetic_uk: '', phonetic_us: '', translation: '花园', frequency: 1, difficulty: 1 }

const youdaoResult: OnlineDictResult = {
  source: 'youdao',
  word: 'serendipity',
  phonetic_uk: '[ˌserənˈdɪpəti]',
  phonetic_us: '[ˌserənˈdɪpəti]',
  audio_uk: 'https://dict.youdao.com/dictvoice?audio=serendipity&type=1',
  audio_us: 'https://dict.youdao.com/dictvoice?audio=serendipity&type=2',
  translations: ['n. 意外发现珍奇事物的才能'],
  senses: [],
  examples: [{ en: 'A lucky stroke of serendipity.', zh: '' }],
  url: 'https://dict.youdao.com/w/serendipity',
}

const cambridgeResult: OnlineDictResult = {
  source: 'cambridge',
  word: 'apple',
  phonetic_uk: '/ˈæp.əl/',
  phonetic_us: '/ˈæp.əl/',
  audio_uk: 'https://dictionary.cambridge.org/media/uk/apple.mp3',
  audio_us: 'https://dictionary.cambridge.org/media/us/apple.mp3',
  translations: ['noun: 苹果'],
  senses: [{ pos: 'noun', defs: ['苹果'], examples: [{ en: 'an apple tree', zh: '苹果树' }] }],
  examples: [],
  url: '',
}

const wrappers: VueWrapper[] = []
beforeEach(() => {
  setActivePinia(createPinia())
  invoke.mockReset()
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_all_vocab_books') return books
    if (cmd === 'dict_lookup_english') return offlineWord
    if (cmd === 'dict_online_youdao') return youdaoResult
    if (cmd === 'dict_online_cambridge') return cambridgeResult
    if (cmd === 'dict_translate_sentence') return { translated: '那只敏捷的棕色狐狸。', source_lang: 'EN' }
    return null
  })
})
afterEach(() => { wrappers.splice(0).forEach(w => w.unmount()); document.body.innerHTML = '' })

async function mountPopover() {
  const wrapper = mount(DictLookupPopover, {
    attachTo: document.body,
    props: { text: 'garden', position: { x: 30, y: 40 }, novelId: null, chapterId: null },
    global: { plugins: [ElementPlus], stubs: { ElSelect: { props: ['modelValue', 'disabled'], emits: ['update:modelValue'], template: '<select :value="modelValue"><option :value="1">阅读生词</option></select>' } } },
  })
  wrappers.push(wrapper)
  await flushPromises()
  return wrapper
}

describe('online dictionary sources', () => {
  it('routes single English words to the offline dictionary', async () => {
    const store = useDictionaryStore()
    await store.lookupAuto('garden')
    expect(invoke).toHaveBeenCalledWith('dict_lookup_english', { word: 'garden' })
    expect(invoke).not.toHaveBeenCalledWith('dict_translate_sentence', expect.anything())
    expect(store.direction).toBe('english')
    expect(store.activeSource).toBe('offline')
    expect(store.currentWord?.word).toBe('garden')
  })

  it('falls back to Youdao web parsing when the offline dictionary has no entry', async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_all_vocab_books') return books
      if (cmd === 'dict_lookup_english') return null
      if (cmd === 'dict_online_youdao') return youdaoResult
      return null
    })
    const store = useDictionaryStore()
    await store.lookupAuto('serendipity')
    expect(invoke).toHaveBeenCalledWith('dict_online_youdao', { word: 'serendipity' })
    expect(store.activeSource).toBe('youdao')
    expect(store.onlineResult?.translations[0]).toContain('意外发现')
  })

  it('routes multi-word English text to DeepL sentence translation', async () => {
    const store = useDictionaryStore()
    await store.lookupAuto('The quick brown fox jumps')
    expect(invoke).toHaveBeenCalledWith('dict_translate_sentence', { text: 'The quick brown fox jumps' })
    expect(store.direction).toBe('sentence')
    expect(store.sentenceTranslation).toContain('狐狸')
  })

  it('routes short Chinese words offline and Chinese sentences to DeepL', async () => {
    const store = useDictionaryStore()
    await store.lookupAuto('花园')
    expect(invoke).toHaveBeenCalledWith('dict_lookup_chinese', { keyword: '花园' })
    expect(store.direction).toBe('chinese')
    await store.lookupAuto('那只敏捷的棕色狐狸跳过了那只懒惰的狗，然后扬长而去。')
    expect(invoke).toHaveBeenCalledWith('dict_translate_sentence', { text: '那只敏捷的棕色狐狸跳过了那只懒惰的狗，然后扬长而去。' })
    expect(store.sentenceTranslation).toBeTruthy()
  })

  it('shows source tabs and renders online results inside the popover', async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_all_vocab_books') return books
      if (cmd === 'dict_lookup_english') return null
      if (cmd === 'dict_online_youdao') return youdaoResult
      return null
    })
    const store = useDictionaryStore()
    await store.lookupAuto('serendipity')
    const wrapper = await mountPopover()
    const tabs = wrapper.findAll('.dict-source-tab')
    expect(tabs.map(t => t.text())).toEqual(['离线', '有道', '剑桥'])
    expect(wrapper.find('.dict-source-tab.active').text()).toBe('有道')
    expect(wrapper.text()).toContain('意外发现珍奇事物的才能')
    expect(wrapper.text()).toContain('A lucky stroke of serendipity.')
  })

  it('switches to Cambridge on tab click and groups senses by part of speech', async () => {
    const store = useDictionaryStore()
    await store.lookupAuto('apple')
    const wrapper = await mountPopover()
    await wrapper.findAll('.dict-source-tab').find(t => t.text() === '剑桥')!.trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('dict_online_cambridge', { word: 'apple' })
    expect(store.activeSource).toBe('cambridge')
    expect(wrapper.text()).toContain('noun')
    expect(wrapper.text()).toContain('苹果树')
  })

  it('shows online errors without breaking the popover', async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_all_vocab_books') return books
      if (cmd === 'dict_lookup_english') return null
      if (cmd === 'dict_online_youdao') throw new Error('词典网站返回 403')
      return null
    })
    const store = useDictionaryStore()
    await store.lookupAuto('serendipity')
    const wrapper = await mountPopover()
    expect(wrapper.text()).toContain('词典网站返回 403')
    // 错误状态下可以切回离线源
    await wrapper.findAll('.dict-source-tab').find(t => t.text() === '离线')!.trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('dict_lookup_english', { word: 'serendipity' })
  })

  it('collects online results into a personal vocab book', async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_all_vocab_books') return books
      if (cmd === 'dict_lookup_english') return null
      if (cmd === 'dict_online_youdao') return youdaoResult
      if (cmd === 'create_vocab_word') return { id: 9 }
      return null
    })
    const store = useDictionaryStore()
    await store.lookupAuto('serendipity')
    const wrapper = await mountPopover()
    const collect = wrapper.find('button.el-button--primary')
    await collect.trigger('click'); await flushPromises()
    expect(invoke).toHaveBeenCalledWith('create_vocab_word', expect.objectContaining({
      vocabBookId: 1,
      word: 'serendipity',
      definition: 'n. 意外发现珍奇事物的才能',
    }))
  })

  it('marks a stale sentence response as ignored when a new lookup starts', async () => {
    const store = useDictionaryStore()
    const resolvers: Array<(v: unknown) => void> = []
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'get_all_vocab_books') return books
      if (cmd === 'dict_translate_sentence') {
        return new Promise(resolve => { resolvers.push(resolve) })
      }
      return offlineWord
    })
    const p1 = store.lookupSentence('first sentence')
    const p2 = store.lookupSentence('second sentence')
    resolvers[0]({ translated: '旧结果', source_lang: 'EN' })
    resolvers[1]({ translated: '新结果', source_lang: 'EN' })
    await p1; await p2; await flushPromises()
    // 旧请求的结果不应覆盖新查询
    expect(store.sentenceTranslation).toBe('新结果')
  })

  it('returns null online result when a source is not mocked', async () => {
    invoke.mockImplementation(async () => null)
    const store = useDictionaryStore()
    await store.lookupOnline('word', 'cambridge')
    expect(store.activeSource).toBe('cambridge')
    expect(store.onlineResult).toBeNull()
    expect(useVocabBookStore()).toBeTruthy()
  })
})
