export interface VocabWord {
  id: number
  vocabBookId: number
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  novelId: number | null
  chapterId: number | null
  proficiency: 'unknown' | 'familiar' | 'mastered'
  memoryTag: string
  createdAt: string
  matchTerms: string
}

export interface VocabWordFormData {
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  proficiency: 'unknown' | 'familiar' | 'mastered'
  memoryTag: string
  chapterId?: number | null
}

export interface HighlightWord {
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  novelId: number | null
  proficiency: 'unknown' | 'familiar' | 'mastered'
  matchTerms: string
}
