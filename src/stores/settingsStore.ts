import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { StepNum } from '@/types/pdfSteps'
import { normalizeSteps, serializeSteps } from '@/types/pdfSteps'
import type { SpeechAccent } from '@/utils/speech'
import type { TtsProvider } from '@/utils/ttsPlayer'
import {
  newProfileId,
  normalizeProfileSettings,
  parseTtsProfiles,
  type TtsProfile,
} from '@/utils/ttsProfile'

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
  /** 火山引擎 TTS API Key */
  ttsVolcKey: string
  /** 小米 MiMo TTS API Key */
  ttsMimoKey: string
  /** 句间停顿毫秒（1x 语速基准），0 = 不停顿 */
  ttsPauseSentence: number
  /** 对白角色默认男声音色（空 = 不套用） */
  ttsMaleVoice: string
  /** 对白角色默认女声音色（空 = 不套用） */
  ttsFemaleVoice: string
  /** 计入对白的引号开符集合（英文 " 恒定启用） */
  ttsQuoteStyles: string[]
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
  /** 云服务商密钥：DashScope / MiniMax / 火山引擎 / 小米 MiMo */
  const ttsDashKey = ref('')
  const ttsMinimaxKey = ref('')
  const ttsMinimaxGroupId = ref('')
  const ttsVolcKey = ref('')
  const ttsMimoKey = ref('')
  /** 句间停顿毫秒（1x 语速基准，播放时随语速缩放） */
  const ttsPauseSentence = ref(200)
  /** 对白角色默认男/女声音色（空 = 不套用，走主音色） */
  const ttsMaleVoice = ref('')
  const ttsFemaleVoice = ref('')
  /** 计入对白的引号开符（英文 " 恒定启用，不在此列） */
  const ttsQuoteStyles = ref<string[]>(['“', '‘', '「', '『'])
  /** 朗读方案（配置方案）：多套参数组合，快速切换即生效 */
  const ttsProfiles = ref<TtsProfile[]>([])
  /** 当前激活的方案 id（空 = 未选择） */
  const ttsActiveProfile = ref('')
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
                if (
                  ['edge', 'system', 'dashscope', 'minimax', 'volcengine', 'mimo', 'sapi'].includes(
                    s.value,
                  )
                ) {
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
                if (Number.isFinite(n) && n >= 0.5 && n <= 2) ttsPitch.value = n
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
              case 'tts_volcengine_key':
                ttsVolcKey.value = s.value
                break
              case 'tts_mimo_key':
                ttsMimoKey.value = s.value
                break
              case 'tts_pause_sentence': {
                const n = Number(s.value)
                if (Number.isFinite(n) && n >= 0 && n <= 1200) {
                  ttsPauseSentence.value = Math.round(n)
                }
                break
              }
              case 'tts_male_voice':
                ttsMaleVoice.value = s.value
                break
              case 'tts_female_voice':
                ttsFemaleVoice.value = s.value
                break
              case 'tts_quote_styles': {
                try {
                  const arr = JSON.parse(s.value)
                  const allowed = ['“', '‘', '「', '『']
                  if (Array.isArray(arr)) {
                    const picked = arr.filter((x): x is string =>
                      typeof x === 'string' && allowed.includes(x),
                    )
                    // 空集合视为全部启用（避免用户全关后对白功能静默失效）
                    ttsQuoteStyles.value = picked.length ? picked : [...allowed]
                  }
                } catch {
                  /* 非法 JSON → 保持默认全启用 */
                }
                break
              }
              case 'tts_profiles':
                ttsProfiles.value = parseTtsProfiles(s.value)
                break
              case 'tts_active_profile':
                ttsActiveProfile.value = s.value
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

  /** 当前服务商对应的 API Key（edge/system/sapi 返回空） */
  function ttsApiKey(): string {
    switch (ttsProvider.value) {
      case 'dashscope':
        return ttsDashKey.value
      case 'minimax':
        return ttsMinimaxKey.value
      case 'volcengine':
        return ttsVolcKey.value
      case 'mimo':
        return ttsMimoKey.value
      default:
        return ''
    }
  }

  // ===== 朗读方案（配置方案）管理 =====

  /** 由当前朗读设置生成方案快照 */
  function ttsProfileSnapshot() {
    return {
      provider: ttsProvider.value,
      voice: ttsVoice.value,
      rate: ttsRate.value,
      pitch: ttsPitch.value,
      volume: ttsVolume.value,
      maleVoice: ttsMaleVoice.value,
      femaleVoice: ttsFemaleVoice.value,
      quoteStyles: [...ttsQuoteStyles.value],
    }
  }

  async function persistTtsProfiles(): Promise<void> {
    try {
      await invoke('set_setting', {
        key: 'tts_profiles',
        value: JSON.stringify(ttsProfiles.value),
      })
    } catch (e) {
      console.error('[settingsStore] persist tts_profiles failed:', e)
    }
  }

  async function persistActiveProfile(): Promise<void> {
    try {
      await invoke('set_setting', { key: 'tts_active_profile', value: ttsActiveProfile.value })
    } catch (e) {
      console.error('[settingsStore] persist tts_active_profile failed:', e)
    }
  }

  /** 保存当前设置为 新方案，返回新方案 id（失败返回 null） */
  async function saveTtsProfile(name: string): Promise<string | null> {
    const trimmed = name.trim()
    if (!trimmed) return null
    const profile: TtsProfile = { id: newProfileId(), name: trimmed, settings: ttsProfileSnapshot() }
    ttsProfiles.value = [...ttsProfiles.value, profile]
    ttsActiveProfile.value = profile.id
    await persistTtsProfiles()
    await persistActiveProfile()
    return profile.id
  }

  /** 将当前设置覆盖写入指定方案 */
  async function updateTtsProfile(id: string): Promise<void> {
    const idx = ttsProfiles.value.findIndex((p) => p.id === id)
    if (idx < 0) return
    const next = [...ttsProfiles.value]
    next[idx] = { ...next[idx], settings: ttsProfileSnapshot() }
    ttsProfiles.value = next
    await persistTtsProfiles()
  }

  /** 删除方案；若删除的是激活方案则清空激活态（当前设置保持不变） */
  async function deleteTtsProfile(id: string): Promise<void> {
    ttsProfiles.value = ttsProfiles.value.filter((p) => p.id !== id)
    if (ttsActiveProfile.value === id) ttsActiveProfile.value = ''
    await persistTtsProfiles()
    await persistActiveProfile()
  }

  /** 套用方案：写入各项设置（立即生效并持久化），记住为当前选择 */
  async function applyTtsProfile(id: string): Promise<void> {
    const profile = ttsProfiles.value.find((p) => p.id === id)
    if (!profile) return
    const s = normalizeProfileSettings(profile.settings)
    if (!s) return
    await setTtsSettings({
      ttsProvider: s.provider,
      ttsVoice: s.voice,
      ttsRate: s.rate,
      ttsPitch: s.pitch,
      ttsVolume: s.volume,
      ttsMaleVoice: s.maleVoice,
      ttsFemaleVoice: s.femaleVoice,
      // 空集合视为全启用（与 tts_quote_styles 加载逻辑一致）
      ttsQuoteStyles: s.quoteStyles.length ? s.quoteStyles : ['“', '‘', '「', '『'],
    })
    ttsActiveProfile.value = id
    await persistActiveProfile()
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
      get ttsVolcKey() { return ttsVolcKey.value },
      set ttsVolcKey(v: string) { ttsVolcKey.value = v },
      get ttsMimoKey() { return ttsMimoKey.value },
      set ttsMimoKey(v: string) { ttsMimoKey.value = v },
      get ttsPauseSentence() { return ttsPauseSentence.value },
      set ttsPauseSentence(v: number) { ttsPauseSentence.value = v },
      get ttsMaleVoice() { return ttsMaleVoice.value },
      set ttsMaleVoice(v: string) { ttsMaleVoice.value = v },
      get ttsFemaleVoice() { return ttsFemaleVoice.value },
      set ttsFemaleVoice(v: string) { ttsFemaleVoice.value = v },
      get ttsQuoteStyles() { return ttsQuoteStyles.value },
      set ttsQuoteStyles(v: string[]) { ttsQuoteStyles.value = v },
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
      ttsVolcKey: 'tts_volcengine_key',
      ttsMimoKey: 'tts_mimo_key',
      ttsPauseSentence: 'tts_pause_sentence',
      ttsMaleVoice: 'tts_male_voice',
      ttsFemaleVoice: 'tts_female_voice',
      ttsQuoteStyles: 'tts_quote_styles',
    }
    try {
      for (const [k, v] of Object.entries(patch)) {
        const settingKey = keyMap[k as keyof TtsPrefs]
        if (!settingKey) continue
        const value = Array.isArray(v) ? JSON.stringify(v) : String(v)
        await invoke('set_setting', { key: settingKey, value })
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
    ttsVolcKey,
    ttsMimoKey,
    ttsPauseSentence,
    ttsMaleVoice,
    ttsFemaleVoice,
    ttsQuoteStyles,
    ttsProfiles,
    ttsActiveProfile,
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
    ttsApiKey,
    saveTtsProfile,
    updateTtsProfile,
    deleteTtsProfile,
    applyTtsProfile,
  }
})
