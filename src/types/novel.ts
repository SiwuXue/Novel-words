export interface Novel {
  id: number
  title: string
  author: string
  category: string
  rawText: string
  cleanedText: string
  isFavorite: boolean
  createdAt: string
  updatedAt: string
}

export interface NovelFormData {
  title: string
  author: string
  category: string
  rawText?: string
  cleanedText?: string
}

export interface Chapter {
  title: string
  content: string
  startIndex: number
}

export interface ImportResult {
  chapters: Chapter[]
  rawText: string
  cleanedText: string
  detectedTitle: string
}
