/**
 * TTS 朗读引擎：单元队列 + Web Audio 时间线调度（参考 ColorTxt voiceReadLinePlayer）。
 *
 * 播放管线（在线服务商）：
 * - 生产者滚动预取：播放单元 i 时，i+1..i+PRELOAD_UNITS-1 已并发合成并解码（并发上限
 *   MAX_INFLIGHT_SYNTH），缓冲就绪后消费者按 `startAt = max(now, scheduledEnd + 停顿)`
 *   调度 AudioBufferSourceNode —— 片段间零间隙，多音色旁白/对白切换不再等合成。
 * - system / Web Audio 不可用 / 解码失败：回退串行队列（合成 → 播放 → 下一段）。
 *
 * - start(sentences, settings, handlers, voiceOverrides)：任何 start/stop 都使旧队列令牌失效。
 * - jumpTo(index)/restart()：按最近一次 start 的参数从任意单元重启（上一句/下一句/重新合成），
 *   onSentenceStart 的 index 保持绝对下标，播完自然 onFinish(true)（连播语义不变）。
 * - replaceSettings/setVolumeLive：播放中热更新合成参数（下一单元起生效）。
 * - pause/resume：在线走 AudioContext.suspend/resume（时间线冻结，无需重排），系统语音走
 *   speechSynthesis.pause/resume。
 * - onSentenceStart(index, total, text)：单元真正开始播放时触发（高亮/滚动跟随用）。
 * - onFinish(completed)：队列播完（completed=false 表示被中断）。
 */

import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import { QUOTE_OPEN_TO_CLOSE } from './dialogue'

export type TtsProvider = 'edge' | 'system' | 'dashscope' | 'minimax' | 'volcengine' | 'mimo' | 'sapi'

export interface TtsSettings {
  provider: TtsProvider
  voice: string
  /** 倍率 0.5–2.0，1.0 正常 */
  rate: number
  /** 倍率 0.5–2.0，1.0 正常 */
  pitch: number
  /** 0–100 */
  volume: number
  /** 云服务商 API Key（在线服务商必填，缺省回退系统语音） */
  apiKey?: string
  /** MiniMax GroupId（minimax 必填） */
  groupId?: string
  /** 句间停顿毫秒数（1x 语速基准，播放时随语速缩放），0 = 不停顿 */
  sentencePauseMs?: number
}

/** 停顿为 1x 语速下的绝对时长，播放时除以当前语速，使整体节奏随语速缩放。 */
export function scaledPauseMs(configuredMs: number | undefined, rate: number): number {
  if (!Number.isFinite(configuredMs) || !configuredMs || configuredMs <= 0) return 0
  const r = Number.isFinite(rate) && rate > 0 ? rate : 1
  return Math.min(10000, configuredMs / r)
}

/** 缓存 key：服务商 + 音色 + 合成参数 + 文本（文本空白归一）。 */
export function ttsCacheKey(settings: TtsSettings, voice: string, text: string): string {
  return [
    settings.provider,
    voice.trim(),
    settings.rate,
    settings.pitch,
    settings.volume,
    settings.apiKey ?? '',
    settings.groupId ?? '',
    text.replace(/\s+/g, ' ').trim(),
  ].join('\u0001')
}

const AUDIO_CACHE_LIMIT = 64
/** 滚动预取窗口：同时保持就绪/在飞的单元数（参考 ColorTxt EDGE_BUFFER_SIZE） */
const PRELOAD_UNITS = 4
/** 并发合成请求上限，避免触发服务端限流 */
const MAX_INFLIGHT_SYNTH = 3

/** 在线合成结果缓存（原始音频字节）：LRU 限长 + inflight 去重（参考 ColorTxt）。 */
class SynthCache {
  private readonly cache = new Map<string, Uint8Array>()
  private readonly inflight = new Map<string, Promise<Uint8Array>>()

  async getOrFetch(key: string, fetcher: () => Promise<Uint8Array>): Promise<Uint8Array> {
    const hit = this.cache.get(key)
    if (hit) {
      // LRU touch
      this.cache.delete(key)
      this.cache.set(key, hit)
      return hit
    }
    const running = this.inflight.get(key)
    if (running) return running
    const request = fetcher()
      .then((bytes) => {
        this.cache.delete(key)
        this.cache.set(key, bytes)
        while (this.cache.size > AUDIO_CACHE_LIMIT) {
          const oldest = this.cache.keys().next().value
          if (oldest === undefined) break
          this.cache.delete(oldest)
        }
        return bytes
      })
      .finally(() => {
        this.inflight.delete(key)
      })
    this.inflight.set(key, request)
    return request
  }

  /** 清空已完成的缓存（inflight 请求保留，让其自然完成）——「重新合成」用 */
  clear(): void {
    this.cache.clear()
  }
}

export interface TtsHandlers {
  onSentenceStart?: (index: number, total: number, text: string) => void
  onFinish?: (completed: boolean) => void
}

/** start 时保存的启动参数（jumpTo/重新合成时按其重启播放） */
interface TtsLastPayload {
  sentences: string[]
  /** 引用可被 replaceSettings 整体替换：播放循环每单元读同一引用，下一单元即生效 */
  settings: TtsSettings
  handlers: TtsHandlers
  voiceOverrides?: Array<string | undefined>
  options?: TtsQueueOptions
}

export interface TtsQueueOptions {
  /**
   * 逐单元「开始前停顿」（毫秒，1x 语速基准，播放时随语速缩放）。
   * 与 sentences 对齐，缺省项与整体缺省都走 settings.sentencePauseMs。
   * 分音色场景用它让句内片段无缝衔接（0），只在真实句末保留停顿。
   */
  pauseBeforeMs?: Array<number | undefined>
}

/** 句子切分：中英句末标点断句，短句合并、超长硬切。 */
export function splitSentences(text: string, maxLen = 300): string[] {
  const normalized = text.replace(/\s+/g, ' ').trim()
  if (!normalized) return []
  const pieces = normalized
    .split(/(?<=[。！？!?；;…])\s*|(?<=[.!?])\s+(?=["“''(A-Z])/)
    .map((s) => s.trim())
    .filter(Boolean)
  // 逐句返回（朗读以句为单位，不合并短句）
  const out: string[] = []
  for (const piece of pieces) {
    if (piece.length > maxLen) {
      for (let i = 0; i < piece.length; i += maxLen) {
        out.push(piece.slice(i, i + maxLen))
      }
    } else {
      out.push(piece)
    }
  }
  return out
}

export interface SentenceSpan {
  text: string
  start: number
  end: number
}

/** 句末标点（中文；英文 ./!/? 另判后接空白+大写/引号） */
const CN_END_CHARS = new Set(['。', '！', '？', '；', '…', '!', '?', ';'])

/**
 * 句子切分（带原文偏移）：供高亮映射与朗读队列使用。
 * 关键：**引号内的句末标点不切句**——否则一句对白（「"身体不舒服吗？要多喝热水。"」）
 * 会被切成 3 段，产生多余的合成请求与段间停顿（多音色下切换点翻倍、听感变慢）。
 */
export function splitSentenceSpans(text: string, maxLen = 300): SentenceSpan[] {
  const raw: Array<{ start: number; end: number }> = []
  let start = 0
  let i = 0
  /** 当前所处引号的闭符；null = 不在引号内 */
  let quoteCloser: string | null = null
  while (i < text.length) {
    const ch = text[i]
    // 换行是硬边界（段落/行），并重置引号态：避免跨行未闭合引号吞掉后续内容
    if (ch === '\n') {
      if (i > start) raw.push({ start, end: i })
      start = i + 1
      i = i + 1
      quoteCloser = null
      continue
    }
    if (quoteCloser) {
      if (ch === quoteCloser) quoteCloser = null
      i++
      continue
    }
    const closer = QUOTE_OPEN_TO_CLOSE[ch]
    if (closer) {
      quoteCloser = closer
      i++
      continue
    }
    let cut = false
    if (CN_END_CHARS.has(ch)) {
      cut = true
    } else if (ch === '.') {
      // 英文句号：仅在后接空白 + 大写/引号/括号时断句（避免 Mr. / 3.14）
      const rest = text.slice(i + 1)
      cut = i + 1 >= text.length || /^\s+["“''(\[A-Z]/.test(rest)
    }
    if (cut) {
      let next = i + 1
      while (next < text.length && /\s/.test(text[next])) next++
      if (next > start) raw.push({ start, end: i + 1 })
      start = next
      i = next
      continue
    }
    i++
  }
  if (start < text.length) raw.push({ start, end: text.length })

  // trim 边缘空白 + 硬切超长（逐句返回，不合并）
  const spans: SentenceSpan[] = []
  for (const seg of raw) {
    let { start: s, end } = seg
    while (s < end && /\s/.test(text[s])) s++
    while (end > s && /\s/.test(text[end - 1])) end--
    const piece = text.slice(s, end)
    if (!piece) continue
    if (piece.length > maxLen) {
      for (let i = s; i < end; i += maxLen) {
        spans.push({ text: text.slice(i, i + maxLen), start: i, end: Math.min(i + maxLen, end) })
      }
    } else {
      spans.push({ text: piece, start: s, end })
    }
  }
  return spans
}

function bytesToDataUrl(bytes: Uint8Array): string {
  let binary = ''
  for (let i = 0; i < bytes.length; i += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000))
  }
  // 按字节头判型：RIFF → WAV（火山/MiMo/SAPI），否则按 MP3 处理（Edge/DashScope/MiniMax）
  const isWav =
    bytes.length >= 4 &&
    bytes[0] === 0x52 && // R
    bytes[1] === 0x49 && // I
    bytes[2] === 0x46 && // F
    bytes[3] === 0x46 // F
  const mime = isWav ? 'audio/wav' : 'audio/mpeg'
  return `data:${mime};base64,${btoa(binary)}`
}

/** 系统语音可用性（无 speechSynthesis 的测试/浏览器环境直接回退串行） */
function hasSpeechSynthesis(): boolean {
  return typeof window !== 'undefined' && 'speechSynthesis' in window
}

class TtsPlayer {
  /** 播放令牌：start/stop 递增；旧队列检测到令牌变化即中断 */
  private token = 0
  private paused = false
  private audio: HTMLAudioElement | null = null
  private currentUtterance: SpeechSynthesisUtterance | null = null
  private readonly synthCache = new SynthCache()
  /** 最近一次 start 的启动参数（stop 不清空，jumpTo/重新合成需要） */
  private lastPayload: TtsLastPayload | null = null

  // --- Web Audio 时间线（参考 ColorTxt）---
  private audioCtx: AudioContext | null = null
  private gain: GainNode | null = null
  /** 时间线上已排播的结束时间（下一段 startAt 的基准） */
  private scheduledEnd = 0
  private readonly liveSources = new Set<AudioBufferSourceNode>()

  /** idle / playing / paused（响应式） */
  private _state = ref<'idle' | 'playing' | 'paused'>('idle')
  get state(): 'idle' | 'playing' | 'paused' {
    return this._state.value
  }
  currentIndex = ref(-1)
  totalSentences = ref(0)

  async start(
    sentences: string[],
    settings: TtsSettings,
    handlers: TtsHandlers = {},
    /** 逐句音色覆盖（角色分音色）：与 sentences 对齐，空/缺省走 settings.voice */
    voiceOverrides?: Array<string | undefined>,
    options?: TtsQueueOptions,
  ): Promise<void> {
    this.stop(true)
    const myToken = ++this.token
    if (sentences.length === 0) {
      handlers.onFinish?.(false)
      return
    }
    this.lastPayload = { sentences, settings, handlers, voiceOverrides, options }
    this.paused = false
    this._state.value = 'playing'
    this.totalSentences.value = sentences.length
    this.currentIndex.value = -1

    // 在线服务商优先走 Web Audio 时间线；不可用（无 AudioContext/系统语音）时回退串行队列
    const timeline = settings.provider !== 'system' ? this.ensureAudioContext() : null
    if (timeline) {
      await this.runTimeline(timeline, sentences, settings, handlers, voiceOverrides, myToken, options)
    } else {
      await this.runSerial(sentences, settings, handlers, voiceOverrides, myToken, options)
    }

    if (myToken === this.token) {
      this._state.value = 'idle'
      this.currentIndex.value = -1
      handlers.onFinish?.(true)
    }
  }

  /**
   * 从第 index 个单元重启播放（上一句/下一句/重新合成的底层能力）。
   * 复用最近一次 start 的参数（settings/handlers/overrides/options）；
   * onSentenceStart 的 index 仍是绝对下标，高亮/进度映射无需偏移；
   * 播到队尾自然触发 onFinish(true)，连播语义与 start 完全一致。
   */
  async jumpTo(index: number): Promise<void> {
    const payload = this.lastPayload
    if (!payload || payload.sentences.length === 0) return
    const from = Math.max(0, Math.min(index, payload.sentences.length - 1))
    this.stop(true) // 掐断旧时间线/音频；stop 不触发 onFinish
    const myToken = ++this.token
    this.paused = false
    this._state.value = 'playing'
    this.totalSentences.value = payload.sentences.length
    this.currentIndex.value = from

    const timeline = payload.settings.provider !== 'system' ? this.ensureAudioContext() : null
    if (timeline) {
      await this.runTimeline(
        timeline,
        payload.sentences,
        payload.settings,
        payload.handlers,
        payload.voiceOverrides,
        myToken,
        payload.options,
        from,
      )
    } else {
      await this.runSerial(
        payload.sentences,
        payload.settings,
        payload.handlers,
        payload.voiceOverrides,
        myToken,
        payload.options,
        from,
      )
    }

    if (myToken === this.token) {
      this._state.value = 'idle'
      this.currentIndex.value = -1
      payload.handlers.onFinish?.(true)
    }
  }

  /** 从头重播（无历史启动参数时静默忽略） */
  async restart(): Promise<void> {
    await this.jumpTo(0)
  }

  /** 是否有可跳转的历史启动参数 */
  hasPayload(): boolean {
    return this.lastPayload !== null
  }

  /** 播放中热更新合成参数：下一单元起生效（时间线上已排播的音频不受影响） */
  replaceSettings(next: TtsSettings): void {
    if (this.lastPayload) this.lastPayload.settings = next
  }

  /** 音量即时生效（Web Audio 时间线路径；串行路径下一句生效） */
  setVolumeLive(volume: number): void {
    if (this.gain) this.gain.gain.value = Math.min(1, Math.max(0, volume / 100))
  }

  /** 清空合成缓存（「重新合成」：同参数强制重试） */
  clearSynthCache(): void {
    this.synthCache.clear()
  }

  /** 单元 i 开始前的停顿（毫秒）：优先用逐单元覆盖，缺省走设置。from = 播放起点（起点句不引入前置停顿） */
  private pauseBeforeMs(
    i: number,
    settings: TtsSettings,
    options: TtsQueueOptions | undefined,
    from = 0,
  ): number {
    if (i <= 0 || i === from) return 0
    const custom = options?.pauseBeforeMs?.[i]
    if (typeof custom === 'number') return scaledPauseMs(custom, settings.rate)
    return scaledPauseMs(settings.sentencePauseMs, settings.rate)
  }

  /**
   * Web Audio 时间线播放：生产者滚动预取 + 消费者按 scheduledEnd 无缝排播。
   * 关键是"下一段在上一段播完前就已解码就绪"，因此多音色切换不产生等待间隙。
   */
  private async runTimeline(
    ctx: AudioContext,
    sentences: string[],
    settings: TtsSettings,
    handlers: TtsHandlers,
    voiceOverrides: Array<string | undefined> | undefined,
    myToken: number,
    options?: TtsQueueOptions,
    from = 0,
  ): Promise<void> {
    const gain = this.gain
    if (!gain) {
      await this.runSerial(sentences, settings, handlers, voiceOverrides, myToken, options, from)
      return
    }
    try {
      await ctx.resume()
    } catch {
      /* 自动播放策略拒绝时：调度仍会在 resume 后生效 */
    }
    gain.gain.value = Math.min(1, Math.max(0, settings.volume / 100))

    // 生产者：把 i..i+PRELOAD_UNITS-1 的合成提前发出（并发受限），结果放 ready
    const ready = new Map<number, Promise<AudioBuffer | null>>()
    const pending: Array<Promise<unknown>> = []
    let queued = from // 预取起点与播放起点对齐：跳转后不再从 0 合成
    const track = (task: Promise<unknown>): void => {
      const wrapped = task
        .catch(() => {})
        .then(() => {
          const idx = pending.indexOf(wrapped)
          if (idx >= 0) pending.splice(idx, 1)
        })
      pending.push(wrapped)
    }
    const fill = async (until: number): Promise<void> => {
      while (queued <= until && queued < sentences.length) {
        if (myToken !== this.token) return
        if (pending.length >= MAX_INFLIGHT_SYNTH) await Promise.race(pending)
        if (myToken !== this.token) return
        const i = queued++
        const voice = voiceOverrides?.[i] || settings.voice
        const task = this.decodeUnit(sentences[i], settings, voice)
        ready.set(i, task)
        track(task)
      }
    }

    this.scheduledEnd = ctx.currentTime
    for (let i = from; i < sentences.length; i++) {
      if (myToken !== this.token) return
      while (this.paused && myToken === this.token) {
        await this.sleep(80)
        if (myToken !== this.token) return
      }
      // 滚动预取：本段播放期间，窗口内的后续段已在合成/解码
      await fill(Math.min(sentences.length - 1, i + PRELOAD_UNITS - 1))
      if (myToken !== this.token) return

      const buffer = await ready.get(i)
      ready.delete(i)
      if (myToken !== this.token) return

      this.currentIndex.value = i
      handlers.onSentenceStart?.(i, sentences.length, sentences[i])

      if (buffer) {
        // 排播到时间线：上段结束（+停顿）后立即接上，可感知间隙 ≈ 0
        const pauseSec = this.pauseBeforeMs(i, settings, options, from) / 1000
        const startAt = Math.max(ctx.currentTime + 0.01, this.scheduledEnd + pauseSec)
        gain.gain.value = Math.min(1, Math.max(0, settings.volume / 100))
        const src = ctx.createBufferSource()
        src.buffer = buffer
        src.connect(gain)
        this.liveSources.add(src)
        src.onended = () => this.liveSources.delete(src)
        try {
          src.start(startAt)
        } catch {
          this.liveSources.delete(src)
          this.scheduledEnd = ctx.currentTime
          continue
        }
        this.scheduledEnd = startAt + buffer.duration
        await this.waitUntilTime(ctx, this.scheduledEnd, myToken)
      } else {
        // 合成/解码失败：回退系统语音播一遍，并让时间线重新对齐
        try {
          await this.speakSystem(sentences[i], settings)
        } catch {
          /* 彻底失败，跳过该单元 */
        }
        if (myToken !== this.token) return
        this.scheduledEnd = ctx.currentTime
      }
    }
    await this.waitUntilTime(ctx, this.scheduledEnd, myToken)
  }

  /** 等时间线推进到 target（暂停时 AudioContext 冻结，currentTime 不再增长）。 */
  private async waitUntilTime(ctx: AudioContext, target: number, myToken: number): Promise<void> {
    for (;;) {
      if (myToken !== this.token) return
      if (this.paused || ctx.state === 'suspended') {
        await this.sleep(60)
        continue
      }
      const remainMs = (target - ctx.currentTime) * 1000
      if (remainMs <= 0) return
      await this.sleep(Math.min(40, Math.max(5, remainMs)))
    }
  }

  /** 串行队列（system 后端或 Web Audio 不可用时的回退）：合成 → 播放 → 下一段。 */
  private async runSerial(
    sentences: string[],
    settings: TtsSettings,
    handlers: TtsHandlers,
    voiceOverrides: Array<string | undefined> | undefined,
    myToken: number,
    options?: TtsQueueOptions,
    from = 0,
  ): Promise<void> {
    for (let i = from; i < sentences.length; i++) {
      if (myToken !== this.token) return // 被新的 start/stop 中断
      while (this.paused && myToken === this.token) {
        await this.sleep(120)
        if (myToken !== this.token) return
      }
      const voice = voiceOverrides?.[i] || settings.voice

      // 在线后端：先取（或等待预取好的）音频，再进入播放
      let preparedUrl: string | null = null
      if (settings.provider !== 'system') {
        try {
          preparedUrl = await this.prepareSynth(sentences[i], settings, voice)
        } catch {
          preparedUrl = null // 合成失败 → 下方回退系统语音
        }
        if (myToken !== this.token) return
      }

      this.currentIndex.value = i
      handlers.onSentenceStart?.(i, sentences.length, sentences[i])

      // 边播边预合成下一句（inflight 去重，播放前调用会直接命中同一 Promise）
      if (i + 1 < sentences.length && settings.provider !== 'system') {
        const nextVoice = voiceOverrides?.[i + 1] || settings.voice
        this.prepareSynth(sentences[i + 1], settings, nextVoice).catch(() => {})
      }

      try {
        if (preparedUrl) {
          await this.playAudioUrl(preparedUrl, myToken, settings.volume / 100)
        } else {
          await this.speakSystem(sentences[i], settings)
        }
      } catch (e) {
        if (myToken !== this.token) return
        console.error('[ttsPlayer] sentence failed:', e)
        // 合成/播放失败：跳过该句继续（在线后端失败时回退系统语音播一遍）
        if (settings.provider !== 'system') {
          try {
            await this.speakSystem(sentences[i], settings)
          } catch {
            /* 彻底失败，跳过 */
          }
        }
      }

      // 段后停顿 = 下一段的「开始前停顿」（1x 语速基准，随语速缩放）
      if (i < sentences.length - 1 && myToken === this.token) {
        const pause = this.pauseBeforeMs(i + 1, settings, options, from)
        if (pause > 0) {
          await this.sleep(pause)
          if (myToken !== this.token) return
        }
      }
    }
  }

  /** 在线合成单段 → 解码为 AudioBuffer（缓存 + inflight 去重；失败返回 null 走回退） */
  private async decodeUnit(
    text: string,
    settings: TtsSettings,
    voice: string,
  ): Promise<AudioBuffer | null> {
    const ctx = this.ensureAudioContext()
    if (!ctx) return null
    let bytes: Uint8Array
    try {
      bytes = await this.synthBytes(text, settings, voice)
    } catch {
      return null
    }
    try {
      // decodeAudioData 会 detach 传入的 ArrayBuffer，故拷贝一份
      return await ctx.decodeAudioData(bytes.slice().buffer)
    } catch {
      return null
    }
  }

  /** 在线合成单段 → data URL（串行回退路径用） */
  private async prepareSynth(text: string, settings: TtsSettings, voice: string): Promise<string> {
    return bytesToDataUrl(await this.synthBytes(text, settings, voice))
  }

  /** 在线合成单段 → 原始字节（统一走 v3 入口：rate/pitch 传倍率、volume 传 0–100） */
  private synthBytes(text: string, settings: TtsSettings, voice: string): Promise<Uint8Array> {
    const key = ttsCacheKey(settings, voice, text)
    return this.synthCache.getOrFetch(key, async () => {
      const bytes = await invoke<number[]>('tts_synthesize_v3', {
        provider: settings.provider,
        apiKey: settings.apiKey ?? null,
        groupId: settings.groupId ?? null,
        text,
        voice,
        rate: settings.rate,
        pitch: settings.pitch,
        volume: settings.volume,
      })
      return Uint8Array.from(bytes)
    })
  }

  private playAudioUrl(url: string, myToken: number, volumeScale: number): Promise<void> {
    return new Promise((resolve, reject) => {
      if (myToken !== this.token) return resolve()
      const audio = new Audio(url)
      audio.volume = Math.min(1, Math.max(0, volumeScale))
      this.audio = audio
      audio.onended = () => {
        if (this.audio === audio) this.audio = null
        resolve()
      }
      audio.onerror = () => {
        if (this.audio === audio) this.audio = null
        reject(new Error('audio playback failed'))
      }
      // 暂停期间完成加载：加载完若仍在暂停则也暂停
      void audio
        .play()
        .then(() => {
          if (this.paused && this.audio === audio) audio.pause()
        })
        .catch((e) => {
          if (this.audio === audio) this.audio = null
          reject(e)
        })
    })
  }

  private speakSystem(text: string, settings: TtsSettings): Promise<void> {
    return new Promise((resolve, reject) => {
      if (!hasSpeechSynthesis()) {
        reject(new Error('当前环境不支持系统语音'))
        return
      }
      const utter = new SpeechSynthesisUtterance(text)
      utter.voice = window.speechSynthesis.getVoices().find((v) => v.name === settings.voice) ?? null
      utter.lang = utter.voice?.lang ?? (settings.voice.startsWith('zh') ? 'zh-CN' : 'en-US')
      utter.rate = Math.min(10, Math.max(0.1, settings.rate))
      utter.pitch = Math.min(2, Math.max(0, settings.pitch))
      utter.volume = Math.min(1, Math.max(0, settings.volume / 100))
      this.currentUtterance = utter
      let settled = false
      utter.onend = () => {
        if (!settled) {
          settled = true
          if (this.currentUtterance === utter) this.currentUtterance = null
          resolve()
        }
      }
      utter.onerror = (e) => {
        if (!settled && e.error !== 'interrupted' && e.error !== 'canceled') {
          settled = true
          if (this.currentUtterance === utter) this.currentUtterance = null
          reject(new Error('系统语音播放失败'))
        }
      }
      window.speechSynthesis.speak(utter)
    })
  }

  /** 惰性创建 AudioContext + 音量节点（不可用时返回 null → 回退串行） */
  private ensureAudioContext(): AudioContext | null {
    if (this.audioCtx) return this.audioCtx
    if (typeof window === 'undefined') return null
    const Ctor =
      window.AudioContext ??
      (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
    if (!Ctor) return null
    try {
      const ctx = new Ctor()
      const gain = ctx.createGain()
      gain.connect(ctx.destination)
      this.audioCtx = ctx
      this.gain = gain
      return ctx
    } catch {
      return null
    }
  }

  pause(): void {
    if (this._state.value !== 'playing') return
    this.paused = true
    this._state.value = 'paused'
    this.audio?.pause()
    // 时间线冻结：已排播的音频节点保持位置，resume 后继续（无需重排）
    void this.audioCtx?.suspend().catch(() => {})
    window.speechSynthesis?.pause()
  }

  resume(): void {
    if (this._state.value !== 'paused') return
    this.paused = false
    this._state.value = 'playing'
    void this.audioCtx?.resume().catch(() => {})
    this.audio?.play()
    window.speechSynthesis?.resume()
  }

  /** 停止并清空队列（completed=false 的 onFinish 已在 start 循环中处理） */
  stop(silent = false): void {
    this.token += 1
    this.paused = false
    this._state.value = 'idle'
    this.currentIndex.value = -1
    if (this.audio) {
      this.audio.pause()
      this.audio = null
    }
    // 掐断时间线上已排播的音频（再次 start 会重新初始化 scheduledEnd）
    for (const src of this.liveSources) {
      try {
        src.stop()
      } catch {
        /* 已结束 */
      }
    }
    this.liveSources.clear()
    this.scheduledEnd = 0
    if (this.audioCtx?.state === 'suspended') void this.audioCtx.resume().catch(() => {})
    window.speechSynthesis?.cancel()
    this.currentUtterance = null
    if (!silent) {
      // 由 stop 调用方决定是否通知 onFinish；此处仅重置
    }
  }

  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms))
  }
}

/** 全局单例：跨组件共享播放状态 */
export const ttsPlayer = new TtsPlayer()
