import { getLocale, t } from '@/i18n'
import type { PresetVocabBook } from '@/types/vocabBook'

const examAliases: Record<string, string> = { cet4: 'cet4-all', CET4luan_1: 'cet4-all', CET4luan_2: 'cet4-all', CET6_2: 'cet6-all', CET6luan_1: 'cet6-all', KaoYan_2: 'kaoyan-all', KaoYanluan_1: 'kaoyan-all', Level4luan_1: 'tem4-all', Level4luan_2: 'tem4-all', Level8_1: 'tem8-all', Level8luan_2: 'tem8-all', ChuZhongluan_2: 'junior-exam-all', GaoZhongluan_2: 'senior-exam-all' }

export function presetName(book: PresetVocabBook): string {
  if (getLocale() === 'zh') return book.name
  return presetKeyName(book.presetKey)
}

function presetKeyName(key: string): string {
  const normalized = examAliases[key] || key
  const nameKey = `preset.name.${normalized}`
  const name = t(nameKey)
  if (name !== nameKey) return name
  const grade = key.match(/^PEP(XiaoXue|ChuZhong)(\d+)_(\d+)$/)
  if (grade) return t(grade[1] === 'XiaoXue' ? 'preset.pepPrimary' : 'preset.pepJunior', { grade: grade[2], term: t(`preset.term${grade[3]}`) })
  const volume = key.match(/^(PEP|BeiShi)GaoZhong_(\d+)$/)
  if (volume) return t(volume[1] === 'PEP' ? 'preset.pepSenior' : 'preset.bnuSenior', { n: volume[2] })
  return key
}

export function presetSourceName(source: string): string {
  if (/^(PEP|BeiShi)/.test(source)) return presetKeyName(source)
  const match = source.match(/^(CET4|CET6|KaoYan|Level4|Level8|IELTS|TOEFL|GRE|GMAT|SAT|BEC|ChuZhong|GaoZhong)(luan)?_(\d+)$/)
  if (!match) return source
  const keys: Record<string, string> = { CET4: 'cet4-all', CET6: 'cet6-all', KaoYan: 'kaoyan-all', Level4: 'tem4-all', Level8: 'tem8-all', IELTS: 'ielts-all', TOEFL: 'toefl-all', GRE: 'gre-all', GMAT: 'gmat-all', SAT: 'sat-all', BEC: 'bec-all', ChuZhong: 'junior-exam-all', GaoZhong: 'senior-exam-all' }
  return t(match[2] ? 'preset.shuffledSource' : 'preset.numberedSource', { name: t(`preset.name.${keys[match[1]]}`), n: match[3] })
}

export function presetDescription(book: PresetVocabBook): string {
  if (getLocale() === 'zh') return book.description
  if (book.category === 'textbook') return t(book.presetKey.startsWith('BeiShi') ? 'preset.bnuDescription' : 'preset.textbookDescription')
  return t('preset.mergedDescription', { n: book.sources?.length || 1 })
}
