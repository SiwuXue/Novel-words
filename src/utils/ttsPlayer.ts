/**
 * TTS 朗读引擎：句子队列 + 播放令牌。
 *
 * - 服务商：edge / dashscope / minimax / volcengine / mimo / sapi（在线或本地合成，
 *   统一经 Rust `tts_synthesize_v3`）+ system（浏览器 speechSynthesis 兜底）。
 * - start(sentences, settings, handlers)：开始播放队列；任何 start/stop 都会使
 *   旧队列的令牌失效，实现"切章即停"。
 * - pause/resume：在线后端暂停 audio 元素；系统后端暂停 speechSynthesis。
 * - onSentenceStart(index, text)：当前句开始（高亮/滚动跟随用）。
 * - onFinish(completed)：队列播完（completed=false 表示被中断）。
 */

import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

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

/** 在线合成结果缓存（data URL）：LRU 限长 + inflight 去重（参考 ColorTxt）。 */
class SynthCache {
  private readonly cache = new Map<string, string>()
  private readonly inflight = new Map<string, Promise<string>>()

  async getOrFetch(key: string, fetcher: () => Promise<string>): Promise<string> {
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
      .then((url) => {
        this.cache.delete(key)
        this.cache.set(key, url)
        while (this.cache.size > AUDIO_CACHE_LIMIT) {
          const oldest = this.cache.keys().next().value
          if (oldest === undefined) break
          this.cache.delete(oldest)
        }
        return url
      })
      .finally(() => {
        this.inflight.delete(key)
      })
    this.inflight.set(key, request)
    return request
  }
}

export interface TtsHandlers {
  onSentenceStart?: (index: number, total: number, text: string) => void
  onFinish?: (completed: boolean) => void
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

/** 句子切分（带原文偏移）：供高亮映射使用；切分规则与 splitSentences 一致。 */
export function splitSentenceSpans(text: string, maxLen = 300): SentenceSpan[] {
  const boundary = /(?<=[。！？!?；;…])\s*|(?<=[.!?])\s+(?=["“''(A-Z])/g
  const raw: Array<{ start: number; end: number }> = []
  let match: RegExpExecArray | null
  let last = 0
  while ((match = boundary.exec(text)) !== null) {
    if (match.index > last) {
      raw.push({ start: last, end: match.index })
      last = match.index + match[0].length
    }
    if (match[0] === '') boundary.lastIndex++
  }
  if (last < text.length) raw.push({ start: last, end: text.length })

  // trim 边缘空白 + 硬切超长（逐句返回，不合并）
  const spans: SentenceSpan[] = []
  for (const seg of raw) {
    let { start, end } = seg
    while (start < end && /\s/.test(text[start])) start++
    while (end > start && /\s/.test(text[end - 1])) end--
    const piece = text.slice(start, end)
    if (!piece) continue
    if (piece.length > maxLen) {
      for (let i = start; i < end; i += maxLen) {
        spans.push({ text: text.slice(i, i + maxLen), start: i, end: Math.min(i + maxLen, end) })
      }
    } else {
      spans.push({ text: piece, start, end })
    }
  }
  return spans
}

function bytesToDataUrl(bytes: number[]): string {
  let binary = ''
  for (let i = 0; i < bytes.length; i += 0x8000) {
    binary += String.fromCharCode(...bytes.slice(i, i + 0x8000))
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

class TtsPlayer {
  /** 播放令牌：start/stop 递增；旧队列检测到令牌变化即中断 */
  private token = 0
  private paused = false
  private audio: HTMLAudioElement | null = null
  private currentUtterance: SpeechSynthesisUtterance | null = null
  private readonly synthCache = new SynthCache()

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
  ): Promise<void> {
    this.stop(true)
    const myToken = ++this.token
    if (sentences.length === 0) {
      handlers.onFinish?.(false)
      return
    }
    this.paused = false
    this._state.value = 'playing'
    this.totalSentences.value = sentences.length
    this.currentIndex.value = -1

    for (let i = 0; i < sentences.length; i++) {
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
      let prefetch: Promise<string> | null = null
      if (i + 1 < sentences.length && settings.provider !== 'system') {
        const nextVoice = voiceOverrides?.[i + 1] || settings.voice
        prefetch = this.prepareSynth(sentences[i + 1], settings, nextVoice)
        prefetch.catch(() => {}) // 预取失败在真正使用时处理，避免 unhandled rejection
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

      // 句间停顿（1x 语速基准，随语速缩放；暂停时等暂停结束再继续计时判断）
      if (i < sentences.length - 1 && myToken === this.token) {
        const pause = scaledPauseMs(settings.sentencePauseMs, settings.rate)
        if (pause > 0) {
          await this.sleep(pause)
          if (myToken !== this.token) return
        }
      }
    }
    if (myToken === this.token) {
      this._state.value = 'idle'
      this.currentIndex.value = -1
      handlers.onFinish?.(true)
    }
  }

  /** 在线合成单句 → data URL（缓存 + inflight 去重；系统语音不适用） */
  private prepareSynth(text: string, settings: TtsSettings, voice: string): Promise<string> {
    const key = ttsCacheKey(settings, voice, text)
    return this.synthCache.getOrFetch(key, async () => {
      // 统一走 v3 入口：rate/pitch 传原始倍率，volume 传 0–100，偏移换算由 Rust 端按服务商内部完成
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
      return bytesToDataUrl(bytes)
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
      if (!('speechSynthesis' in window)) {
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

  pause(): void {
    if (this._state.value !== 'playing') return
    this.paused = true
    this._state.value = 'paused'
    this.audio?.pause()
    window.speechSynthesis?.pause()
  }

  resume(): void {
    if (this._state.value !== 'paused') return
    this.paused = false
    this._state.value = 'playing'
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
