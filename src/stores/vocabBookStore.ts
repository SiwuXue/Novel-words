import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { VocabBook, VocabBookFormData } from '@/types/vocabBook'

export const useVocabBookStore = defineStore('vocabBook', () => {
  const books = ref<VocabBook[]>([])
  const loading = ref(false)

  async function fetchAll() {
    loading.value = true
    try {
      books.value = await invoke<VocabBook[]>('get_all_vocab_books')
    } catch (e) {
      console.error('[vocabBookStore] fetchAll failed:', e)
    } finally {
      loading.value = false
    }
  }

  async function create(data: VocabBookFormData) {
    const book = await invoke<VocabBook>('create_vocab_book', {
      name: data.name,
      description: data.description || '',
    })
    books.value.unshift(book)
    return book
  }

  async function update(id: number, data: VocabBookFormData) {
    await invoke('update_vocab_book', {
      id,
      name: data.name,
      description: data.description || '',
    })
    const idx = books.value.findIndex((b) => b.id === id)
    if (idx !== -1) {
      books.value[idx] = {
        ...books.value[idx],
        name: data.name,
        description: data.description || '',
        updatedAt: new Date().toISOString(),
      }
    }
  }

  async function remove(id: number) {
    await invoke('delete_vocab_book', { id })
    books.value = books.value.filter((b) => b.id !== id)
  }

  return { books, loading, fetchAll, create, update, remove }
})
