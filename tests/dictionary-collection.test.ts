import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import DictLookupPopover from '@/components/novel/DictLookupPopover.vue'
import { useDictionaryStore } from '@/stores/dictionaryStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'

const invoke = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('@/utils/speech', () => ({ speakWord: vi.fn() }))
const books = [{ id: 1, name: '阅读生词', isPreset: false }, { id: 2, name: '小说精选', isPreset: false }, { id: 99, name: 'CET-4 只读预设', isPreset: true }]
const dictionaryWord = { word: 'garden', phonetic_uk: '', phonetic_us: '', translation: '花园', frequency: 1, difficulty: 1 }
const shared = { id: 8, word: 'garden', proficiency: 'mastered', memoryTag: '', sourceBooks: [{ id: 2, name: '小说精选' }], active: true }
const wrappers: VueWrapper[] = []
beforeEach(() => {
  setActivePinia(createPinia()); invoke.mockReset()
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'get_all_vocab_books') return books
    if (cmd === 'lookup_user_vocab') return shared
    if (cmd === 'create_vocab_word') return { id: 4, userVocabId: 8, word: 'garden', proficiency: 'mastered' }
    return null
  })
  useDictionaryStore().currentWord = { ...dictionaryWord }
})
afterEach(() => { wrappers.splice(0).forEach(w => w.unmount()); document.body.innerHTML = '' })
async function mountPopover() {
  const wrapper = mount(DictLookupPopover, { attachTo: document.body, props: { text: 'garden', position: { x: 30, y: 40 }, novelId: 7, chapterId: 12 }, global: { plugins: [ElementPlus], stubs: { ElSelect: { props: ['modelValue', 'disabled'], emits: ['update:modelValue'], template: '<select :value="modelValue" :disabled="disabled" @change="$emit(\'update:modelValue\', Number($event.target.value))"><slot /></select>' }, ElOption: { props: ['value', 'label'], template: '<option :value="value">{{ label }}</option>' } } } })
  wrappers.push(wrapper); await flushPromises()
  return wrapper
}
describe('collect dictionary words into personal books', () => {
  it('preserves the user-selected cached book when initialization finishes later', async () => {
    useVocabBookStore().books = books.filter(b=>!b.isPreset) as any
    useSettingsStore().defaultVocabBookId=1
    let finish!: (value:unknown)=>void
    invoke.mockImplementation(cmd=>cmd==='get_all_vocab_books' ? new Promise(resolve=>{finish=resolve}) : Promise.resolve(shared))
    const wrapper=await mountPopover()
    await wrapper.find('select').setValue('2')
    finish(books);await flushPromises()
    expect((wrapper.find('select').element as HTMLSelectElement).value).toBe('2')
  })
  it('hides read-only presets and replaces an invalid default with a personal book', async () => {
    useSettingsStore().defaultVocabBookId = 99
    const wrapper = await mountPopover()
    expect(wrapper.findAll('option').map(o => o.text())).toEqual(['阅读生词', '小说精选'])
    expect((wrapper.find('select').element as HTMLSelectElement).value).toBe('1')
  })
  it('can collect the same word into another book while recognizing normalized duplicates in the first', async () => {
    const wrapper = await mountPopover()
    await wrapper.find('button.el-button--primary').trigger('click'); await flushPromises()
    expect(wrapper.find('button.el-button--primary').attributes('disabled')).toBeDefined()
    useDictionaryStore().currentWord = { ...dictionaryWord, word: ' GARDEN ' }; await flushPromises()
    expect(wrapper.find('button.el-button--primary').attributes('disabled')).toBeDefined()
    await wrapper.find('select').setValue('2'); await flushPromises()
    expect(wrapper.find('button.el-button--primary').attributes('disabled')).toBeUndefined()
    await wrapper.find('button.el-button--primary').trigger('click'); await flushPromises()
    expect(invoke).toHaveBeenLastCalledWith('create_vocab_word', expect.objectContaining({ vocabBookId: 2, novelId: 7, chapterId: 12 }))
  })
  it('shows existing proficiency automatically when an English dictionary result appears', async () => {
    const wrapper = await mountPopover()
    expect(invoke).toHaveBeenCalledWith('lookup_user_vocab', { word: 'garden' })
    expect(wrapper.text()).toContain('继承已有学习状态')
    expect(wrapper.text()).toContain('已掌握')
  })
  it('ignores learning-state responses for a dictionary result that has been replaced', async () => {
    let resolveGarden!: (value: unknown) => void
    invoke.mockImplementation(async (cmd: string, args: { word?: string }) => {
      if (cmd === 'get_all_vocab_books') return books
      if (cmd === 'lookup_user_vocab') {
        if (args.word === 'garden') return new Promise(resolve => { resolveGarden = resolve })
        return { ...shared, word: 'door', proficiency: 'familiar' }
      }
      return null
    })
    const wrapper = await mountPopover()
    expect(invoke).toHaveBeenCalledWith('lookup_user_vocab', { word: 'garden' })
    useDictionaryStore().currentWord = { ...dictionaryWord, word: 'door' }; await flushPromises()
    expect(wrapper.text()).toContain('熟悉')
    resolveGarden(shared); await flushPromises()
    expect(wrapper.text()).toContain('熟悉')
    expect(wrapper.text()).not.toContain('已掌握')
  })
  it('uses a labeled keyboard-focusable pronunciation button for Chinese matches', async () => {
    const store = useDictionaryStore(); store.direction = 'chinese'; store.currentWord = null; store.chineseResults = [{ ...dictionaryWord }]
    const wrapper = await mountPopover()
    const button = wrapper.find('button.dict-list-speak')
    expect(button.exists()).toBe(true)
    expect(button.attributes('aria-label')).toContain('garden')
  })
})
