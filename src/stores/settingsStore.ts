import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { StepNum } from '@/types/pdfSteps'
import { normalizeSteps, serializeSteps } from '@/types/pdfSteps'
import type { SpeechAccent } from '@/utils/speech'

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

  async function setSpeechAccent(accent: SpeechAccent) {
    speechAccent.value = accent
    try {
      await invoke('set_setting', { key: 'speech_accent', value: accent })
    } catch (e) {
      console.error('[settingsStore] setSpeechAccent failed:', e)
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
  }
})
