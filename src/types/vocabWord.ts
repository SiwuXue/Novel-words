export interface VocabWord {
  id: number
  userVocabId: number | null
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
  proficiencyChanged?: boolean
}

export type Proficiency = VocabWord['proficiency']

export interface UserVocabEntry {
  id: number
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  proficiency: Proficiency
  memoryTag: string
  lastReviewedAt: number
  active: boolean
  sourceBooks: { id: number; name: string }[]
}

export interface UserVocabPage {
  total: number
  words: UserVocabEntry[]
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
