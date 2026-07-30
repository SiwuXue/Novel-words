export interface PdfTemplate {
  id: number
  name: string
  paperSize: 'A4' | 'A5' | 'Custom'
  fontFamily: string
  fontSize: number
  lineSpacing: number
  margins: string // JSON: { top, bottom, left, right }
  annotationMode: string // deprecated, kept for backward compat
  templateType: 'intensive' | 'sidebar' | 'recitation' | 'dictation'
  isBuiltin: boolean
  createdAt: string
  updatedAt: string
}

export interface PdfTemplateFormData {
  name: string
  paperSize: 'A4' | 'A5' | 'Custom'
  fontFamily: string
  fontSize: number
  lineSpacing: number
  margins: string
  annotationMode: string
  templateType: 'intensive' | 'sidebar' | 'recitation' | 'dictation'
  isBuiltin: boolean
}

export const TEMPLATE_TYPE_LABELS: Record<string, string> = {
  intensive: '精读版 — 行间注释 + 词汇表',
  sidebar: '侧边注释版 — 右侧单词栏',
  recitation: '背诵专用版 — 左右对照自测',
  dictation: '默写空白版 — 填空默写',
}
