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

/** 查询方向：英→中 或 中→英 */
export type LookupDirection = 'english' | 'chinese'

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

  /** 英→中查询 */
  async function lookupEnglish(word: string) {
    looking.value = true
    lookupError.value = ''
    direction.value = 'english'
    currentWord.value = null
    chineseResults.value = []
    keyword.value = word
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
  }

  /** 中→英查询 */
  async function lookupChinese(text: string) {
    looking.value = true
    lookupError.value = ''
    direction.value = 'chinese'
    currentWord.value = null
    chineseResults.value = []
    keyword.value = text
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

  /** 根据选中文本自动判断方向并查询 */
  async function lookupAuto(text: string) {
    const trimmed = text.trim()
    if (!trimmed) return
    // 含中文字符 → 中→英
    if (/[\u4e00-\u9fa5]/.test(trimmed)) {
      await lookupChinese(trimmed)
    } else {
      await lookupEnglish(trimmed)
    }
  }

  function clear() {
    currentWord.value = null
    chineseResults.value = []
    lookupError.value = ''
    keyword.value = ''
  }

  return {
    looking,
    currentWord,
    chineseResults,
    lookupError,
    direction,
    keyword,
    lookupEnglish,
    lookupChinese,
    lookupAuto,
    clear,
  }
})
