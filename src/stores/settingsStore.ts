import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { PdfTemplate } from '@/types/pdf'

export const useSettingsStore = defineStore('settings', () => {
  const theme = ref<'light' | 'dark'>('light')
  const defaultExportFolder = ref('')
  const defaultVocabBookId = ref<number | null>(null)
  const pdfTemplate = ref<PdfTemplate | null>(null)
  const pdfTemplates = ref<PdfTemplate[]>([])

  // Placeholder — will be implemented in Step 14

  return { theme, defaultExportFolder, defaultVocabBookId, pdfTemplate, pdfTemplates }
})
