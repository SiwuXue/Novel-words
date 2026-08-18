export interface PdfTemplate {
  id: number
  name: string
  paperSize: 'A4' | 'A5' | 'Custom'
  fontFamily: string
  fontSize: number
  lineSpacing: number
  margins: string // JSON: { top, bottom, left, right }
  annotationMode: string
  templateType: 'intensive'
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
  templateType: 'intensive'
  isBuiltin: boolean
}

export const TEMPLATE_TYPE_LABELS: Record<string, string> = {
  intensive: '精读版',
}
