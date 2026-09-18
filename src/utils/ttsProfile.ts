/**
 * 朗读方案（配置方案）辅助：自动命名标签与快照工具。
 *
 * 方案 = 一组可即时套用的朗读参数组合（服务商 + 音色 + 对白检测 + 语速音量等），
 * 持久化在 app_settings（tts_profiles / tts_active_profile），逻辑在 settingsStore。
 */

import { t } from '@/i18n'
import type { TtsProvider } from '@/utils/ttsPlayer'

export interface TtsProfileSettings {
  provider: TtsProvider
  voice: string
  rate: number
  pitch: number
  volume: number
  maleVoice: string
  femaleVoice: string
  quoteStyles: string[]
}

export interface TtsProfile {
  id: string
  name: string
  settings: TtsProfileSettings
}

/** 服务商短名（下拉/标签用，避免设置页的长描述文案） */
const PROVIDER_SHORT: Record<TtsProvider, string> = {
  edge: 'Edge TTS',
  system: '',
  dashscope: 'DashScope',
  minimax: 'MiniMax',
  volcengine: '火山引擎',
  mimo: 'MiMo',
  sapi: 'SAPI5',
}

function providerShort(provider: TtsProvider): string {
  return PROVIDER_SHORT[provider] || provider
}

/** 方案自动标签：`Edge TTS · 旁白/对白` / `系统语音 · 单音色`（仿 ColorTxt） */
export function ttsProfileAutoLabel(p: Pick<TtsProfile, 'settings'>): string {
  const provider = providerShort(p.settings.provider)
  const multi = Boolean(p.settings.maleVoice || p.settings.femaleVoice)
  const mode = multi ? t('settings.ttsProfileModeMulti') : t('settings.ttsProfileModeSingle')
  return `${provider} · ${mode}`
}

/** 由当前朗读设置生成方案快照（校验并补默认值）。 */
export function normalizeProfileSettings(
  raw: Partial<TtsProfileSettings> | undefined | null,
): TtsProfileSettings | null {
  if (!raw || typeof raw !== 'object') return null
  const providers = ['edge', 'system', 'dashscope', 'minimax', 'volcengine', 'mimo', 'sapi']
  const provider = raw.provider
  if (!provider || !providers.includes(provider)) return null
  const num = (v: unknown, lo: number, hi: number, dflt: number) =>
    typeof v === 'number' && Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : dflt
  return {
    provider,
    voice: typeof raw.voice === 'string' ? raw.voice : '',
    rate: num(raw.rate, 0.5, 2, 1),
    pitch: num(raw.pitch, 0.5, 2, 1),
    volume: num(raw.volume, 0, 100, 100),
    maleVoice: typeof raw.maleVoice === 'string' ? raw.maleVoice : '',
    femaleVoice: typeof raw.femaleVoice === 'string' ? raw.femaleVoice : '',
    quoteStyles: Array.isArray(raw.quoteStyles)
      ? raw.quoteStyles.filter((x): x is string => typeof x === 'string').slice(0, 8)
      : [],
  }
}

/** 解析持久化的方案 JSON（容错：非数组/非法项跳过）。 */
export function parseTtsProfiles(json: string | undefined): TtsProfile[] {
  if (!json) return []
  try {
    const arr = JSON.parse(json)
    if (!Array.isArray(arr)) return []
    const out: TtsProfile[] = []
    for (const item of arr) {
      if (!item || typeof item !== 'object') continue
      const id = typeof item.id === 'string' ? item.id : ''
      const name = typeof item.name === 'string' && item.name.trim() ? item.name.trim() : ''
      const settings = normalizeProfileSettings(item.settings)
      if (!id || !name || !settings) continue
      out.push({ id, name, settings })
    }
    return out.slice(0, 50)
  } catch {
    return []
  }
}

/** 生成方案 id（无 crypto.randomUUID 环境回退时间戳+随机）。 */
export function newProfileId(): string {
  try {
    if (typeof crypto !== 'undefined' && crypto.randomUUID) return crypto.randomUUID()
  } catch {
    /* fallthrough */
  }
  return `p-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
}
