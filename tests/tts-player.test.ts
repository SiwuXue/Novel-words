import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  scaledPauseMs,
  splitSentenceSpans,
  splitSentences,
  ttsCacheKey,
  ttsPlayer,
} from '@/utils/ttsPlayer'

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

  it('引号内的句末标点不切句（对白不被切碎）', () => {
    const text = '女生淡淡道："身体不舒服吗？要多喝热水。"'
    const spans = splitSentenceSpans(text)
    // 整句一个 span：旁白前缀 + 完整对白（引号内两个句号不切）
    expect(spans).toHaveLength(1)
    expect(spans[0].text).toBe(text)

    const multi = '他说："第一句。第二句。第三句。"然后离开了。'
    expect(splitSentenceSpans(multi).map((s) => s.text)).toEqual([
      '他说："第一句。第二句。第三句。"然后离开了。',
    ])
  })

  it('引号后的句末标点仍正常切句', () => {
    const text = '"你终于来了。"王小明说。他放下书包。'
    expect(splitSentenceSpans(text).map((s) => s.text)).toEqual([
      '"你终于来了。"王小明说。',
      '他放下书包。',
    ])
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

  it('caches online synthesis: identical sentence synthesizes once', async () => {
    invoke.mockImplementation(async () => new Array(64).fill(65))
    const settings = { provider: 'edge', voice: 'v', rate: 1, pitch: 1, volume: 100 }
    // 第一遍：两句各合成一次 + 预取
    await ttsPlayer.start(['same', 'same'], settings, {})
    const callsAfterFirst = invoke.mock.calls.filter((c) =>
      String(c[0]).startsWith('tts_synthesize'),
    ).length
    expect(callsAfterFirst).toBeGreaterThanOrEqual(1)
    // 第二遍：全部命中缓存，不再发起新合成
    await ttsPlayer.start(['same', 'same'], settings, {})
    const callsAfterSecond = invoke.mock.calls.filter((c) =>
      String(c[0]).startsWith('tts_synthesize'),
    ).length
    expect(callsAfterSecond).toBe(callsAfterFirst)
  })

  it('different voices produce different cache keys', () => {
    const settings = { provider: 'dashscope', voice: '', rate: 1, pitch: 1, volume: 100 }
    const a = ttsCacheKey(settings, 'Cherry', ' 你好。 ')
    const b = ttsCacheKey(settings, 'Ethan', '你好。')
    const c = ttsCacheKey(settings, 'Cherry', '你好。')
    expect(a).not.toBe(b)
    // 文本空白归一 → 同 key
    expect(a).toBe(c)
  })

  it('scaledPauseMs divides by rate and clamps', () => {
    expect(scaledPauseMs(0, 1)).toBe(0)
    expect(scaledPauseMs(undefined, 1)).toBe(0)
    expect(scaledPauseMs(400, 2)).toBe(200)
    expect(scaledPauseMs(400, 0.5)).toBe(800)
    expect(scaledPauseMs(999999, 0.5)).toBeLessThanOrEqual(10000)
  })
})

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  ttsPlayer.stop()
})
