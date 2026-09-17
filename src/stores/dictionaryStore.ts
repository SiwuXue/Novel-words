import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface DictWord {
  word: string
  phonetic_uk: string
  phonetic_us: string
  translation: string
  frequency: number
  difficulty: number
}

/** 在线例句 */
export interface OnlineExample {
  en: string
  zh: string
}

/** 在线词典结构化释义（按词性分组） */
export interface OnlineSense {
  pos: string
  defs: string[]
  examples: OnlineExample[]
}

/** 在线词典结果（Rust dict_online_* 命令返回，字段 snake_case） */
export interface OnlineDictResult {
  source: 'youdao' | 'cambridge'
  word: string
  phonetic_uk: string
  phonetic_us: string
  audio_uk: string
  audio_us: string
  translations: string[]
  senses: OnlineSense[]
  examples: OnlineExample[]
  url: string
}

export interface SentenceTranslation {
  translated: string
  source_lang: string
}

/** 查询方向：英→中 / 中→英 / 整句翻译 */
export type LookupDirection = 'english' | 'chinese' | 'sentence'

/** 词典源：离线库 / 有道网页 / 剑桥网页 */
export type DictSource = 'offline' | 'youdao' | 'cambridge'
export type OnlineDictSource = Exclude<DictSource, 'offline'>

export const useDictionaryStore = defineStore('dictionary', () => {
  const looking = ref(false)
  /** 英→中：单个结果（可能 null） */
  const currentWord = ref<DictWord | null>(null)
  /** 中→英：多条结果列表 */
  const chineseResults = ref<DictWord[]>([])
  const lookupError = ref('')
  /** 当前查询方向，UI 用以决定渲染单条还是列表 */
  const direction = ref<LookupDirection>('english')
  /** 查询关键词（用于浮窗标题显示） */
  const keyword = ref('')

  // ----- 在线词典源 -----
  /** 弹窗当前选中的词典源（仅英→中方向可切换） */
  const activeSource = ref<DictSource>('offline')
  const onlineResult = ref<OnlineDictResult | null>(null)
  const onlineLooking = ref(false)
  const onlineError = ref('')

  // ----- 整句翻译 -----
  const sentenceTranslation = ref('')
  const sentenceSourceLang = ref('')
  const sentenceLooking = ref(false)
  const sentenceError = ref('')
  /** 防止慢请求回来后覆盖新查询 */
  let sentenceRequest = 0

  /** 英→中查询（离线库；查不到自动回落有道网页） */
  async function lookupEnglish(word: string) {
    looking.value = true
    lookupError.value = ''
    direction.value = 'english'
    currentWord.value = null
    chineseResults.value = []
    keyword.value = word
    activeSource.value = 'offline'
    onlineResult.value = null
    onlineError.value = ''
    try {
      // Rust 返回字段为 snake_case（phonetic_uk / phonetic_us）
      const raw = await invoke<DictWord | null>('dict_lookup_english', { word })
      currentWord.value = raw
    } catch (e: any) {
      lookupError.value = String(e?.message || e)
      currentWord.value = null
    } finally {
      looking.value = false
    }
    // 离线库未收录 → 自动回落有道网页解析
    if (!currentWord.value) {
      await lookupOnline(word, 'youdao')
    }
  }

  /** 在线词典查询（有道 / 剑桥网页解析） */
  async function lookupOnline(word: string, source: OnlineDictSource) {
    const w = word.trim()
    if (!w) return
    activeSource.value = source
    onlineResult.value = null
    onlineError.value = ''
    onlineLooking.value = true
    direction.value = 'english'
    keyword.value = w
    try {
      onlineResult.value = await invoke<OnlineDictResult>('dict_online_' + source, { word: w })
    } catch (e: any) {
      onlineError.value = String(e?.message || e)
      onlineResult.value = null
    } finally {
      onlineLooking.value = false
    }
  }

  /** 中→英查询（离线库模糊匹配） */
  async function lookupChinese(text: string) {
    looking.value = true
    lookupError.value = ''
    direction.value = 'chinese'
    currentWord.value = null
    chineseResults.value = []
    keyword.value = text
    activeSource.value = 'offline'
    onlineResult.value = null
    onlineError.value = ''
    try {
      chineseResults.value = await invoke<DictWord[]>('dict_lookup_chinese', {
        keyword: text,
      })
    } catch (e: any) {
      lookupError.value = String(e?.message || e)
      chineseResults.value = []
    } finally {
      looking.value = false
    }
  }

  /** 整句翻译（DeepLX，自动判断中/英方向） */
  async function lookupSentence(text: string) {
    const request = ++sentenceRequest
    sentenceLooking.value = true
    sentenceError.value = ''
    sentenceTranslation.value = ''
    sentenceSourceLang.value = ''
    direction.value = 'sentence'
    currentWord.value = null
    chineseResults.value = []
    onlineResult.value = null
    keyword.value = text
    try {
      const raw = await invoke<SentenceTranslation>('dict_translate_sentence', { text: text.trim() })
      if (request !== sentenceRequest) return
      sentenceTranslation.value = raw.translated
      sentenceSourceLang.value = raw.source_lang
    } catch (e: any) {
      if (request !== sentenceRequest) return
      sentenceError.value = String(e?.message || e)
    } finally {
      if (request === sentenceRequest) sentenceLooking.value = false
    }
  }

  /** 是否为含空格的多词文本（整句/短语） */
  function isPhrase(text: string): boolean {
    return /\s/.test(text.trim())
  }

  /** 根据选中文本自动判断方向并查询：
   *  - 英文单词 → 离线词典（未收录回落有道）
   *  - 中文短词 → 离线中→英
   *  - 短语/整句 → DeepLX 翻译 */
  async function lookupAuto(text: string) {
    const trimmed = text.trim()
    if (!trimmed) return
    const hasCjk = /[\u4e00-\u9fa5]/.test(trimmed)
    if (hasCjk) {
      // 中文短词（无空格且不太长）走离线中→英，其余走 DeepL 译英
      if (!isPhrase(trimmed) && trimmed.length <= 12) {
        await lookupChinese(trimmed)
      } else {
        await lookupSentence(trimmed)
      }
    } else if (isPhrase(trimmed)) {
      await lookupSentence(trimmed)
    } else {
      await lookupEnglish(trimmed)
    }
  }

  function clear() {
    currentWord.value = null
    chineseResults.value = []
    lookupError.value = ''
    keyword.value = ''
    onlineResult.value = null
    onlineError.value = ''
    onlineLooking.value = false
    sentenceTranslation.value = ''
    sentenceError.value = ''
    sentenceLooking.value = false
    activeSource.value = 'offline'
    direction.value = 'english'
  }

  return {
    looking,
    currentWord,
    chineseResults,
    lookupError,
    direction,
    keyword,
    activeSource,
    onlineResult,
    onlineLooking,
    onlineError,
    sentenceTranslation,
    sentenceSourceLang,
    sentenceLooking,
    sentenceError,
    lookupEnglish,
    lookupChinese,
    lookupOnline,
    lookupSentence,
    lookupAuto,
    clear,
  }
})
