export interface Novel {
  id: number
  title: string
  author: string
  category: string
  rawText: string
  cleanedText: string
  isFavorite: boolean
  /** 'zh' = Chinese novel (match by Chinese definition) |
   *  'en' = English novel (match by English word) */
  language: 'zh' | 'en'
  createdAt: string
  updatedAt: string
}

export interface NovelFormData {
  title: string
  author: string
  category: string
  rawText?: string
  cleanedText?: string
  language?: 'zh' | 'en'
}

export interface Chapter {
  id: number
  novelId: number
  title: string
  content: string
  sortOrder: number
  startIndex: number
  createdAt: string
}

export interface ImportResult {
  chapters: Chapter[]
  rawText: string
  cleanedText: string
  detectedTitle: string
}
