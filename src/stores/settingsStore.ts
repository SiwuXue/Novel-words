import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { StepNum } from '@/types/pdfSteps'
import { normalizeSteps, serializeSteps } from '@/types/pdfSteps'
import type { SpeechAccent } from '@/utils/speech'
import type { TtsProvider } from '@/utils/ttsPlayer'

interface TtsPrefs {
  ttsProvider: TtsProvider
  ttsVoice: string
  ttsRate: number
  ttsPitch: number
  ttsVolume: number
  ttsAutoNext: boolean
  ttsDashKey: string
  ttsMinimaxKey: string
  ttsMinimaxGroupId: string
}

export type PdfBackground = 'grid' | 'dots' | 'none'

export type AutoBackup = 'off' | 'daily' | 'weekly' | 'monthly'

export const useSettingsStore = defineStore('settings', () => {
  const theme = ref<'light' | 'dark'>('light')
  const defaultExportFolder = ref('')
  const defaultVocabBookId = ref<number | null>(null)
  const pdfIntensiveSteps = ref<StepNum[]>([1, 2, 3])
  const pdfBackground = ref<PdfBackground>('grid')
  const autoBackup = ref<AutoBackup>('weekly')
  const reviewDailyGoal = ref<number>(20)
  const speechAccent = ref<SpeechAccent>('us')
  /** 每日新词上限（0 = 不限），阅读中标记新词超过时提醒 */
  const dailyNewWordLimit = ref(0)
  /** 朗读：服务商、音色、语速/音调倍率、音量、自动连播、云 Key */
  const ttsProvider = ref<TtsProvider>('system')
  const ttsVoice = ref('zh-CN-XiaoxiaoNeural')
  const ttsRate = ref(1)
  const ttsPitch = ref(1)
  const ttsVolume = ref(100)
  const ttsAutoNext = ref(true)
  /** 云服务商密钥：DashScope / MiniMax */
  const ttsDashKey = ref('')
  const ttsMinimaxKey = ref('')
  const ttsMinimaxGroupId = ref('')
  /** DeepLX 翻译端点（空串 = 使用 Rust 端默认公共实例） */
  const deeplEndpoint = ref('')
  const loaded = ref(false)
  let loadPromise: Promise<void> | null = null

  async function load() {
    if (loaded.value) return
    if (!loadPromise) {
      loadPromise = (async () => {
        try {
          const settings = await invoke<Array<{ key: string; value: string }>>(
            'get_all_settings',
          )
          for (const s of settings) {
            switch (s.key) {
              case 'theme':
                if (s.value === 'dark' || s.value === 'light') {
                  theme.value = s.value
                }
                break
              case 'default_export_folder':
                defaultExportFolder.value = s.value
                break
              case 'default_vocab_book_id': {
                const n = Number(s.value)
                defaultVocabBookId.value = Number.isFinite(n) && n > 0 ? n : null
                break
              }
              case 'pdf_intensive_steps': {
                try {
                  pdfIntensiveSteps.value = normalizeSteps(JSON.parse(s.value))
                } catch {
                  /* 非法 JSON → 保持默认 [1,2,3] */
                }
                break
              }
              case 'pdf_background':
                if (s.value === 'grid' || s.value === 'dots' || s.value === 'none') {
                  pdfBackground.value = s.value
                }
                break
              case 'auto_backup':
                if (s.value === 'off' || s.value === 'daily' || s.value === 'weekly' || s.value === 'monthly') {
                  autoBackup.value = s.value
                }
                break
              case 'review_daily_goal': {
                const n = Number(s.value)
                if (Number.isFinite(n) && n > 0) reviewDailyGoal.value = Math.floor(n)
                break
              }
              case 'speech_accent':
                if (s.value === 'uk' || s.value === 'us') {
                  speechAccent.value = s.value
                }
                break
              case 'daily_new_word_limit': {
                const n = Number(s.value)
                if (Number.isFinite(n) && n >= 0) dailyNewWordLimit.value = Math.floor(n)
                break
              }
              case 'tts_provider':
                if (['edge', 'system', 'dashscope', 'minimax'].includes(s.value)) {
                  ttsProvider.value = s.value as TtsProvider
                }
                break
              case 'tts_voice':
                if (s.value) ttsVoice.value = s.value
                break
              case 'tts_rate': {
                const n = Number(s.value)
                if (Number.isFinite(n) && n >= 0.5 && n <= 2) ttsRate.value = n
                break
              }
              case 'tts_pitch': {
                const n = Number(s.value)
                if (Number.isFinite(n) && n >= 0.5 && n <= 1.5) ttsPitch.value = n
                break
              }
              case 'tts_volume': {
                const n = Number(s.value)
                if (Number.isFinite(n) && n >= 0 && n <= 100) ttsVolume.value = n
                break
              }
              case 'tts_auto_next':
                ttsAutoNext.value = s.value !== 'false'
                break
              case 'tts_dashscope_key':
                ttsDashKey.value = s.value
                break
              case 'tts_minimax_key':
                ttsMinimaxKey.value = s.value
                break
              case 'tts_minimax_group_id':
                ttsMinimaxGroupId.value = s.value
                break
              case 'deepl_endpoint':
                deeplEndpoint.value = s.value
                break
            }
          }
        } catch (e) {
          console.error('[settingsStore] load failed:', e)
        } finally {
          loaded.value = true
        }
      })()
    }
    await loadPromise
  }

  /** Apply theme to DOM, persist to localStorage + DB. */
  async function setTheme(t: 'light' | 'dark') {
    theme.value = t
    const html = document.documentElement
    if (t === 'dark') {
      html.classList.add('dark')
    } else {
      html.classList.remove('dark')
    }
    localStorage.setItem('theme', t)
    try {
      await invoke('set_setting', { key: 'theme', value: t })
    } catch (e) {
      console.error('[settingsStore] setTheme failed:', e)
    }
  }

  async function setDefaultExportFolder(path: string) {
    defaultExportFolder.value = path
    try {
      await invoke('set_setting', { key: 'default_export_folder', value: path })
    } catch (e) {
      console.error('[settingsStore] setDefaultExportFolder failed:', e)
    }
  }

  async function setDefaultVocabBookId(id: number | null) {
    defaultVocabBookId.value = id
    try {
      await invoke('set_setting', {
        key: 'default_vocab_book_id',
        value: id != null ? String(id) : '',
      })
    } catch (e) {
      console.error('[settingsStore] setDefaultVocabBookId failed:', e)
    }
  }

  async function setPdfIntensiveSteps(steps: StepNum[]) {
    const normalized = normalizeSteps(steps)
    pdfIntensiveSteps.value = normalized
    try {
      await invoke('set_setting', {
        key: 'pdf_intensive_steps',
        value: serializeSteps(normalized),
      })
    } catch (e) {
      console.error('[settingsStore] setPdfIntensiveSteps failed:', e)
    }
  }

  async function setPdfBackground(bg: PdfBackground) {
    pdfBackground.value = bg
    try {
      await invoke('set_setting', { key: 'pdf_background', value: bg })
    } catch (e) {
      console.error('[settingsStore] setPdfBackground failed:', e)
    }
  }

  async function setAutoBackup(v: AutoBackup) {
    autoBackup.value = v
    try {
      await invoke('set_setting', { key: 'auto_backup', value: v })
    } catch (e) {
      console.error('[settingsStore] setAutoBackup failed:', e)
    }
  }

  async function setReviewDailyGoal(n: number) {
    const v = Math.max(1, Math.floor(n))
    reviewDailyGoal.value = v
    try {
      await invoke('set_setting', { key: 'review_daily_goal', value: String(v) })
    } catch (e) {
      console.error('[settingsStore] setReviewDailyGoal failed:', e)
    }
  }

  function ttsPrefsTarget(): TtsPrefs {
    return {
      get ttsProvider() { return ttsProvider.value },
      set ttsProvider(v: TtsProvider) { ttsProvider.value = v },
      get ttsVoice() { return ttsVoice.value },
      set ttsVoice(v: string) { ttsVoice.value = v },
      get ttsRate() { return ttsRate.value },
      set ttsRate(v: number) { ttsRate.value = v },
      get ttsPitch() { return ttsPitch.value },
      set ttsPitch(v: number) { ttsPitch.value = v },
      get ttsVolume() { return ttsVolume.value },
      set ttsVolume(v: number) { ttsVolume.value = v },
      get ttsAutoNext() { return ttsAutoNext.value },
      set ttsAutoNext(v: boolean) { ttsAutoNext.value = v },
      get ttsDashKey() { return ttsDashKey.value },
      set ttsDashKey(v: string) { ttsDashKey.value = v },
      get ttsMinimaxKey() { return ttsMinimaxKey.value },
      set ttsMinimaxKey(v: string) { ttsMinimaxKey.value = v },
      get ttsMinimaxGroupId() { return ttsMinimaxGroupId.value },
      set ttsMinimaxGroupId(v: string) { ttsMinimaxGroupId.value = v },
    }
  }

  async function setTtsSettings(patch: Partial<TtsPrefs>) {
    Object.assign(ttsPrefsTarget(), patch)
    const keyMap: Partial<Record<keyof TtsPrefs, string>> = {
      ttsProvider: 'tts_provider',
      ttsVoice: 'tts_voice',
      ttsRate: 'tts_rate',
      ttsPitch: 'tts_pitch',
      ttsVolume: 'tts_volume',
      ttsAutoNext: 'tts_auto_next',
      ttsDashKey: 'tts_dashscope_key',
      ttsMinimaxKey: 'tts_minimax_key',
      ttsMinimaxGroupId: 'tts_minimax_group_id',
    }
    try {
      for (const [k, v] of Object.entries(patch)) {
        const settingKey = keyMap[k as keyof TtsPrefs]
        if (settingKey) await invoke('set_setting', { key: settingKey, value: String(v) })
      }
    } catch (e) {
      console.error('[settingsStore] setTtsSettings failed:', e)
    }
  }

  async function setDailyNewWordLimit(n: number) {
    dailyNewWordLimit.value = n
    try {
      await invoke('set_setting', { key: 'daily_new_word_limit', value: String(n) })
    } catch (e) {
      console.error('[settingsStore] setDailyNewWordLimit failed:', e)
    }
  }

  async function setSpeechAccent(accent: SpeechAccent) {
    speechAccent.value = accent
    try {
      await invoke('set_setting', { key: 'speech_accent', value: accent })
    } catch (e) {
      console.error('[settingsStore] setSpeechAccent failed:', e)
    }
  }

  async function setDeeplEndpoint(endpoint: string) {
    deeplEndpoint.value = endpoint
    try {
      await invoke('set_setting', { key: 'deepl_endpoint', value: endpoint })
    } catch (e) {
      console.error('[settingsStore] setDeeplEndpoint failed:', e)
    }
  }

  return {
    theme,
    defaultExportFolder,
    defaultVocabBookId,
    pdfIntensiveSteps,
    pdfBackground,
    autoBackup,
    reviewDailyGoal,
    speechAccent,
    dailyNewWordLimit,
    ttsProvider,
    ttsVoice,
    ttsRate,
    ttsPitch,
    ttsVolume,
    ttsAutoNext,
    ttsDashKey,
    ttsMinimaxKey,
    ttsMinimaxGroupId,
    deeplEndpoint,
    loaded,
    load,
    setTheme,
    setDefaultExportFolder,
    setDefaultVocabBookId,
    setPdfIntensiveSteps,
    setPdfBackground,
    setAutoBackup,
    setReviewDailyGoal,
    setSpeechAccent,
    setDeeplEndpoint,
    setDailyNewWordLimit,
    setTtsSettings,
  }
})
