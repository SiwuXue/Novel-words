# 精读版 PDF 导出格式重构计划

## Summary

将精读版（intensive）PDF 导出格式从当前的 `[word /phonetic/]` + 词汇表模式，重构为用户提供的 PDF 样本格式：`word（释义）` 行内标注 + Step 1 / Step 2 两阶段学习结构，红色英文单词 + 紫色中文释义，去除音标和词汇表。

## Current State Analysis（当前实现 vs 目标格式）

### 当前精读版实现（intensive.rs）

| 维度 | 当前实现 |
|---|---|
| **标注格式** | `[word /phonetic/]` — 方括号 + 英文单词 + 音标 |
| **颜色** | 掌握度背景色（红/黄/绿 pastel），文字为默认色 |
| **结构** | 单次渲染：正文 → 章节生词表 → 全文总词汇表 |
| **章节标题** | 仅中文标题，无英文章节号 |
| **步骤** | 无 Step 1/Step 2 分段 |
| **页码** | 无 |
| **页眉** | 无 |
| **音标** | 行内标注中包含 |
| **词汇表** | 每章末尾 3 列生词表 + 全文总词汇表 |

### 目标格式（用户提供 PDF 样本）

| 维度 | 目标格式 |
|---|---|
| **标注格式** | `word（释义）` — 英文单词 + 全角括号 + 中文释义 |
| **颜色** | 英文单词红色（#CC0000），中文释义紫色（#990099） |
| **结构** | Step 1（带释义）+ Step 2（空白括号回忆），同一章节内容渲染两遍 |
| **章节标题** | "Chapter XX" + 中文标题 + 副标题 |
| **步骤** | Step 1: 在语境中背单词 → "—— Step 1 完 ——" → Step 2: 看单词回忆词义 |
| **页码** | "第 X 页"，每章重置 |
| **页眉** | 版权信息（作者 + 知识产权声明 + 书名） |
| **音标** | 不在行内显示 |
| **词汇表** | 无（完全去除） |

### 数据模型限制

以下目标 PDF 中的元素在当前数据模型中不存在对应字段，无法直接复现：

| 元素 | 原因 | 处理方式 |
|---|---|---|
| `【血案初现篇 · 第 1 章】` 卷信息 | Novel/Chapter 模型无"卷"概念 | 用 `【第 N 章】` 替代 |
| `本章词汇：100 词` 后的词汇主题 | VocabWord 无主题分类字段 | 只显示词数，不显示主题 |
| 封面页 + 目录页 | 需要卷/章分组数据 | 本期不做，后续迭代 |
| `en ch` 水印 | 防复制水印 | 不复现 |

## Proposed Changes（具体修改方案）

### 1. `src-tauri/src/pdf/mod.rs` — PdfContext 增强

#### 1.1 新增颜色常量

```rust
const TEXT_RED: Color = Color::Rgb(Rgb::new(0xCC as f32 / 255.0, 0x00 as f32 / 255.0, 0x00 as f32 / 255.0, None));
const TEXT_PURPLE: Color = Color::Rgb(Rgb::new(0x99 as f32 / 255.0, 0x00 as f32 / 255.0, 0x99 as f32 / 255.0, None));
const TEXT_BLACK: Color = Color::Greyscale(Greyscale { percent: 0.0, icc_profile: None });
const TEXT_GRAY: Color = Color::Greyscale(Greyscale { percent: 40.0, icc_profile: None });
```

#### 1.2 PdfContext 新增字段

```rust
pub struct PdfContext {
    // ... 现有字段 ...
    pub show_chrome: bool,
    pub chapter_page: usize,
    pub novel_title: String,
    pub novel_author: String,
}
```

#### 1.3 新增 `draw_text_colored` 方法

功能与 `draw_text` 相同，但接受 `Color` 参数，在绘制前设置 fill color，绘制后重置为黑色。

#### 1.4 新增 `render_page_chrome` 方法

在每页最终化前调用，绘制页眉（版权信息）和页脚（页码）。

#### 1.5 修改 `new_page` 方法

在最终化当前页之前，如果 `show_chrome` 为 true，调用 `render_page_chrome`，并递增 `chapter_page`。

#### 1.6 新增 `reset_chapter_page` 方法

每章开始时重置章节内页码为 1。

#### 1.7 修改 `generate_pdf` 函数

- 对 `intensive` 模板类型：跳过初始标题页，直接从第一章开始
- 设置 `show_chrome = true`，`novel_title`，`novel_author`
- 最终页面最终化时也需调用 `render_page_chrome`

---

### 2. `src-tauri/src/pdf/intensive.rs` — 精读版重写

#### 2.1 重写 `render` 函数

核心流程：对每章渲染 Step 1（带释义）+ Step 2（空白括号），不再生成词汇表。

```
对每章：
  1. 章节标题（Chapter XX + 中文标题 + 【第 N 章】+ 本章词汇数）
  2. Step 1: 在语境中背单词（标题 + 说明）
  3. 渲染正文 — 英文红色 + 中文释义紫色括号
  4. "—— Step 1 完 ——" 标记
  5. Step 2: 看单词回忆词义（标题 + 说明）
  6. 渲染同一正文 — 英文红色 + 空白括号
```

#### 2.2 新增 `draw_chapter_header` 函数

绘制章节标题区域：Chapter XX（居中大字号）+ 中文标题（居中）+ 【第 N 章】（红色居中）+ 本章词汇：N 词（灰色）。

#### 2.3 新增 `draw_step1_header` / `draw_step2_header` 函数

Step 1/Step 2 标题 + 灰色说明文字。

#### 2.4 新增 `draw_step1_end_marker` 函数

居中绘制 "—— Step 1 完 ——"。

#### 2.5 重写标注渲染函数

**`render_annotated_paragraph_step1`**（Step 1 — 带释义）：
- 匹配前的中文文本（黑色）→ 跳过匹配的中文词 → 英文单词（红色）→ 全角左括号（黑色）→ 中文释义（紫色）→ 全角右括号（黑色）

**`render_annotated_paragraph_step2`**（Step 2 — 空白括号）：
- 匹配前的中文文本（黑色）→ 跳过匹配的中文词 → 英文单词（红色）→ 空白全角括号 `（          ）`（黑色）

#### 2.6 修改 `draw_segment` 辅助函数

增加 `color: Color` 参数以支持彩色文本逐字符绘制和自动换行。

#### 2.7 移除词汇表

移除 `render` 中全文总词汇表和每章生词表生成代码。

---

### 3. `src/utils/pdfPreview.ts` — 前端预览同步

#### 3.1 更新标注渲染函数

- `renderParagraph` → `renderParagraphStep1`：英文红色 + 中文释义紫色括号
- 新增 `renderParagraphStep2`：英文红色 + 空白括号

#### 3.2 重写 `buildIntensive`

章节标题区域 + Step 1 标题/说明/正文 + "—— Step 1 完 ——" + Step 2 标题/说明/正文。不再生成全文总词汇表。

#### 3.3 更新 CSS

新增样式类：`.vocab-en`（红色）、`.vocab-def`（紫色）、`.vocab-blank`（灰色）、章节标题样式、Step 标题样式。

---

### 4. `src-tauri/src/commands/pdf_template.rs` — 模板参数微调

精读版内置模板参数微调：`font_size` 14→12，`line_spacing` 1.8→1.5，`margins` 增大上边距至 30mm。

## Assumptions & Decisions

1. **卷信息缺失**：用 `【第 N 章】` 替代 `【血案初现篇 · 第 1 章】`
2. **词汇主题缺失**：只显示词数，不显示主题
3. **封面页/目录页**：本期不做，作为后续迭代
4. **水印不复现**：`en ch` 防复制水印不复现
5. **页码每章重置**：`chapter_page` 在每章开始时重置为 1
6. **匹配器不变**：matcher.rs 不修改
7. **标注替换而非追加**：英文单词替换原文中的中文词位置，中文词移入括号内
8. **音标保留在数据中**：VocabWord.phonetic 不变，只是不在精读版行内显示
9. **其他模板不受影响**：sidebar/recitation/dictation 保持现有行为

## Verification Steps

1. **编译检查**：`cd src-tauri && cargo check`
2. **前端构建**：`npm run build`
3. **运行应用**：`npm run tauri dev`
4. **导出测试**：导入测试小说 + 词汇本 → 选择"精读版" → 导出 PDF
5. **格式对比**：
   - [ ] 章节标题格式（Chapter XX + 中文标题 + 副标题）
   - [ ] Step 1 标题 + 说明文字
   - [ ] 行内标注 `word（释义）`（英文红色 + 中文紫色）
   - [ ] "—— Step 1 完 ——" 标记
   - [ ] Step 2 标题 + 说明文字
   - [ ] 行内标注 `word（          ）`（英文红色 + 空白括号）
   - [ ] 无词汇表
   - [ ] 无音标
   - [ ] 页码 "第 X 页"
   - [ ] 页眉版权信息
6. **前端预览**：PdfExportDialog 预览面板与导出 PDF 一致
7. **多章节测试**：2+ 章节，确认每章页码重置、Step 1/Step 2 结构正确
