# 精读版单词熟练度颜色区分 — 实现计划

## 问题分析

精读版 PDF 导出（以及实时预览）中，所有英文单词都统一使用红色 `text_red()`，无法区分用户对不同单词的熟练度。但系统中已存在熟练度分级：

| 熟练度 | 值 | 用户含义 |
|---|---|---|
| 生疏 | `unknown` | 需要重点学习 |
| 熟悉 | `familiar` | 认识但需巩固 |
| 已掌握 | `mastered` | 已掌握，可略过 |

## 设计方案

### 颜色方案
参考现有 `proficiencyColors.ts` 的背景色方案，改为**文字颜色**区分（更适合精读版 inline 标注）：

| 熟练度 | 文字颜色 | 说明 |
|---|---|---|
| `unknown` 生疏 | **红色** `#CC0000` | 最醒目，需重点学习 |
| `familiar` 熟悉 | **橙色** `#E67E22` | 中等醒目 |
| `mastered` 已掌握 | **灰色** `#666666` | 低醒目，可略过 |

### 修改范围

#### 1. 前端：`src/utils/pdfPreview.ts`
- 添加 `proficiencyTextColor()` 函数，根据 proficiency 返回对应 CSS 颜色
- 修改 `renderParagraphStep1()` 和 `renderParagraphStep2()` 中的英文单词 `<span>` 颜色，从固定红色改为根据 `word.proficiency` 动态设置
- 更新 `baseCss()` 中的 CSS 类，支持三级颜色

#### 2. 前端：`src/utils/proficiencyColors.ts`
- 添加文字颜色映射（与 Rust 后端保持一致）

#### 3. 后端：`src-tauri/src/pdf/mod.rs`
- 添加 `text_color_for_proficiency()` 函数，返回对应熟练度的 `Color`

#### 4. 后端：`src-tauri/src/pdf/intensive.rs`
- 修改 `render_annotated_paragraph_step1()` 中的 `draw_segment(en, text_red())` → 使用 `text_color_for_proficiency(m.word.proficiency)`
- 修改 `render_annotated_paragraph_step2()` 同上

## 风险与注意事项
- 前后端颜色必须保持一致（在注释中标记同步点）
- 熟练度为空或异常值时，回退到红色（unknown）
- 不影响侧边栏版、背诵版、默写版的现有逻辑
