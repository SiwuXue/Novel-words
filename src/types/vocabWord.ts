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

export interface VocabWordFormData {
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  proficiency: 'unknown' | 'familiar' | 'mastered'
  memoryTag: string
}

export interface HighlightWord {
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  proficiency: 'unknown' | 'familiar' | 'mastered'
}
