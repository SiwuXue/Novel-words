export interface VocabBook {
  id: number
  name: string
  description: string
  createdAt: string
  updatedAt: string
}

export interface VocabWord {
  id: number
  vocabBookId: number
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  novelId: number | null
  proficiency: 'unknown' | 'familiar' | 'mastered'
  memoryTag: string
  createdAt: string
}

export interface HighlightWord {
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  proficiency: 'unknown' | 'familiar' | 'mastered'
}

export interface VocabBookFormData {
  name: string
  description: string
}

export interface VocabWordFormData {
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  proficiency: 'unknown' | 'familiar' | 'mastered'
  memoryTag: string
}
