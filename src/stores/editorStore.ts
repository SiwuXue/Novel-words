import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useNovelStore } from './novelStore'
import { detectChapters } from '@/utils/chapterDetector'
import type { Chapter } from '@/types/novel'

export const useEditorStore = defineStore('editor', () => {
  const isDirty = ref(false)
  const saving = ref(false)
  const chapterList = ref<Chapter[]>([])
  const activeChapterIndex = ref(0)

  let autosaveTimer: ReturnType<typeof setTimeout> | null = null
  /** Promise of the in-flight autosave, so flushSave can await it
   *  instead of triggering a second concurrent write. */
  let inFlight: Promise<void> | null = null

  /** Derive chapters client-side; avoids shipping the full text to Rust. */
  function loadChapters(text: string) {
    chapterList.value = detectChapters(text)
    activeChapterIndex.value = 0
  }

  /** Auto-save with 3s debounce. If a previous autosave is still awaiting
   *  Rust, the next call awaits it before scheduling a new write. */
  async function scheduleAutosave(novelId: number, html: string) {
    isDirty.value = true
    if (autosaveTimer) clearTimeout(autosaveTimer)
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null
      void runAutosave(novelId, html)
    }, 3000)
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
    scheduleAutosave,
    flushSave,
    reset,
  }
})