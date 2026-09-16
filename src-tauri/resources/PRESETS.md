# 离线预设词汇资源

`presets.zip` 由用户提供的 `dict/book` 目录中 67 个 ZIP 转换而成，包含 13 套合并考试词库和 27 套教材单册。转换使用 Python 标准库，不依赖网络或额外安装包。原始 ZIP 内的 `.json` 文件实际为 JSON Lines。

## 重建与验证

在项目根目录运行，Python 版本要求 3.8 或以上：

```powershell
python scripts/prepare_presets.py --source "<包含67个源ZIP的目录>"
python -m unittest discover -s scripts -p test_prepare_presets.py -v
```

输出默认写入 `src-tauri/resources/presets.zip`，可用 `--output "<输出ZIP路径>"` 修改。源目录默认是项目内的 `DictionaryData/book`，脚本未写死个人电脑路径。源目录必须包含全部 67 个预期来源；缺失、重复、未知来源、来源标识不符、空词形或无效 JSON 会使构建失败。源 ZIP 直接读取，不解压到文件系统。

## 归并规则

- 考试来源按 `CET4`、`CET6`、`KaoYan`、`Level4`、`Level8`、`IELTS`、`TOEFL`、`GRE`、`GMAT`、`SAT`、`BEC`、`ChuZhong`、`GaoZhong` 合并，`luan` 版归入同一考试。
- 教材保持源标识为预设 key。小学使用年级上下册，初中包含七、八年级上下册和九年级上册；人教版高中包含第 1 至 11 册，北师大高中仅第 2、3、7 册。名称和说明不宣称未提供的册次完整。
- `wordKey` 只做去首尾空白、转小写、合并连续空白、将 `’` 和 `‘` 替换为 `'`。保留其他标点、短语及原有屈折词形，不词形还原、不按 `wordRank` 截断。
- 主释义合并各来源 `trans` 中所有不同的 `(pos, tranCn)` 对，按词性和释义文本稳定排序，格式保持为 `POS. 中文`。源词性标签只去掉结尾已有的点；没有词性时保留原释义。`syno`、`relWord` 的近义词和同根词释义不混入本词主释义。
- 可选字段按来源完整度选择，评分依次比较不同释义数、释义总长度、双语例句是否存在、音标是否存在、记忆法长度、搭配数、例句总长度；同分按来源 key、规范化内容的 JSON 文本排序。展示词形取最佳来源，音标保持应用原有的美音、英音、通用音标优先级。
- 例句优先双语，再比较来源完整度和例句长度。记忆法优先较长文本，再比较来源完整度；`memoryTag` 保留该记忆法的描述，缺描述时使用记忆法文本。
- 搭配汇总全部来源，按英文规范化文本和中文释义去重，稳定排序后最多保留 5 条。保留 `\n【记忆】...` 和 `\n【搭配】\n  · ...` 格式。
- 每词 `sources` 记录该词实际出现的全部源 key，每本 `sources` 记录其全部输入来源。每词来源不因展示字段的选择而丢失。
- JSON key、来源列表、单词、ZIP 成员排序固定；ZIP 时间戳固定为 `1980-01-01 00:00:00`，权限和压缩等级固定。同一 Python/zlib 环境中重复构建的字节结果一致。

## ZIP 格式

ZIP 包含 `manifest.json` 和 40 个 `data/<key>.jsonl`。文本为 UTF-8，每个 JSON Lines 文件行末有换行。

```json
{
  "version": 1,
  "books": [{
    "key": "cet6-all",
    "name": "大学英语六级",
    "description": "合并所提供的4个来源版本，按规范化词形去重，保留不同词性、释义与来源。",
    "category": "university",
    "wordCount": 3992,
    "file": "data/cet6-all.jsonl",
    "sources": ["CET6_1", "CET6_2", "CET6_3", "CET6luan_1"]
  }]
}
```

每个词条使用以下字段，字符串字段缺数据时为空字符串；主释义缺失时为 `（暂无释义）`：

```json
{"word":"look up","wordKey":"look up","definition":"v. 查找","phonetic":"","exampleSentence":"","memoryTag":"","sources":["CET6_1"]}
```

`category` 仅使用 `university`、`international`、`school`、`textbook`。`wordCount` 来自该文件实际去重行数。

## 当前资源核验

当前 67 个源文件共 148,427 条原始记录。转换后共 80,206 条预设内成员关系，跨全部预设共有 23,302 个不同 `wordKey`。ZIP 共 41 个成员，压缩后 12,757,541 字节，成员解压总量 35,932,722 字节。

SHA-256：`fbf130415c149cf416381b80a9f6f68b076d83f89fb227a67b45bbe552b27650`

| 预设 key | 真实去重词数 |
| --- | ---: |
| cet4-all | 4,544 |
| cet6-all | 3,992 |
| kaoyan-all | 5,047 |
| tem4-all | 4,340 |
| tem8-all | 12,410 |
| ielts-all | 5,275 |
| toefl-all | 10,367 |
| gre-all | 9,984 |
| gmat-all | 3,312 |
| sat-all | 4,464 |
| bec-all | 2,825 |
| junior-exam-all | 1,987 |
| senior-exam-all | 3,743 |
| PEPXiaoXue3_1 | 64 |
| PEPXiaoXue3_2 | 72 |
| PEPXiaoXue4_1 | 84 |
| PEPXiaoXue4_2 | 104 |
| PEPXiaoXue5_1 | 131 |
| PEPXiaoXue5_2 | 156 |
| PEPXiaoXue6_1 | 130 |
| PEPXiaoXue6_2 | 108 |
| PEPChuZhong7_1 | 392 |
| PEPChuZhong7_2 | 492 |
| PEPChuZhong8_1 | 419 |
| PEPChuZhong8_2 | 463 |
| PEPChuZhong9_1 | 551 |
| PEPGaoZhong_1 | 311 |
| PEPGaoZhong_2 | 319 |
| PEPGaoZhong_3 | 366 |
| PEPGaoZhong_4 | 307 |
| PEPGaoZhong_5 | 357 |
| PEPGaoZhong_6 | 391 |
| PEPGaoZhong_7 | 384 |
| PEPGaoZhong_8 | 420 |
| PEPGaoZhong_9 | 352 |
| PEPGaoZhong_10 | 361 |
| PEPGaoZhong_11 | 309 |
| BeiShiGaoZhong_2 | 244 |
| BeiShiGaoZhong_3 | 295 |
| BeiShiGaoZhong_7 | 334 |
