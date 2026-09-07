import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { detectChaptersOffMainThread } from '@/utils/chapterDetectorWorker'
import { detectChaptersInBatches } from '@/utils/chapterDetector'
import { looksLikeHtml } from '@/utils/editorHtml'
import type { Chapter, ChapterSummary } from '@/types/novel'

export const useEditorStore = defineStore('editor', () => {
  const isDirty = ref(false)
  const saving = ref(false)
  const chapterList = ref<Chapter[]>([])
  const activeChapterIndex = ref(0)

  let autosaveTimer: ReturnType<typeof setTimeout> | null = null
  let pendingChapterId: number | null = null
  /** Promise of the in-flight autosave, so flushSave can await it
   *  instead of triggering a second concurrent write. */
  let inFlight: Promise<void> | null = null

  /** Load persisted chapters only; an empty list means the novel needs parsing. */
  async function loadStoredChapters(novelId: number): Promise<Chapter[]> {
    try {
      const summaries = await invoke<ChapterSummary[]>('get_chapter_list', { novelId })
      return summaries.map((chapter) => ({ ...chapter, content: '' }))
    } catch (e) {
      console.error('[editorStore] get_chapters failed:', e)
      return []
    }
  }

  async function loadChapterContent(chapterId: number): Promise<string> {
    const current = chapterList.value.find((chapter) => chapter.id === chapterId)
    if (current?.content) return current.content
    const chapter = await invoke<Chapter>('get_chapter_content', { chapterId })
    const index = chapterList.value.findIndex((item) => item.id === chapterId)
    if (index >= 0) {
      chapterList.value[index] = { ...chapterList.value[index], ...chapter }
    }
    return chapter.content
  }

  function setChapterContent(chapterId: number | null, content: string) {
    if (!chapterId) return
    const chapter = chapterList.value.find((item) => item.id === chapterId)
    if (chapter) chapter.content = content
  }

  /** Load chapters from DB, falling back to chunked client-side detection. */
  async function loadChapters(
    novelId: number,
    text: string,
    storedChapters?: Chapter[],
    onProgress?: (progress: number) => void,
  ) {
    const dbChapters = storedChapters ?? await loadStoredChapters(novelId)
    if (dbChapters.length > 0) {
      chapterList.value = dbChapters
      activeChapterIndex.value = 0
      return
    }

    // Fallback: client-side detection from plain text. Yield between chunks
    // so the editor can paint progress instead of blocking on a huge string.
    // If text is HTML (from previous editor autosave), strip tags first
    const plainText = looksLikeHtml(text) ? stripHtml(text) : text
    let detected: Chapter[]
    try {
      detected = await detectChaptersOffMainThread(plainText, onProgress)
    } catch (e) {
      console.warn('[editorStore] chapter worker failed, fallback to main thread:', e)
      // A Worker can be unavailable in preview/build environments; keep the
      // editor usable with the chunked main-thread implementation.
      detected = await detectChaptersInBatches(plainText, onProgress)
    }
    chapterList.value = detected.map((chapter) => ({
      ...chapter,
      novelId,
    }))
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

  /** Auto-save with 30s debounce. If a previous autosave is still awaiting
   *  Rust, the next call awaits it before scheduling a new write. */
  async function scheduleAutosave(novelId: number, html: string, chapterId: number | null = null) {
    isDirty.value = true
    pendingChapterId = chapterId
    if (autosaveTimer) clearTimeout(autosaveTimer)
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null
      void runAutosave(novelId, html, pendingChapterId)
    }, 30000)
  }

  async function runAutosave(novelId: number, html: string, chapterId: number | null) {
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
        if (chapterId) {
          await invoke('update_chapter_content', { chapterId, content: html })
          setChapterContent(chapterId, html)
        } else {
          await invoke('update_novel_content', { id: novelId, cleanedText: html })
        }
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
  async function flushSave(
    novelId: number,
    html: string,
    chapterId: number | null = pendingChapterId,
  ) {
    if (autosaveTimer) {
      clearTimeout(autosaveTimer)
      autosaveTimer = null
    }
    if (inFlight) {
      try {
        await inFlight
      } catch {
        /* ignore */
      }
    }
    if (isDirty.value) {
      await runAutosave(novelId, html, chapterId)
    }
  }

  function reset() {
    isDirty.value = false
    saving.value = false
    chapterList.value = []
    activeChapterIndex.value = 0
    pendingChapterId = null
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
    loadStoredChapters,
    loadChapterContent,
    setChapterContent,
    loadChapters,
    scheduleAutosave,
    flushSave,
    reset,
  }
})
