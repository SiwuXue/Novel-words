import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Chapter } from '@/types/novel'

export const useEditorStore = defineStore('editor', () => {
  const isDirty = ref(false)
  const selectedText = ref('')
  const contextSentence = ref('')
  const chapterList = ref<Chapter[]>([])
  const showAddToVocab = ref(false)
  const lastSaveTime = ref<Date | null>(null)

  // Placeholder — will be implemented in Step 6

  return { isDirty, selectedText, contextSentence, chapterList, showAddToVocab, lastSaveTime }
})
