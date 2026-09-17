/** 熟练度档位：ignore 为逐词阅读的"忽略"标记，不进入复习队列与统计 */
export type Proficiency = 'unknown' | 'familiar' | 'mastered' | 'ignore'

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
  proficiency: Proficiency
  memoryTag: string
  createdAt: string
  matchTerms: string
}

export interface VocabWordFormData {
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  proficiency: Exclude<Proficiency, 'ignore'>
  memoryTag: string
  chapterId?: number | null
  proficiencyChanged?: boolean
}

export interface HighlightWord {
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  novelId: number | null
  proficiency: Proficiency
  matchTerms: string
}

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
