/**
 * Minimal i18n (no external dependency). zh + en dictionaries, a reactive
 * `t()` usable directly in templates, and a persisted locale switcher.
 */
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import enUs from 'element-plus/es/locale/lang/en'
import zh from './locales/zh'
import en from './locales/en'

export type Locale = 'zh' | 'en'

const STORAGE_KEY = 'app_locale'

const messages: Record<Locale, Record<string, string>> = { zh, en }

const saved = (typeof localStorage !== 'undefined' ? localStorage.getItem(STORAGE_KEY) : null) as
  | Locale
  | null
const locale = ref<Locale>(saved === 'en' ? 'en' : 'zh')

/** Reactive current locale for controls that need to stay in sync. */
export const currentLocale = computed(() => locale.value)

/** Translate a key, interpolating `{name}` placeholders. Falls back to zh → key. */
export function t(key: string, params?: Record<string, string | number>): string {
  let s = messages[locale.value]?.[key] ?? messages.zh[key] ?? key
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      s = s.split(`{${k}}`).join(String(v))
    }
  }
  return s
}

export function getLocale(): Locale {
  return locale.value
}

/** Element Plus locale object, reactive to the app locale. */
export const elementLocale = computed(() => (locale.value === 'en' ? enUs : zhCn))

export async function setLocale(l: Locale) {
  locale.value = l
  try {
    localStorage.setItem(STORAGE_KEY, l)
  } catch {
    /* ignore */
  }
  try {
    await invoke('set_setting', { key: 'locale', value: l })
  } catch {
    /* ignore */
  }
}
