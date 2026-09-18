import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { splitSentenceSpans, splitSentences, ttsPlayer } from '@/utils/ttsPlayer'

const invoke = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('@/utils/speech', () => ({ speakWord: vi.fn() }))

describe('sentence splitting', () => {
  it('splits on Chinese and English sentence endings', () => {
    const spans = splitSentenceSpans('你好，世界！Hello world. Nice day?')
    const texts = spans.map((s) => s.text)
    expect(texts).toEqual(['你好，世界！', 'Hello world.', 'Nice day?'])
  })

  it('keeps offsets that slice back to the sentence text', () => {
    const text = 'First sentence. Second one! 第三句？'
    for (const span of splitSentenceSpans(text)) {
      expect(text.slice(span.start, span.end).trim()).toBe(span.text.trim())
    }
  })

  it('hard-splits overlong sentences without boundaries', () => {
    const long = 'a'.repeat(700)
    const spans = splitSentenceSpans(long)
    expect(spans.length).toBe(3)
    expect(spans.every((s) => s.text.length <= 300)).toBe(true)
  })

  it('splitSentences returns plain strings', () => {
    expect(splitSentences('第一句。第二句！')).toEqual(['第一句。', '第二句！'])
  })
})

describe('ttsPlayer queue', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    invoke.mockReset()
    ttsPlayer.stop()
  })

  it('skips failing sentences and completes the queue', async () => {
    // 无 speechSynthesis 的 jsdom 环境：系统语音逐句失败但队列继续
    const finish = vi.fn()
    const started: number[] = []
    await ttsPlayer.start(
      ['one', 'two'],
      { provider: 'system', voice: '', rate: 1, pitch: 1, volume: 100 },
      {
        onSentenceStart: (i) => started.push(i),
        onFinish: finish,
      },
    )
    expect(started).toEqual([0, 1])
    expect(finish).toHaveBeenCalledWith(true)
    expect(ttsPlayer.state).toBe('idle')
  })

  it('stops immediately when stop is called mid-queue', async () => {
    invoke.mockImplementation(async () => new Array(64).fill(65)) // 模拟 edge 音频字节
    const finish = vi.fn()
    const promise = ttsPlayer.start(
      ['a', 'b', 'c'],
      { provider: 'edge', voice: 'zh-CN-XiaoxiaoNeural', rate: 1, pitch: 1, volume: 100 },
      { onFinish: finish },
    )
    ttsPlayer.stop()
    await promise
    expect(finish).not.toHaveBeenCalled()
    expect(ttsPlayer.state).toBe('idle')
  })

  it('invalidates the previous queue when a new start begins', async () => {
    const firstFinish = vi.fn()
    const p1 = ttsPlayer.start(
      ['old'],
      { provider: 'system', voice: '', rate: 1, pitch: 1, volume: 100 },
      { onFinish: firstFinish },
    )
    await ttsPlayer.start(
      ['new'],
      { provider: 'system', voice: '', rate: 1, pitch: 1, volume: 100 },
      { onFinish: vi.fn() },
    )
    await p1
    expect(firstFinish).not.toHaveBeenCalled()
  })
})

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  ttsPlayer.stop()
})
