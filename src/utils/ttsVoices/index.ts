/**
 * TTS 音色目录统一入口（数据来源：ColorTxt 项目的音色表，完整移植）。
 *
 * - getVoices(provider)：各服务商音色列表（minimax 为静态兜底，可被动态拉取结果覆盖）。
 * - groupVoices(list)：按分组标签聚合，中文分组在前、英文次之，其余按原顺序。
 * - voiceOptionLabel()：下拉行文案（♀/♂ 前缀 + label）。
 */

import { EDGE_TTS_VOICES, type EdgeTtsVoice } from './edgeVoices'
import {
  DASHSCOPE_TTS_VOICES,
  DASHSCOPE_TTS_VOICE_GROUP_LABELS,
  type DashscopeTtsVoice,
} from './dashscopeVoices'
import {
  VOLCENGINE_TTS_VOICES,
  VOLCENGINE_TTS_VOICE_GROUP_LABELS,
  type VolcengineTtsVoice,
} from './volcengineVoices'
import { MIMO_TTS_VOICES, type MimoTtsVoice } from './mimoVoices'

export type TtsCatalogProvider =
  | 'edge'
  | 'dashscope'
  | 'minimax'
  | 'volcengine'
  | 'mimo'
  | 'sapi'

export interface TtsVoice {
  id: string
  /** 下拉主行文案（不含性别前缀） */
  label: string
  /** 分组标签（如「中文（简体，中国）」「普通话」「中文」） */
  group: string
  gender: 'male' | 'female' | 'unknown'
  /** 下拉副行说明 */
  description?: string
}

/* eslint-disable @typescript-eslint/no-explicit-any */

function fromEdge(v: EdgeTtsVoice): TtsVoice {
  return {
    id: v.id,
    label: v.label,
    group: edgeLocaleGroup(v.lang),
    gender: v.gender,
    description: v.description,
  }
}

function fromDashscope(v: DashscopeTtsVoice): TtsVoice {
  return {
    id: v.id,
    label: v.label,
    group: DASHSCOPE_TTS_VOICE_GROUP_LABELS[v.group] ?? v.group,
    gender: v.gender,
    description: v.description,
  }
}

function fromVolcengine(v: VolcengineTtsVoice): TtsVoice {
  return {
    id: v.id,
    label: v.label,
    group: VOLCENGINE_TTS_VOICE_GROUP_LABELS[v.group] ?? v.group,
    gender: v.gender,
    description: v.description,
  }
}

function fromMimo(v: MimoTtsVoice): TtsVoice {
  return {
    id: v.id,
    label: v.label,
    group: v.group === 'chinese' ? '中文' : '英文',
    gender: v.gender,
    description: v.description,
  }
}

/** Edge locale → 分组名（中文/英文细分地区，其余用 Intl 兜底）。 */
function edgeLocaleGroup(lang: string): string {
  const map: Record<string, string> = {
    'zh-CN': '中文（简体，中国）',
    'zh-HK': '中文（粤语，香港）',
    'zh-TW': '中文（繁体，台湾）',
    'en-US': '英语（美国）',
    'en-GB': '英语（英国）',
    'en-HK': '英语（香港）',
    'en-AU': '英语（澳大利亚）',
    'ja-JP': '日语（日本）',
    'ko-KR': '韩语（韩国）',
  }
  if (map[lang]) return map[lang]
  try {
    const dn = new Intl.DisplayNames(['zh'], { type: 'language' })
    const base = lang.split('-')[0]
    return dn.of(base) ?? lang
  } catch {
    return lang
  }
}

/** MiniMax 静态兜底表（动态 get_voice 失败时使用）。 */
const MINIMAX_FALLBACK: TtsVoice[] = [
  { id: 'female-shaonv', label: '少女', group: '系统音色', gender: 'female', description: '年轻女声' },
  { id: 'female-yujie', label: '御姐', group: '系统音色', gender: 'female', description: '成熟女声' },
  { id: 'female-chengshu', label: '成熟女性', group: '系统音色', gender: 'female', description: '稳重女声' },
  { id: 'female-tianmei', label: '甜美', group: '系统音色', gender: 'female', description: '甜美女声' },
  { id: 'male-qn-qingse', label: '青涩青年', group: '系统音色', gender: 'male', description: '年轻男声' },
  { id: 'male-qn-jingying', label: '精英青年', group: '系统音色', gender: 'male', description: '干练男声' },
  { id: 'male-qn-bada', label: '霸道青年', group: '系统音色', gender: 'male', description: '强势男声' },
  { id: 'presenter_female', label: '女主播', group: '系统音色', gender: 'female', description: '播音女声' },
  { id: 'presenter_male', label: '男主播', group: '系统音色', gender: 'male', description: '播音男声' },
  { id: 'English_captivating_female1', label: 'Captivating Female', group: '系统音色', gender: 'female', description: '英语女声' },
  { id: 'English_captivating_male1', label: 'Captivating Male', group: '系统音色', gender: 'male', description: '英语男声' },
]

/** 各服务商音色目录。 */
export function getVoices(provider: TtsCatalogProvider): TtsVoice[] {
  switch (provider) {
    case 'edge':
      return EDGE_TTS_VOICES.map(fromEdge)
    case 'dashscope':
      return DASHSCOPE_TTS_VOICES.map(fromDashscope)
    case 'volcengine':
      return VOLCENGINE_TTS_VOICES.map(fromVolcengine)
    case 'mimo':
      return MIMO_TTS_VOICES.map(fromMimo)
    case 'minimax':
      return MINIMAX_FALLBACK
    case 'sapi':
      return [] // SAPI 音色动态枚举（sapi_list_voices 命令），无静态表
  }
}

/** 中文分组优先、英文次之、其余保持出现顺序。 */
function groupOrder(label: string): number {
  if (label.startsWith('中文')) return 0
  if (label.startsWith('普通话') || label.startsWith('方言')) return 1
  if (label.startsWith('英语')) return 2
  if (label === '英文' || label.startsWith('美式英语') || label.startsWith('英式英语')) return 2
  return 3
}

export interface VoiceGroup {
  label: string
  voices: TtsVoice[]
}

/** 按分组聚合（保持组内原顺序），组间按中文→普通话/方言→英语→其余排序。 */
export function groupVoices(voices: TtsVoice[]): VoiceGroup[] {
  const map = new Map<string, TtsVoice[]>()
  for (const v of voices) {
    const list = map.get(v.group)
    if (list) list.push(v)
    else map.set(v.group, [v])
  }
  const groups = [...map.entries()].map(([label, list]) => ({ label, voices: list }))
  groups.sort((a, b) => {
    const oa = groupOrder(a.label)
    const ob = groupOrder(b.label)
    if (oa !== ob) return oa - ob
    return a.label.localeCompare(b.label, 'zh')
  })
  return groups
}

/** 下拉行文案：性别前缀 + label。 */
export function voiceOptionLabel(v: TtsVoice): string {
  const prefix = v.gender === 'female' ? '♀ ' : v.gender === 'male' ? '♂ ' : ''
  return `${prefix}${v.label}`
}

/** 按关键字过滤音色（匹配 id/label/描述，大小写不敏感）。 */
export function filterVoices(voices: TtsVoice[], query: string): TtsVoice[] {
  const q = query.trim().toLowerCase()
  if (!q) return voices
  return voices.filter(
    (v) =>
      v.id.toLowerCase().includes(q) ||
      v.label.toLowerCase().includes(q) ||
      (v.description ?? '').toLowerCase().includes(q),
  )
}
