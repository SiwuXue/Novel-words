/**
 * 对白切分与说话人识别（TTS 二期·多角色分音色）。
 *
 * - splitDialogueSegments(text)：把整章文本切成旁白/对白段，对白段尽量归因说话人
 *   （对白前后的 "XX说 / XX道 / said XX" 规则；识别不到 speaker 为 null）。
 * - speakerForSpan(segments, start, end)：句子 span → 说话人（按重叠长度归因）。
 * - collectSpeakers(text)：章节内出现的说话人及对白数（角色面板用）。
 *
 * 纯函数无依赖；引号支持四种配对（参考 ColorTxt）：
 * “” 双引号 / ‘’ 单引号 / 「」 直角引号 / 『』 双直角引号，以及英文 " "。
 */

export interface DialogueSegment {
  type: 'narration' | 'dialogue'
  /** 识别到的说话人；narration 恒为 null */
  speaker: string | null
  start: number
  end: number
}

/** 引号配对定义：open → close 字符映射 */
const QUOTE_PAIRS: ReadonlyArray<{ open: string; close: string }> = [
  { open: '“', close: '”' },
  { open: '‘', close: '’' },
  { open: '「', close: '」' },
  { open: '『', close: '』' },
  { open: '"', close: '"' },
]

const ALL_CLOSERS: Record<string, string> = Object.fromEntries(
  QUOTE_PAIRS.map((p) => [p.open, p.close]),
)

/** 开 → 闭 引号映射（供句子切分等复用，保证对白切分与句子边界一致）。 */
export const QUOTE_OPEN_TO_CLOSE: Readonly<Record<string, string>> = ALL_CLOSERS

/** 英文双引号恒定计入对白。 */
const ALWAYS_ON_OPEN = '"'

/** 按启用开符集合构建开→闭映射；未传 = 全部启用。 */
function closersFor(enabledOpens?: ReadonlyArray<string>): Record<string, string> {
  if (!enabledOpens) return ALL_CLOSERS
  const set = new Set(enabledOpens)
  set.add(ALWAYS_ON_OPEN)
  const out: Record<string, string> = {}
  for (const p of QUOTE_PAIRS) {
    if (set.has(p.open)) out[p.open] = p.close
  }
  return out
}

/** 对白切分：按引号对切出对白段，段间为旁白（对白内只找当前引号的闭符）。 */
export function splitDialogueSegments(
  text: string,
  enabledOpens?: ReadonlyArray<string>,
): DialogueSegment[] {
  const CLOSERS = closersFor(enabledOpens)
  const segments: DialogueSegment[] = []
  let narrationStart = 0
  let i = 0
  while (i < text.length) {
    const ch = text[i]
    const closer = CLOSERS[ch]
    if (!closer) {
      i++
      continue
    }
    const close = text.indexOf(closer, i + 1)
    if (close === -1 || close - i > 2000) {
      // 未闭合或异常长的引号：当作普通文本
      i++
      continue
    }
    if (i > narrationStart) {
      segments.push({ type: 'narration', speaker: null, start: narrationStart, end: i })
    }
    segments.push({
      type: 'dialogue',
      speaker: attributeSpeaker(text, i, close + 1),
      start: i,
      end: close + 1,
    })
    i = close + 1
    narrationStart = i
  }
  if (narrationStart < text.length) {
    segments.push({ type: 'narration', speaker: null, start: narrationStart, end: text.length })
  }
  return segments
}

/** 常见说话动词（长词在前，避免短词截断长词）。 */
const CN_VERBS = [
  '说道',
  '问道',
  '喊道',
  '笑道',
  '哭道',
  '吼道',
  '叹道',
  '答道',
  '低声道',
  '轻声道',
  '大声道',
  '低声说',
  '轻声说',
  '大声说',
  '接着说',
  '又说',
  '再说',
  '嘀咕',
  '说',
  '问',
  '喊',
  '叫',
  '答',
  '道',
]
const EN_VERBS = [
  'whispered',
  'exclaimed',
  'shouted',
  'replied',
  'murmured',
  'continued',
  'remarked',
  'yelled',
  'asked',
  'added',
  'said',
]

/** 名字字符：中日韩 + 字母数字 + 间隔号。 */
const NAME_CHARS = '[\\u4e00-\\u9fff\\u3040-\\u30ffA-Za-z0-9·]{1,12}'

/** 归因说话人：优先对白后缀（“……”，王小明说。），其次对白前缀（王小明说：“……”）。 */
function attributeSpeaker(text: string, openIdx: number, endIdx: number): string | null {
  // --- 对白后：结尾引号后的 30 字窗口（不跨行，避免串到下一段的"XX道"） ---
  const after = text.slice(endIdx, endIdx + 30).split('\n')[0]
  for (const verb of CN_VERBS) {
    const re = new RegExp(`^\\s*[，,。、！]?\\s*(${NAME_CHARS}?)${verb}`)
    const m = after.match(re)
    if (m && m[1]) {
      const name = cleanName(m[1])
      if (name) return name
    }
  }
  for (const verb of EN_VERBS) {
    const re = new RegExp(
      `^\\s*,?\\s*${verb}\\s+(?:the\\s+)?([A-Z][A-Za-z]*(?:\\s[A-Z][A-Za-z]*)?)`,
    )
    const m = after.match(re)
    if (m && m[1]) {
      const name = cleanName(m[1])
      if (name) return name
    }
  }
  // 英文对白后缀·名字在前："…," Tom said.
  for (const verb of EN_VERBS) {
    const re = new RegExp(
      `^\\s*,?\\s*([A-Z][A-Za-z]*(?:\\s[A-Z][A-Za-z]*)?)\\s+${verb}\\b`,
    )
    const m = after.match(re)
    if (m && m[1]) {
      const name = cleanName(m[1])
      if (name) return name
    }
  }

  // --- 对白前：开头引号前的 30 字窗口（不跨行，只看当前行；允许结尾残留冒号/引号） ---
  const before = text.slice(Math.max(0, openIdx - 30), openIdx).split('\n').pop() ?? ''
  for (const verb of CN_VERBS) {
    const re = new RegExp(`(${NAME_CHARS})${verb}\\s*[:：“"]?\\s*$`)
    const m = before.match(re)
    if (m && m[1]) {
      const name = cleanName(m[1])
      if (name) return name
    }
  }
  for (const verb of EN_VERBS) {
    const re = new RegExp(
      `([A-Z][A-Za-z]*(?:\\s[A-Z][A-Za-z]*)?)\\s*${verb}\\s*[,;:]?\\s*$`,
    )
    const m = before.match(re)
    if (m && m[1]) {
      const name = cleanName(m[1])
      if (name) return name
    }
  }
  return null
}

/** 清洗名字：去两端装饰符，过滤数字/单字符无意义片段。 */
function cleanName(raw: string): string | null {
  const name = raw.replace(/^[的与和及同对跟让让]+/, '').replace(/[的地了着]+$/, '').trim()
  if (!name || name.length > 12) return null
  if (/^[0-9]+$/.test(name)) return null
  if (!/[\u4e00-\u9fff\u3040-\u30ffA-Za-z]/.test(name)) return null
  return name
}

/** 句子 span → 说话人：取与对白段重叠最长的说话人（重叠不足句子 1/3 视为旁白）。 */
export function speakerForSpan(
  segments: DialogueSegment[],
  start: number,
  end: number,
): string | null {
  let best: DialogueSegment | null = null
  let bestLen = 0
  for (const seg of segments) {
    if (seg.type !== 'dialogue' || !seg.speaker) continue
    const overlap = Math.min(end, seg.end) - Math.max(start, seg.start)
    if (overlap > bestLen) {
      bestLen = overlap
      best = seg
    }
  }
  if (best && bestLen * 3 >= end - start) return best.speaker
  return null
}

/** 章节内说话人统计（角色面板：按对白条数排序）。 */
export function collectSpeakers(
  text: string,
  enabledOpens?: ReadonlyArray<string>,
): Array<{ name: string; count: number }> {
  const counts = new Map<string, number>()
  for (const seg of splitDialogueSegments(text, enabledOpens)) {
    if (seg.type !== 'dialogue' || !seg.speaker) continue
    counts.set(seg.speaker, (counts.get(seg.speaker) ?? 0) + 1)
  }
  return [...counts.entries()]
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count)
}

/** 性别关键词：称呼词判断（「男生」→男、「妈妈」→女）。 */
const MALE_HINTS = ['男', '父', '爸', '哥', '弟', '叔', '伯', '爷', '舅', '郎', '先生', '少爷']
const FEMALE_HINTS = ['女', '母', '妈', '娘', '姐', '妹', '婆', '姨', '姑', '嫂', '夫人', '太太', '小姐']

/**
 * 说话人性别启发式（无角色库时的兜底，如设置页试听）：
 * 名字含男性称呼词 → male，含女性称呼词 → female，两者都含或都不含 → 不写入（走主音色）。
 */
export function guessGenders(
  text: string,
  enabledOpens?: ReadonlyArray<string>,
): Record<string, 'male' | 'female'> {
  const out: Record<string, 'male' | 'female'> = {}
  for (const { name } of collectSpeakers(text, enabledOpens)) {
    const male = MALE_HINTS.some((k) => name.includes(k))
    const female = FEMALE_HINTS.some((k) => name.includes(k))
    if (male === female) continue // 都命中或都没命中：不确定
    out[name] = male ? 'male' : 'female'
  }
  return out
}

export interface GenderDefaults {
  male?: string
  female?: string
}

/**
 * 构建逐句音色覆盖：句子落在某说话人的对白段内时，
 * 显式指派的音色 > 按 gender 套用男/女默认音色 > undefined（走主音色）。
 * 与 ttsPlayer.start 的 voiceOverrides 参数对齐。
 */
export function buildVoiceOverrides(
  spans: Array<{ start: number; end: number }>,
  fullText: string,
  charVoices: Record<string, string>,
  charGenders?: Record<string, 'male' | 'female' | 'unknown'>,
  genderDefaults?: GenderDefaults,
  enabledOpens?: ReadonlyArray<string>,
): Array<string | undefined> {
  const hasCharInfo =
    Object.keys(charVoices).length > 0 ||
    Object.keys(charGenders ?? {}).length > 0 ||
    Boolean(genderDefaults?.male || genderDefaults?.female)
  if (spans.length === 0 || !hasCharInfo) return []
  const segments = splitDialogueSegments(fullText, enabledOpens)
  return spans.map((span) => {
    const speaker = speakerForSpan(segments, span.start, span.end)
    if (!speaker) return undefined
    const explicit = charVoices[speaker]
    if (explicit) return explicit
    const gender = charGenders?.[speaker]
    if (gender === 'male') return genderDefaults?.male || undefined
    if (gender === 'female') return genderDefaults?.female || undefined
    return undefined
  })
}

/** 解析说话人音色：显式指派 > 性别默认 > undefined（主音色）。 */
function resolveSpeakerVoice(
  speaker: string,
  charVoices: Record<string, string>,
  charGenders?: Record<string, 'male' | 'female' | 'unknown'>,
  genderDefaults?: GenderDefaults,
): string | undefined {
  const explicit = charVoices[speaker]
  if (explicit) return explicit
  const gender = charGenders?.[speaker]
  if (gender === 'male') return genderDefaults?.male || undefined
  if (gender === 'female') return genderDefaults?.female || undefined
  return undefined
}

/** 朗读单元：比句子更细——句子内再按引号切旁白/对白片段，各自独立音色。 */
export interface SpeechUnit {
  text: string
  start: number
  end: number
  /** undefined = 主音色（旁白、未识别说话人或未配置对应默认音色） */
  voice?: string
  /** 是否为所在句子的第一个片段（句内片段之间不加句间停顿，衔接更顺） */
  startsSentence?: boolean
}

/**
 * 构建朗读单元：对每个句子 span，按对白段再细分为旁白片段（主音色）与
 * 对白片段（按说话人解析音色）。引号跨句时对白段与多个句子相交，同样正确。
 * 旁白前缀（如「女生淡淡道：」）不再跟随对白音色，真正实现旁白/对白区分。
 * 与 ttsPlayer.start 对齐：sentences = units.map(u => u.text)、
 * voiceOverrides = units.map(u => u.voice)。
 */
export function buildSpeechUnits(
  spans: Array<{ start: number; end: number }>,
  fullText: string,
  charVoices: Record<string, string>,
  charGenders?: Record<string, 'male' | 'female' | 'unknown'>,
  genderDefaults?: GenderDefaults,
  enabledOpens?: ReadonlyArray<string>,
): SpeechUnit[] {
  const hasCharInfo =
    Object.keys(charVoices).length > 0 ||
    Object.keys(charGenders ?? {}).length > 0 ||
    Boolean(genderDefaults?.male || genderDefaults?.female)
  if (spans.length === 0) return []
  // 无任何角色/性别信息：退化为逐句朗读，全部走主音色
  if (!hasCharInfo) {
    return spans.map((s) => ({
      text: fullText.slice(s.start, s.end),
      start: s.start,
      end: s.end,
      startsSentence: true,
    }))
  }
  const segments = splitDialogueSegments(fullText, enabledOpens)
  const units: SpeechUnit[] = []
  for (const span of spans) {
    // 同一句内的相邻同音色片段合并（如相邻两个引号间仅空白）；跨句不合并不影响逐句粒度
    const parts: SpeechUnit[] = []
    const push = (rawStart: number, rawEnd: number, voice: string | undefined): void => {
      let start = rawStart
      let end = rawEnd
      while (start < end && /\s/.test(fullText[start])) start++
      while (end > start && /\s/.test(fullText[end - 1])) end--
      if (start >= end) return
      const last = parts[parts.length - 1]
      if (last && (last.voice ?? undefined) === (voice ?? undefined)) {
        last.end = end
        return
      }
      parts.push({ text: '', start, end, voice, startsSentence: parts.length === 0 })
    }
    for (const seg of segments) {
      const s = Math.max(seg.start, span.start)
      const e = Math.min(seg.end, span.end)
      if (e <= s) continue
      push(
        s,
        e,
        seg.type === 'dialogue' && seg.speaker
          ? resolveSpeakerVoice(seg.speaker, charVoices, charGenders, genderDefaults)
          : undefined,
      )
    }
    units.push(...parts)
  }
  for (const u of units) u.text = fullText.slice(u.start, u.end)
  return units
}
