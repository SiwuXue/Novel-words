import { describe, expect, it } from 'vitest'
import {
  buildVoiceOverrides,
  collectSpeakers,
  guessGenders,
  speakerForSpan,
  splitDialogueSegments,
} from '@/utils/dialogue'

describe('splitDialogueSegments', () => {
  it('切分中文对白与旁白', () => {
    const text = '清晨的阳光洒进屋内。“你终于来了。”王小明说。“等你好久了。”李老师说。'
    const segs = splitDialogueSegments(text)
    const dialogues = segs.filter((s) => s.type === 'dialogue')
    expect(dialogues).toHaveLength(2)
    expect(dialogues[0]).toMatchObject({ speaker: '王小明' })
    expect(dialogues[1]).toMatchObject({ speaker: '李老师' })
    // 段落覆盖全文本且不重叠
    expect(segs[0].start).toBe(0)
    expect(segs[segs.length - 1].end).toBe(text.length)
    for (let i = 1; i < segs.length; i++) {
      expect(segs[i].start).toBe(segs[i - 1].end)
    }
  })

  it('对白前置的说话人（XX说："……"）', () => {
    const text = '王小明说道：“今天天气不错，我们去爬山吧。”'
    const segs = splitDialogueSegments(text)
    const dialogue = segs.find((s) => s.type === 'dialogue')
    expect(dialogue?.speaker).toBe('王小明')
  })

  it('英文 said 后缀与前缀', () => {
    const after = '"It is a fine day," said Tom.'
    expect(splitDialogueSegments(after).find((s) => s.type === 'dialogue')?.speaker).toBe('Tom')

    const before = 'Alice asked "Where are you going?" and left.'
    expect(splitDialogueSegments(before).find((s) => s.type === 'dialogue')?.speaker).toBe('Alice')
  })

  it('识别不到说话人时返回 null，引号未闭合不炸', () => {
    const noAttribution = '“好。”他点头。“走吧。”'
    const segs = splitDialogueSegments(noAttribution)
    const dialogues = segs.filter((s) => s.type === 'dialogue')
    expect(dialogues.length).toBeGreaterThanOrEqual(1)

    const unclosed = '他打开门说：“进来吧'
    expect(() => splitDialogueSegments(unclosed)).not.toThrow()
    expect(splitDialogueSegments(unclosed).every((s) => s.type === 'narration')).toBe(true)
  })

  it('支持单引号与直角引号对白（参考 ColorTxt 四种样式）', () => {
    const single = '‘你终于来了。’王小明说。'
    expect(splitDialogueSegments(single).find((s) => s.type === 'dialogue')?.speaker).toBe('王小明')

    const corner = '「今天天气不错。」李老师说。'
    expect(splitDialogueSegments(corner).find((s) => s.type === 'dialogue')?.speaker).toBe('李老师')

    const doubleCorner = '『请进。』门卫喊道。'
    expect(splitDialogueSegments(doubleCorner).find((s) => s.type === 'dialogue')?.speaker).toBe('门卫')
  })

  it('双引号内嵌套单引号/直角引号不误切', () => {
    const text = '王小明说：“他刚才念了‘静夜思’，还写了「床前明月光」。”'
    const segs = splitDialogueSegments(text)
    const dialogues = segs.filter((s) => s.type === 'dialogue')
    expect(dialogues).toHaveLength(1)
    expect(dialogues[0].speaker).toBe('王小明')
    expect(text.slice(dialogues[0].start, dialogues[0].end)).toContain('静夜思')
  })

  it('speakerForSpan：对白句归因，旁白句为 null', () => {
    const text = '“你终于来了。”王小明说。他放下书包。'
    const segs = splitDialogueSegments(text)
    const dia = segs.find((s) => s.type === 'dialogue')!
    // 完全落在对白段内的 span → 王小明
    expect(speakerForSpan(segs, dia.start, dia.end)).toBe('王小明')
    // 落在旁白段内的 span → null
    const nar = segs.find((s) => s.type === 'narration' && s.end > s.start)!
    expect(speakerForSpan(segs, nar.start, nar.end)).toBeNull()
  })

  it('collectSpeakers 汇总并按出现次数排序', () => {
    const text =
      '“A。”王小明说。“B。”王小明又说。“C。”李老师问。“D。”路过的同学喊道。'
    const speakers = collectSpeakers(text)
    expect(speakers[0]).toMatchObject({ name: '王小明', count: 2 })
    expect(speakers.map((s) => s.name)).toContain('李老师')
  })

  it('enabledOpens 过滤引号样式：禁用「」后直角引号不算对白，英文 " 恒定生效', () => {
    const text = '「直角对白。」李老师说。"English quote," Tom said.'
    const enabled = ['“', '‘', '『']
    const segs = splitDialogueSegments(text, enabled)
    const dialogues = segs.filter((s) => s.type === 'dialogue')
    expect(dialogues).toHaveLength(1)
    expect(dialogues[0].speaker).toBe('Tom')
    // 直角引号内容留在旁白里
    expect(text.slice(segs[0].start, segs[0].end)).toContain('直角对白')
    // collectSpeakers 透传同一过滤
    expect(collectSpeakers(text, enabled).map((s) => s.name)).toEqual(['Tom'])
    // 未传 = 全部启用
    expect(splitDialogueSegments(text).filter((s) => s.type === 'dialogue')).toHaveLength(2)
  })
})

describe('buildVoiceOverrides', () => {
  const text = '“你终于来了。”王小明说。他放下书包。'
  const segs = splitDialogueSegments(text)
  const dia = segs.find((s) => s.type === 'dialogue')!
  const spans = [{ start: dia.start, end: dia.end }]

  it('显式指派的音色优先于性别默认', () => {
    const out = buildVoiceOverrides(
      spans,
      text,
      { 王小明: 'custom-voice' },
      { 王小明: 'male' },
      { male: 'male-default', female: 'female-default' },
    )
    expect(out).toEqual(['custom-voice'])
  })

  it('无显式音色时按性别套用默认音色', () => {
    const out = buildVoiceOverrides(spans, text, {}, { 王小明: 'male' }, { male: 'male-default' })
    expect(out).toEqual(['male-default'])

    const out2 = buildVoiceOverrides(
      spans,
      text,
      {},
      { 王小明: 'female' },
      { female: 'female-default' },
    )
    expect(out2).toEqual(['female-default'])
  })

  it('unknown 性别或无默认音色时返回 undefined（走主音色）', () => {
    expect(buildVoiceOverrides(spans, text, {}, { 王小明: 'unknown' }, { male: 'm' })).toEqual([
      undefined,
    ])
    expect(buildVoiceOverrides(spans, text, {}, { 王小明: 'male' }, {})).toEqual([undefined])
    // 旁白句
    const nar = segs.find((s) => s.type === 'narration' && s.end > s.start)!
    expect(
      buildVoiceOverrides(
        [{ start: nar.start, end: nar.end }],
        text,
        {},
        { 王小明: 'male' },
        { male: 'm' },
      ),
    ).toEqual([undefined])
  })
})

describe('guessGenders', () => {
  it('按称呼词判断性别', () => {
    const text = '男生一脸关切的问道："身体不舒服吗？要多喝热水。"\n女生淡淡道："你人还怪好的嘞。"'
    expect(guessGenders(text)).toEqual({ 男生: 'male', 女生: 'female' })
  })

  it('妈妈/小姐等女性称呼', () => {
    const text = '妈妈喊道："回家吃饭！"'
    expect(guessGenders(text)).toEqual({ 妈妈: 'female' })
  })

  it('同时命中男女称呼词或无称呼词时不判定', () => {
    // 「先生」命中 male；纯名字无称呼词不写入
    const text = '王小明说："你好。"'
    expect(guessGenders(text)).toEqual({})
  })
})
