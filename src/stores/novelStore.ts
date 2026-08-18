import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Novel, NovelFormData } from '@/types/novel'

export const useNovelStore = defineStore('novel', () => {
  const novels = ref<Novel[]>([])
  const currentNovel = ref<Novel | null>(null)
  const loading = ref(false)

  async function fetchAll() {
    loading.value = true
    try {
      novels.value = await invoke<Novel[]>('get_all_novels')
    } catch (e) {
      console.error('Failed to fetch novels:', e)
    } finally {
      loading.value = false
    }
  }

  async function fetchOne(id: number) {
    loading.value = true
    try {
      // Guard: timeout 8s so loading never gets stuck forever
      const timeout = new Promise<never>((_, reject) =>
        setTimeout(() => reject(new Error('get_novel timeout')), 8000)
      )
      const result = await Promise.race([
        invoke<Novel>('get_novel', { id }),
        timeout,
      ])
      currentNovel.value = result
    } catch (e) {
      console.error('[novelStore] fetchOne failed:', e)
      currentNovel.value = null
      throw e  // 上抛给 page 决定如何呈现错误
    } finally {
      loading.value = false
    }
  }

  async function create(data: NovelFormData) {
    const novel = await invoke<Novel>('create_novel', {
      title: data.title,
      author: data.author,
      category: data.category,
      rawText: data.rawText || '',
      cleanedText: data.cleanedText || data.rawText || '',
      language: data.language || 'zh',
    })
    novels.value.unshift(novel)
    return novel
  }

  async function update(id: number, data: Partial<Novel>) {
    // currentNovel might be null when editing from list page; fall back to list
    const n = currentNovel.value?.id === id
      ? currentNovel.value
      : novels.value.find(n => n.id === id)
    if (!n) return
    const merged = { ...n, ...data }
    await invoke('update_novel', {
      id,
      title: merged.title || '',
      author: merged.author || '',
      category: merged.category || '',
      rawText: merged.rawText || '',
      cleanedText: merged.cleanedText || '',
      isFavorite: merged.isFavorite || false,
      language: merged.language || 'zh',
    })
    if (currentNovel.value?.id === id) {
      currentNovel.value = { ...currentNovel.value, ...merged }
    }
    // Refresh list
    await fetchAll()
  }

  async function remove(id: number) {
    await invoke('delete_novel', { id })
    novels.value = novels.value.filter((n) => n.id !== id)
    if (currentNovel.value?.id === id) {
      currentNovel.value = null
    }
  }

  async function search(query: string) {
    if (!query.trim()) {
      return fetchAll()
    }
    loading.value = true
    try {
      novels.value = await invoke<Novel[]>('search_novels', { query })
    } catch (e) {
      console.error('Failed to search novels:', e)
    } finally {
      loading.value = false
    }
  }

  return { novels, currentNovel, loading, fetchAll, fetchOne, create, update, remove, search }
})
