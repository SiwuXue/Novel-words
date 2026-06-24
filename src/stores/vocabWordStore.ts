import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { VocabWord, VocabWordFormData } from '@/types/vocabWord'

export const useVocabWordStore = defineStore('vocabWord', () => {
  const words = ref<VocabWord[]>([])
  const loading = ref(false)

  async function fetchAll(bookId: number) {
    loading.value = true
    try {
      words.value = await invoke<VocabWord[]>('get_vocab_words', {
        vocabBookId: bookId,
      })
    } catch (e) {
      console.error('[vocabWordStore] fetchAll failed:', e)
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
    await invoke('update_vocab_word', {
      id,
      word: data.word,
      definition: data.definition || '',
      phonetic: data.phonetic || '',
      exampleSentence: data.exampleSentence || '',
      proficiency: data.proficiency || 'unknown',
      memoryTag: data.memoryTag || '',
    })
    const idx = words.value.findIndex((w) => w.id === id)
    if (idx !== -1) {
      words.value[idx] = {
        ...words.value[idx],
        ...data,
      }
    }
  }

  async function remove(id: number) {
    await invoke('delete_vocab_word', { id })
    words.value = words.value.filter((w) => w.id !== id)
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

  return { words, loading, fetchAll, create, update, remove, search }
})
