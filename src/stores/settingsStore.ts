import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export const useSettingsStore = defineStore('settings', () => {
  const theme = ref<'light' | 'dark'>('light')
  const defaultExportFolder = ref('')
  const defaultVocabBookId = ref<number | null>(null)
  const loaded = ref(false)

  async function load() {
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
        }
      }
    } catch (e) {
      console.error('[settingsStore] load failed:', e)
    } finally {
      loaded.value = true
    }
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

  return {
    theme,
    defaultExportFolder,
    defaultVocabBookId,
    loaded,
    load,
    setTheme,
    setDefaultExportFolder,
    setDefaultVocabBookId,
  }
})
