import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { VocabWord, VocabWordFormData } from '@/types/vocabWord'

export const useVocabWordStore = defineStore('vocabWord', () => {
  const words = ref<VocabWord[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref('')
  let generation = 0
  let pageContext: { bookId: number; opts: Parameters<typeof fetchPage>[1] } | null = null

  async function fetchAll(bookId: number) {
    const request = ++generation
    pageContext = null
    loading.value = true
    error.value = ''
    try {
      const result = await invoke<VocabWord[]>('get_vocab_words', {
        vocabBookId: bookId,
      })
      if (request !== generation) return
      words.value = result
      total.value = words.value.length
    } catch (e) {
      if (request !== generation) return
      error.value = e instanceof Error ? e.message : String(e)
      console.error('[vocabWordStore] fetchAll failed:', e)
    } finally {
      if (request === generation) loading.value = false
    }
  }

  async function fetchPage(
    bookId: number,
    opts: {
      query?: string
      proficiencies?: ('unknown' | 'familiar' | 'mastered')[]
      offset: number
      limit: number
    },
  ) {
    const request = ++generation
    pageContext = { bookId, opts: { ...opts } }
    loading.value = true
    error.value = ''
    try {
      const page = await invoke<{ total: number; words: VocabWord[] }>(
        'get_vocab_words_page',
        {
          vocabBookId: bookId,
          query: opts.query?.trim() || null,
          proficiencies: opts.proficiencies?.length ? opts.proficiencies : null,
          offset: opts.offset,
          limit: opts.limit,
        },
      )
      if (request !== generation) return
      words.value = page.words
      total.value = page.total
    } catch (e) {
      if (request !== generation) return
      error.value = e instanceof Error ? e.message : String(e)
      console.error('[vocabWordStore] fetchPage failed:', e)
    } finally {
      if (request === generation) loading.value = false
    }
  }

  async function create(bookId: number, data: VocabWordFormData) {
    const request = generation
    const word = await invoke<VocabWord>('create_vocab_word', {
      vocabBookId: bookId,
      word: data.word,
      definition: data.definition || '',
      phonetic: data.phonetic || '',
      exampleSentence: data.exampleSentence || '',
      novelId: null,
      chapterId: data.chapterId ?? null,
      proficiency: data.proficiency || 'unknown',
      memoryTag: data.memoryTag || '',
    })
    if (request === generation) words.value.unshift(word)
    return word
  }

  async function update(id: number, data: VocabWordFormData) {
    const request = generation
    const existing = words.value.find((w) => w.id === id)
    const changed = data.proficiencyChanged ?? (existing !== undefined && data.proficiency !== existing.proficiency)

    await invoke('update_vocab_word', {
      id,
      word: data.word,
      definition: data.definition || '',
      phonetic: data.phonetic || '',
      exampleSentence: data.exampleSentence || '',
      proficiency: changed ? data.proficiency : null,
      memoryTag: data.memoryTag || '',
    })
    if (request !== generation) return
    // The backend may resolve a renamed word to a different shared record.
    if (pageContext) await fetchPage(pageContext.bookId, pageContext.opts)
    else if (existing) await fetchAll(existing.vocabBookId)
  }

  async function remove(id: number) {
    await invoke('delete_vocab_word', { id })
    words.value = words.value.filter((w) => w.id !== id)
  }

  async function removeMany(ids: number[]) {
    if (ids.length === 0) return 0
    const count = await invoke<number>('delete_vocab_words', { ids })
    const idSet = new Set(ids)
    words.value = words.value.filter((w) => !idSet.has(w.id))
    return count
  }

  async function search(bookId: number, query: string) {
    const request = ++generation
    pageContext = null
    loading.value = true
    error.value = ''
    try {
      const result = await invoke<VocabWord[]>('search_vocab_words', {
        vocabBookId: bookId,
        query,
      })
      if (request !== generation) return
      words.value = result
      total.value = result.length
    } catch (e) {
      if (request !== generation) return
      error.value = e instanceof Error ? e.message : String(e)
      console.error('[vocabWordStore] search failed:', e)
    } finally {
      if (request === generation) loading.value = false
    }
  }

  return { error, words, total, loading, fetchAll, fetchPage, create, update, remove, removeMany, search }
})
