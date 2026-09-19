/**
 * 朗读会话 composable：组装 ttsPlayer 启动参数 + 「上一句/下一句/重新合成」会话逻辑。
 * 供 WordTapReader（逐词模式）与 NovelEditorPage（普通阅读模式）共用。
 *
 * 关键设计：「上一句/下一句」以句子为单位，而非朗读单元（unit/片段）——
 * 对白分音色模式下 currentIndex 可能落在句中片段，直接用会只重播半句；
 * 因此所有跳转都以「句首锚点」（startsSentence 的 unit 下标）为基准。
 */
import { computed } from 'vue'
import {
  ttsPlayer,
  splitSentenceSpans,
  type TtsSettings,
} from '@/utils/ttsPlayer'
import { buildSpeechUnits, type SpeechUnit } from '@/utils/dialogue'
import { useSettingsStore } from '@/stores/settingsStore'

/** 角色分音色上下文（面板回传 + 挂载时自加载的数据） */
export interface TtsDialogueContext {
  charVoices: Record<string, string>
  charGenders: Record<string, 'male' | 'female' | 'unknown'>
  dialogueEnabled: boolean
}

export interface TtsSessionHandlers {
  /** 单元开始播放（unit 为 units 绝对下标取值，高亮/滚动跟随用） */
  onUnitStart?: (unit: SpeechUnit | undefined) => void
  /** 队列播完（completed=false 表示被中断） */
  onFinish?: (completed: boolean) => void
}

/** 当前朗读设置快照（settingsStore → TtsSettings） */
export function currentTtsSettings(): TtsSettings {
  const s = useSettingsStore()
  return {
    provider: s.ttsProvider,
    voice: s.ttsVoice,
    rate: s.ttsRate,
    pitch: s.ttsPitch,
    volume: s.ttsVolume,
    apiKey: s.ttsApiKey(),
    groupId: s.ttsMinimaxGroupId,
    sentencePauseMs: s.ttsPauseSentence,
  }
}

export function useTtsSession() {
  /** 会话内朗读单元（start 时保存，供锚点计算与 onUnitStart 取值） */
  let units: SpeechUnit[] = []
  /** 句首单元下标列表（startsSentence 为 true 的 unit） */
  let sentenceStarts: number[] = []

  async function start(
    text: string,
    dialogue: TtsDialogueContext,
    handlers: TtsSessionHandlers = {},
  ): Promise<void> {
    const settingsStore = useSettingsStore()
    const spans = splitSentenceSpans(text)
    if (spans.length === 0) return
    // 对白分音色：句子内再切旁白/对白片段——旁白走主音色，对白按说话人分音色
    const useDialogue = dialogue.dialogueEnabled && settingsStore.ttsVoiceMode === 'dialogue'
    let sentences: string[]
    let overrides: Array<string | undefined> | undefined
    let pauseBefore: Array<number | undefined> | undefined
    if (useDialogue) {
      const built = buildSpeechUnits(
        spans,
        text,
        dialogue.charVoices,
        dialogue.charGenders,
        { male: settingsStore.ttsMaleVoice, female: settingsStore.ttsFemaleVoice },
        settingsStore.ttsQuoteStyles,
      )
      sentences = built.map((u) => u.text)
      overrides = built.map((u) => u.voice)
      // 句内片段（旁白前缀 ↔ 对白）无缝衔接，只在实际句末保留句间停顿
      pauseBefore = built.map((u, idx) => (idx === 0 || u.startsSentence ? undefined : 0))
      units = built
    } else {
      sentences = spans.map((s) => s.text)
      units = spans.map((s) => ({ text: s.text, start: s.start, end: s.end, startsSentence: true }))
    }
    sentenceStarts = []
    for (let i = 0; i < units.length; i++) {
      if (units[i].startsSentence) sentenceStarts.push(i)
    }

    await ttsPlayer.start(
      sentences,
      currentTtsSettings(),
      {
        onSentenceStart: (i) => handlers.onUnitStart?.(units[i]),
        onFinish: (completed) => handlers.onFinish?.(completed),
      },
      overrides,
      { pauseBeforeMs: pauseBefore },
    )
  }

  function stop(): void {
    ttsPlayer.stop()
    units = []
    sentenceStarts = []
  }

  /** 当前句首锚点：sentenceStarts 中 ≤ currentIndex 的最大值（currentIndex=-1 视为 0） */
  function anchorIndex(): number {
    if (sentenceStarts.length === 0) return 0
    const cur = ttsPlayer.currentIndex.value
    const i = cur < 0 ? 0 : Math.min(cur, units.length - 1)
    let anchor = sentenceStarts[0]
    for (const s of sentenceStarts) {
      if (s <= i) anchor = s
      else break
    }
    return anchor
  }

  const canPrev = computed(
    () => ttsPlayer.state !== 'idle' && anchorIndex() > (sentenceStarts[0] ?? 0),
  )
  const canNext = computed(
    () =>
      ttsPlayer.state !== 'idle' &&
      anchorIndex() < (sentenceStarts[sentenceStarts.length - 1] ?? 0),
  )

  /** 上一句：句首列表中 < 锚点的最大值 */
  function prevSentence(): void {
    const anchor = anchorIndex()
    let target = -1
    for (const s of sentenceStarts) {
      if (s < anchor) target = s
      else break
    }
    if (target >= 0) void ttsPlayer.jumpTo(target)
  }

  /** 下一句：句首列表中 > 锚点的最小值 */
  function nextSentence(): void {
    const anchor = anchorIndex()
    for (const s of sentenceStarts) {
      if (s > anchor) {
        void ttsPlayer.jumpTo(s)
        return
      }
    }
  }

  /** 重新合成：清缓存后从当前句首重播（同参数强制重试） */
  function regenerate(): void {
    ttsPlayer.clearSynthCache()
    void ttsPlayer.jumpTo(anchorIndex())
  }

  return {
    start,
    stop,
    anchorIndex,
    canPrev,
    canNext,
    prevSentence,
    nextSentence,
    regenerate,
  }
}
