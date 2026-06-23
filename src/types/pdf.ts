export interface PdfTemplate {
  id: number
  name: string
  paperSize: 'A4' | 'A5' | 'Custom'
  fontFamily: string
  fontSize: number
  lineSpacing: number
  margins: string // JSON: { top, bottom, left, right }
  annotationMode: 'inline' | 'sidebar' | 'appendix' | 'none'
  createdAt: string
}

export interface PdfTemplateFormData {
  name: string
  paperSize: 'A4' | 'A5' | 'Custom'
  fontFamily: string
  fontSize: number
  lineSpacing: number
  margins: string
  annotationMode: 'inline' | 'sidebar' | 'appendix' | 'none'
}
