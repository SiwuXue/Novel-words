import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useNovelStore } from '@/stores/novelStore'
import { useVocabBookStore } from '@/stores/vocabBookStore'
import { useVocabWordStore } from '@/stores/vocabWordStore'
const invoke = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
beforeEach(() => { setActivePinia(createPinia()); invoke.mockReset() })
describe('load failure recovery', () => {
  it('exposes a failed library request and clears it on successful retry', async () => {
    const store = useNovelStore()
    invoke.mockRejectedValueOnce(new Error('offline'))
    await store.fetchAll()
    expect(store.error).toBe('offline')
    expect(store.loading).toBe(false)
    invoke.mockResolvedValueOnce([])
    await store.fetchAll()
    expect(store.error).toBe('')
  })
  it('distinguishes a failed book list from an empty list', async () => {
    invoke.mockRejectedValueOnce(new Error('database locked'))
    const store = useVocabBookStore(); await store.fetchAll()
    expect(store.error).toBe('database locked')
  })
  it('keeps paginated word failures actionable', async () => {
    invoke.mockRejectedValueOnce(new Error('database locked'))
    const store = useVocabWordStore(); await store.fetchPage(1, { offset:0, limit:20 })
    expect(store.error).toBe('database locked')
  })
})
