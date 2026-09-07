import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useNovelStore } from './novelStore'
import { detectChapters } from '@/utils/chapterDetector'
import { looksLikeHtml } from '@/utils/editorHtml'
import type { Chapter } from '@/types/novel'

export const useEditorStore = defineStore('editor', () => {
  const isDirty = ref(false)
  const saving = ref(false)
  const chapterList = ref<Chapter[]>([])
  const activeChapterIndex = ref(0)

  let autosaveTimer: ReturnType<typeof setTimeout> | null = null
  let chapterRefreshTimer: ReturnType<typeof setTimeout> | null = null
  /** Promise of the in-flight autosave, so flushSave can await it
   *  instead of triggering a second concurrent write. */
  let inFlight: Promise<void> | null = null

  /** Load chapters from DB. Falls back to client-side detection if DB empty. */
  async function loadChapters(novelId: number, text: string) {
    // 1. Try DB first
    try {
      const dbChapters = await invoke<Chapter[]>('get_chapters', { novelId })
      if (dbChapters.length > 0) {
        chapterList.value = dbChapters
        activeChapterIndex.value = 0
        return
      }
    } catch (e) {
      console.error('[editorStore] get_chapters failed:', e)
    }

    // 2. Fallback: client-side detection from plain text
    // If text is HTML (from previous editor autosave), strip tags first
    const plainText = looksLikeHtml(text) ? stripHtml(text) : text
    chapterList.value = detectChapters(plainText)
    activeChapterIndex.value = 0
  }

  /** Strip HTML tags to recover plain text for chapter detection. */
  function stripHtml(html: string): string {
    return html
      .replace(/<br\s*\/?>/gi, '\n')
      .replace(/<\/p>/gi, '\n\n')
      .replace(/<\/h[1-6]>/gi, '\n\n')
      .replace(/<\/li>/gi, '\n')
      .replace(/<\/div>/gi, '\n')
      .replace(/<[^>]*>/g, '')
      .replace(/&lt;/g, '<')
      .replace(/&gt;/g, '>')
      .replace(/&amp;/g, '&')
      .replace(/&quot;/g, '"')
      .replace(/&#39;/g, "'")
      .replace(/\n{3,}/g, '\n\n')
      .trim()
  }

  /** Rebuild chapter content from the current editor buffer without writing
   * to disk yet. This keeps preview/navigation on the latest text. */
  function refreshChaptersFromText(novelId: number, text: string) {
    const plainText = looksLikeHtml(text) ? stripHtml(text) : text
    const detected = detectChapters(plainText)
    const previous = chapterList.value
    chapterList.value = detected.map((chapter, index) => ({
      ...chapter,
      id: previous[index]?.id || 0,
      novelId,
      createdAt: previous[index]?.createdAt || '',
    }))
    activeChapterIndex.value = Math.min(
      activeChapterIndex.value,
      Math.max(0, chapterList.value.length - 1),
    )
  }

  function scheduleChapterRefresh(novelId: number, text: string) {
    if (chapterRefreshTimer) clearTimeout(chapterRefreshTimer)
    chapterRefreshTimer = setTimeout(() => {
      chapterRefreshTimer = null
      refreshChaptersFromText(novelId, text)
    }, 250)
  }

  async function persistChapters(novelId: number, text: string) {
    refreshChaptersFromText(novelId, text)
    await invoke('save_chapters', {
      novelId,
      chapters: chapterList.value,
    })
  }

  /** Auto-save with 30s debounce. If a previous autosave is still awaiting
   *  Rust, the next call awaits it before scheduling a new write. */
  async function scheduleAutosave(novelId: number, html: string) {
    isDirty.value = true
    if (autosaveTimer) clearTimeout(autosaveTimer)
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null
      void runAutosave(novelId, html)
    }, 30000)
  }

  async function runAutosave(novelId: number, html: string) {
    if (inFlight) {
      try {
        await inFlight
      } catch {
        /* previous failure does not block this one */
      }
    }
    saving.value = true
    inFlight = (async () => {
      try {
        const novelStore = useNovelStore()
        await novelStore.update(novelId, { cleanedText: html } as any)
        await persistChapters(novelId, html)
        isDirty.value = false
      } catch (e) {
        console.error('[editorStore] autosave failed:', e)
      } finally {
        saving.value = false
        inFlight = null
      }
    })()
    await inFlight
  }

  /** Cancel pending autosave and flush immediately. Awaits the in-flight
   *  write (if any) so two concurrent update_novel calls never collide. */
  async function flushSave(novelId: number, html: string) {
    if (autosaveTimer) {
      clearTimeout(autosaveTimer)
      autosaveTimer = null
    }
    if (chapterRefreshTimer) {
      clearTimeout(chapterRefreshTimer)
      chapterRefreshTimer = null
    }
    if (inFlight) {
      try {
        await inFlight
      } catch {
        /* ignore */
      }
    }
    if (isDirty.value) {
      await runAutosave(novelId, html)
    }
  }

  function reset() {
    isDirty.value = false
    saving.value = false
    chapterList.value = []
    activeChapterIndex.value = 0
    if (autosaveTimer) {
      clearTimeout(autosaveTimer)
      autosaveTimer = null
    }
    inFlight = null
  }

  return {
    isDirty,
    saving,
    chapterList,
    activeChapterIndex,
    loadChapters,
    refreshChaptersFromText,
    scheduleChapterRefresh,
    scheduleAutosave,
    flushSave,
    reset,
  }
})
