import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'
import ElementPlus from 'element-plus'
import VocabBookListPage from '@/views/VocabBookListPage.vue'
import PresetVocabBooksPage from '@/views/PresetVocabBooksPage.vue'
import VocabWordFormDialog from '@/components/vocabulary/VocabWordFormDialog.vue'
import PresetImportDialog from '@/components/vocabulary/PresetImportDialog.vue'
import AllVocabularyPanel from '@/components/vocabulary/AllVocabularyPanel.vue'
import { useVocabWordStore } from '@/stores/vocabWordStore'

const { invoke, listen } = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen }))
const wrappers: VueWrapper[] = []
const savedWord = { id: 3, userVocabId: 9, vocabBookId: 1, word: 'garden', definition: '花园', phonetic: '', exampleSentence: '', novelId: null, chapterId: null, proficiency: 'mastered' as const, memoryTag: '', createdAt: '', matchTerms: '' }
const entry = { id: 9, word: 'garden', definition: '花园', phonetic: '', exampleSentence: '', proficiency: 'mastered', memoryTag: '', lastReviewedAt: 12, active: false, sourceBooks: [] }
const catalog = [{ id: 1, name: '大学英语四级', description: '考试词汇', presetKey: 'cet4', wordCount: 4500, category: 'university', sources: ['CET4luan_1', 'CET4luan_2'] }, { id: 2, name: '人教版初中七年级上册', description: '分册词汇', presetKey: 'PEPChuZhong7_1', wordCount: 300, category: 'textbook', sources: ['PEPChuZhong7_1'] }]
beforeEach(() => {
  setActivePinia(createPinia()); invoke.mockReset(); listen.mockReset()
  listen.mockResolvedValue(vi.fn())
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_all_vocab_books' || cmd === 'get_all_novels') return []
    if (cmd === 'get_user_vocab_page') return { total: 1, words: [entry] }
    if (cmd === 'lookup_user_vocab') return entry
    if (cmd === 'list_preset_vocab_books') return catalog
    return null
  })
})
afterEach(() => { wrappers.splice(0).forEach(w => w.unmount()); document.body.innerHTML = ''; vi.useRealTimers() })
async function page(component: any, path: string, props = {}) {
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/:pathMatch(.*)*', component: { template: '<div />' } }] })
  await router.push(path); await router.isReady()
  const wrapper = mount(component, { props, attachTo: document.body, global: { plugins: [createPinia(), router, ElementPlus], stubs: { teleport: true, ElSelect: { props: ['modelValue', 'disabled'], emits: ['update:modelValue'], template: '<select :value="modelValue" :disabled="disabled" @change="$emit(\'update:modelValue\', $event.target.value)"><slot /></select>' }, ElOption: { props: ['value', 'label'], template: '<option :value="value">{{ label }}</option>' } } } })
  wrappers.push(wrapper); await flushPromises()
  return { wrapper, router }
}
describe('shared user vocabulary', () => {
  it('discards an older response after switching vocabulary books', async () => {
    const store=useVocabWordStore()
    let resolveOld!: (v:any) => void
    invoke.mockImplementation((_cmd, args) => args.vocabBookId===1 ? new Promise(resolve => {resolveOld=resolve}) : Promise.resolve({ total:1, words:[{...savedWord,vocabBookId:2}] }))
    const old=store.fetchPage(1,{offset:0,limit:50})
    await store.fetchPage(2,{offset:0,limit:50})
    resolveOld({total:1,words:[savedWord]});await old
    expect(store.words[0].vocabBookId).toBe(2)
  })
  it('preserves a local tag entered while learning-state lookup is pending', async () => {
    let finish!: (v:any)=>void
    invoke.mockImplementation(cmd=> cmd==='lookup_user_vocab' ? new Promise(resolve=>{finish=resolve}) : Promise.resolve(null))
    const {wrapper}=await page(VocabWordFormDialog,'/',{modelValue:true})
    await wrapper.find('input[maxlength="200"]').setValue('garden')
    await new Promise(resolve=>setTimeout(resolve,350))
    const tag=wrapper.findAll('input').at(-1)!
    await tag.setValue('旅行')
    finish(entry);await flushPromises()
    expect((tag.element as HTMLInputElement).value).toBe('旅行')
  })
  it('updates shared status from the registry and reloads its current page', async () => {
    const { wrapper } = await page(AllVocabularyPanel, '/')
    await wrapper.findComponent({ name: 'ElTable' }).vm.$emit('selection-change', [entry, entry])
    await flushPromises()
    await wrapper.findAll('button').find(b => b.text() === '标为生疏')!.trigger('click')
    await flushPromises()
    expect(invoke).toHaveBeenCalledWith('set_user_vocab_proficiency', { ids: [9], proficiency: 'unknown' })
    expect(invoke.mock.calls.filter(([cmd]) => cmd === 'get_user_vocab_page')).toHaveLength(2)
  })
  it('opens the all-vocabulary tab from its URL and shows paused words without sources', async () => {
    const { wrapper } = await page(VocabBookListPage, '/vocabulary?tab=all')
    expect(wrapper.text()).toContain('全部词汇')
    expect(wrapper.text()).toContain('garden')
    expect(wrapper.text()).toContain('复习暂停')
    expect(invoke).toHaveBeenCalledWith('get_user_vocab_page', expect.objectContaining({ offset: 0, limit: 50 }))
  })
  it('does not overwrite shared proficiency during metadata edits and reads the real updated word', async () => {
    const store = useVocabWordStore(); store.words = [savedWord]
    invoke.mockResolvedValueOnce(null).mockResolvedValueOnce([{ ...savedWord, definition: '园地', proficiency: 'familiar', userVocabId: 10 }])
    await store.update(3, { word: 'garden', definition: '园地', phonetic: '', exampleSentence: '', proficiency: 'mastered', memoryTag: '' })
    expect(invoke.mock.calls[0]).toEqual(['update_vocab_word', expect.objectContaining({ proficiency: null })])
    expect(store.words[0].proficiency).toBe('familiar')
    expect(store.words[0].userVocabId).toBe(10)
  })
  it('automatically looks up learning state when adding an existing word', async () => {
    const { wrapper } = await page(VocabWordFormDialog, '/', { modelValue: true })
    await flushPromises()
    await wrapper.find('input[maxlength="200"]').setValue('garden')
    await new Promise(resolve => setTimeout(resolve, 350)); await flushPromises()
    expect(invoke).toHaveBeenCalledWith('lookup_user_vocab', { word: 'garden' })
    expect(wrapper.text()).toContain('继承已有学习状态')
    expect(wrapper.text()).toContain('已掌握')
  })
})
describe('preset import catalog', () => {
  it('filters progress by request and supports retry after failure without leaking listeners', async () => {
    let progress: (event: any) => void = () => {}
    const stop = vi.fn(); listen.mockImplementation(async (_event, callback) => { progress = callback; return stop })
    let resolveImport!: (value: any) => void
    let rejectImport!: (reason: any) => void
    invoke.mockImplementation((cmd) => cmd === 'import_preset_vocab_book' ? new Promise((resolve, reject) => { resolveImport=resolve; rejectImport=reject }) : Promise.resolve(null))
    const { wrapper } = await page(PresetImportDialog, '/', { modelValue: true, presetKey: 'cet6-all', presetName: '六级合集' })
    await wrapper.findAll('button').find(b => b.text() === '整套导入')!.trigger('click'); await flushPromises()
    const requestId = invoke.mock.calls.find(([cmd]) => cmd === 'import_preset_vocab_book')![1].requestId
    progress({ payload: { requestId: 'stale', processed: 100, total: 3992, percent: 99, stage: 'loading' } }); await flushPromises()
    expect(wrapper.text()).not.toContain('99%')
    progress({ payload: { requestId, processed: 100, total: 3992, percent: 35, stage: 'loading' } }); await flushPromises()
    expect(wrapper.text()).toContain('35%')
    rejectImport(new Error('Resource unavailable')); await flushPromises()
    expect(wrapper.text()).toContain('Resource unavailable'); expect(stop).toHaveBeenCalledTimes(1)
    await wrapper.findAll('button').find(b => b.text() === '重试')!.trigger('click'); await flushPromises()
    resolveImport({ bookId: 12, newWords: 2000, inherited: 1992, skipped: 0, imported: 3992 }); await flushPromises()
    expect(wrapper.text()).toContain('1992'); expect(stop).toHaveBeenCalledTimes(2)
    await wrapper.findAll('button').find(b => b.text() === '打开词汇本')!.trigger('click')
    expect(wrapper.emitted('imported')?.[0]).toEqual([12])
  })
  it('offers full-list import as the primary action and searches textbook sources', async () => {
    const { wrapper } = await page(PresetVocabBooksPage, '/presets')
    expect(wrapper.findAll('button').filter(b => b.text() === '整套导入')).toHaveLength(2)
    const input = wrapper.find('input[aria-label="搜索预设词表"]')
    expect(input.exists()).toBe(true)
    await input.setValue('七年级'); await flushPromises()
    expect(wrapper.findAll('.preset-card')).toHaveLength(1)
    expect(wrapper.text()).toContain('小说精选')
  })
})
