import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { PdfTemplate, PdfTemplateFormData } from '@/types/pdf'

export const usePdfTemplateStore = defineStore('pdfTemplate', () => {
  const templates = ref<PdfTemplate[]>([])
  const builtinTemplates = ref<PdfTemplate[]>([])
  const loading = ref(false)

  async function fetchAll() {
    loading.value = true
    try {
      templates.value = await invoke<PdfTemplate[]>('get_all_pdf_templates')
    } catch (e) {
      console.error('[pdfTemplateStore] fetchAll failed:', e)
    } finally {
      loading.value = false
    }
  }

  async function fetchBuiltin() {
    if (builtinTemplates.value.length > 0) return
    try {
      builtinTemplates.value = await invoke<PdfTemplate[]>('get_builtin_templates')
    } catch (e) {
      console.error('[pdfTemplateStore] fetchBuiltin failed:', e)
    }
  }

  async function create(data: PdfTemplateFormData) {
    const tpl = await invoke<PdfTemplate>('create_pdf_template', {
      name: data.name,
      paperSize: data.paperSize,
      fontFamily: data.fontFamily,
      fontSize: data.fontSize,
      lineSpacing: data.lineSpacing,
      margins: data.margins,
      annotationMode: data.annotationMode || 'appendix',
      templateType: data.templateType || 'appendix',
      isBuiltin: data.isBuiltin || false,
    })
    templates.value.unshift(tpl)
    return tpl
  }

  async function update(id: number, data: PdfTemplateFormData) {
    await invoke('update_pdf_template', {
      id,
      name: data.name,
      paperSize: data.paperSize,
      fontFamily: data.fontFamily,
      fontSize: data.fontSize,
      lineSpacing: data.lineSpacing,
      margins: data.margins,
      annotationMode: data.annotationMode || 'appendix',
      templateType: data.templateType || 'appendix',
      isBuiltin: data.isBuiltin || false,
    })
    const idx = templates.value.findIndex((t) => t.id === id)
    if (idx !== -1) {
      templates.value[idx] = {
        ...templates.value[idx],
        name: data.name,
        paperSize: data.paperSize,
        fontFamily: data.fontFamily,
        fontSize: data.fontSize,
        lineSpacing: data.lineSpacing,
        margins: data.margins,
        annotationMode: data.annotationMode,
        templateType: data.templateType,
        isBuiltin: data.isBuiltin || false,
      }
    }
  }

  async function remove(id: number) {
    await invoke('delete_pdf_template', { id })
    templates.value = templates.value.filter((t) => t.id !== id)
  }

  return { templates, builtinTemplates, loading, fetchAll, fetchBuiltin, create, update, remove }
})
