export interface VocabBook {
  id: number
  name: string
  description: string
  isPreset: boolean
  presetKey: string
  clonedFromPresetKey: string
  createdAt: string
  updatedAt: string
}

export interface VocabBookFormData {
  name: string
  description: string
}

/** A bundled preset vocab book (read-only reference, e.g. CET4). */
export interface PresetVocabBook {
  id: number
  name: string
  description: string
  presetKey: string
  wordCount: number
}

/** One tailored word in the clone preview. */
export interface PresetCloneItem {
  word: string
  definition: string
  phonetic: string
  exampleSentence: string
  hitCount: number
}

/** Result of preview_preset_clone. */
export interface PresetClonePreview {
  presetKey: string
  novelId: number
  totalPresetWords: number
  matchedCount: number
  limit: number
  items: PresetCloneItem[]
}
