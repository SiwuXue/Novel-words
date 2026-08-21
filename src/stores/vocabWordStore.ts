import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { VocabWord, VocabWordFormData } from '@/types/vocabWord'
import { encodeMemoryTag, parseMemoryTag } from '@/utils/srs'

export const useVocabWordStore = defineStore('vocabWord', () => {
  const words = ref<VocabWord[]>([])
  const total = ref(0)
  const loading = ref(false)

  async function fetchAll(bookId: number) {
    loading.value = true
    try {
      words.value = await invoke<VocabWord[]>('get_vocab_words', {
        vocabBookId: bookId,
      })
      total.value = words.value.length
    } catch (e) {
      console.error('[vocabWordStore] fetchAll failed:', e)
    } finally {
      loading.value = false
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
    loading.value = true
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
      words.value = page.words
      total.value = page.total
    } catch (e) {
      console.error('[vocabWordStore] fetchPage failed:', e)
    } finally {
      loading.value = false
    }
  }

  async function create(bookId: number, data: VocabWordFormData) {
    const word = await invoke<VocabWord>('create_vocab_word', {
      vocabBookId: bookId,
      word: data.word,
      definition: data.definition || '',
      phonetic: data.phonetic || '',
      exampleSentence: data.exampleSentence || '',
      novelId: null,
      proficiency: data.proficiency || 'unknown',
      memoryTag: data.memoryTag || '',
    })
    words.value.unshift(word)
    return word
  }

  async function update(id: number, data: VocabWordFormData) {
    // Preserve any SRS state while allowing the user tag to be edited.
    const existing = words.value.find((w) => w.id === id)
    const srs = existing ? parseMemoryTag(existing.memoryTag).srs : null
    const memoryTag = encodeMemoryTag(data.memoryTag || '', srs)

    await invoke('update_vocab_word', {
      id,
      word: data.word,
      definition: data.definition || '',
      phonetic: data.phonetic || '',
      exampleSentence: data.exampleSentence || '',
      proficiency: data.proficiency || 'unknown',
      memoryTag,
    })
    const idx = words.value.findIndex((w) => w.id === id)
    if (idx !== -1) {
      words.value[idx] = {
        ...words.value[idx],
        ...data,
        memoryTag,
      }
    }
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
    loading.value = true
    try {
      words.value = await invoke<VocabWord[]>('search_vocab_words', {
        vocabBookId: bookId,
        query,
      })
    } catch (e) {
      console.error('[vocabWordStore] search failed:', e)
    } finally {
      loading.value = false
    }
  }

  return { words, total, loading, fetchAll, fetchPage, create, update, remove, removeMany, search }
})
