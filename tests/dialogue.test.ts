import { describe, expect, it } from 'vitest'
import {
  buildSpeechUnits,
  buildVoiceOverrides,
  collectCandidates,
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
    const g = guessGenders(text)
    // key 是归因清洗后的名字片段（"男生一脸关切"/"女生淡淡"），断言值即可
    expect(Object.values(g).sort()).toEqual(['female', 'male'])
  })

  it('妈妈等女性称呼', () => {
    const text = '妈妈喊道："回家吃饭！"'
    expect(Object.values(guessGenders(text))).toEqual(['female'])
  })

  it('无称呼词时不判定', () => {
    const text = '王小明说："你好。"'
    expect(guessGenders(text)).toEqual({})
  })
})

describe('归因窗口不跨行', () => {
  it('上句对白不串到下一行的"XX道"', () => {
    const text = '男生一脸关切的问道："身体不舒服吗？要多喝热水。"\n女生淡淡道："你人还怪好的嘞。"'
    const dialogues = splitDialogueSegments(text).filter((s) => s.type === 'dialogue')
    expect(dialogues).toHaveLength(2)
    expect(dialogues[0].speaker).toContain('男')
    expect(dialogues[1].speaker).toContain('女')
  })

  it('多音色试听样例：男生句 male、女生句 female', () => {
    const text =
      '男生一脸关切的问道："身体不舒服吗？要多喝热水。"\n' +
      '女生淡淡道："你人还怪好的嘞。"\n' +
      '"贾君鹏，妈妈喊你回家吃饭！"这时外面传来一道声音。'
    const dialogues = splitDialogueSegments(text).filter((s) => s.type === 'dialogue')
    const ov = buildVoiceOverrides(dialogues, text, {}, guessGenders(text), {
      male: 'm-voice',
      female: 'f-voice',
    })
    expect(ov[0]).toBe('m-voice')
    expect(ov[1]).toBe('f-voice')
  })
})

describe('buildSpeechUnits（旁白/对白片段级分音色）', () => {
  it('句子内切分：旁白前缀走主音色，对白走角色音色', () => {
    const text = '女生淡淡道："你人还怪好的嘞。"'
    const units = buildSpeechUnits(
      [{ start: 0, end: text.length }],
      text,
      {},
      guessGenders(text),
      { male: 'm-voice', female: 'f-voice' },
    )
    // 第一片段 = 旁白前缀「女生淡淡道：」→ 主音色
    expect(units[0].voice).toBeUndefined()
    expect(units[0].text).toContain('女生淡淡道')
    expect(units[0].text).not.toContain('你人还怪好')
    // 第二片段 = 对白 → 女声
    const dia = units.find((u) => u.voice !== undefined)
    expect(dia?.voice).toBe('f-voice')
    expect(dia?.text).toContain('你人还怪好的嘞')
  })

  it('三句试听样例：每句切旁白+对白，贾君鹏句对白无性别走主音色', () => {
    const text =
      '男生一脸关切的问道："身体不舒服吗？要多喝热水。"\n' +
      '女生淡淡道："你人还怪好的嘞。"\n' +
      '"贾君鹏，妈妈喊你回家吃饭！"这时外面传来一道声音。'
    const units = buildSpeechUnits(
      [{ start: 0, end: text.length }],
      text,
      {},
      guessGenders(text),
      { male: 'm-voice', female: 'f-voice' },
    )
    const voices = units.map((u) => u.voice ?? 'main')
    // 男生旁白前缀 / 男声对白 / 女生旁白前缀 / 女声对白
    // 贾君鹏句：对白无性别走主音色，与旁白后缀同音色 → 合并为一个片段
    expect(voices).toEqual(['main', 'm-voice', 'main', 'f-voice', 'main'])
    expect(units[0].text).toContain('男生一脸关切的问道')
    expect(units[0].text).not.toContain('身体不舒服')
    expect(units[4].text).toContain('贾君鹏')
    expect(units[4].text).toContain('这时外面传来一道声音')
  })

  it('显式角色音色优先于性别默认；无角色信息时退化为逐句主音色', () => {
    const text = '王小明说："你好。"'
    const explicit = buildSpeechUnits(
      [{ start: 0, end: text.length }],
      text,
      { 王小明: 'custom-voice' },
      { 王小明: 'male' },
      { male: 'm-voice' },
    )
    expect(explicit.find((u) => u.voice)?.voice).toBe('custom-voice')

    const plain = buildSpeechUnits([{ start: 0, end: text.length }], text, {})
    expect(plain).toHaveLength(1)
    expect(plain[0].voice).toBeUndefined()
    expect(plain[0].text).toBe(text)
  })
})

describe('归因净化（动词表 / 修饰动作词 / 代词群体词过滤）', () => {
  it('动作词被长动词吃掉，名字保持干净', () => {
    expect(collectSpeakers('铁柱父亲摇头道："这孩子，唉。"').map((s) => s.name)).toEqual([
      '铁柱父亲',
    ])
    expect(collectSpeakers('中年汉子摇头道："不成不成。"').map((s) => s.name)).toEqual([
      '中年汉子',
    ])
  })

  it('代词 / 群体词 / 动词残片不产生角色', () => {
    // 「他感慨道」：长动词吃掉"感慨"，剩下代词 → 无角色（不再出现"他感慨"）
    expect(collectSpeakers('他感慨道："人这一辈子啊。"')).toEqual([])
    expect(collectSpeakers('众人笑道："好！"')).toEqual([])
    expect(collectSpeakers('她笑着说："你也来啦。"')).toEqual([])
  })

  it('长状语短语被长度上限拒绝', () => {
    expect(collectSpeakers('船人不在意在铁柱耳边说："别出声。"')).toEqual([])
  })

  it('修饰短语剥离后得到真实称谓（男生一脸关切的问道 → 男生）', () => {
    const text =
      '男生一脸关切的问道："身体不舒服吗？要多喝热水。"\n女生淡淡道："你人还怪好的嘞。"'
    expect(collectSpeakers(text).map((s) => s.name)).toEqual(['男生', '女生'])
  })

  it('正常角色与英文名不受影响', () => {
    expect(collectSpeakers('铁柱说："先坐下吧。"').map((s) => s.name)).toEqual(['铁柱'])
    expect(collectSpeakers('老人叹道："天要黑了。"').map((s) => s.name)).toEqual(['老人'])
    expect(
      splitDialogueSegments('"It is a fine day," said Tom.').find((s) => s.type === 'dialogue')
        ?.speaker,
    ).toBe('Tom')
  })
})

describe('collectCandidates', () => {
  it('按出现次数与名字长度过滤候选，且不改动 collectSpeakers 语义', () => {
    const text = '“A。”王小明说。“B。”王小明又说。“C。”李老师问。'
    expect(collectCandidates(text).map((c) => c.name)).toEqual(['王小明'])
    // 未过滤版本仍保留只出现一次的角色（guessGenders 依赖它推断性别）
    expect(collectSpeakers(text).map((c) => c.name)).toEqual(['王小明', '李老师'])
    // 阈值可覆盖
    expect(collectCandidates(text, undefined, { minCount: 1 }).map((c) => c.name)).toEqual([
      '王小明',
      '李老师',
    ])
  })
})
