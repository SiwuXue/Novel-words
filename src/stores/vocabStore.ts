import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { VocabBook, VocabWord, HighlightWord } from '@/types/vocabulary'

export const useVocabStore = defineStore('vocab', () => {
  const books = ref<VocabBook[]>([])
  const currentBook = ref<VocabBook | null>(null)
  const words = ref<VocabWord[]>([])
  const highlightWords = ref<HighlightWord[]>([])
  const activeVocabBookId = ref<number | null>(null)

  // Placeholder — will be implemented in Steps 8-11

  return { books, currentBook, words, highlightWords, activeVocabBookId }
})
