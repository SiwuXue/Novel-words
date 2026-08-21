//! Intensive reading template: three-stage learning structure.
//! Step 1: English word (by proficiency color) + Chinese definition (purple) inline after the
//!         matched Chinese term is replaced.
//! Step 2: Same content but the definition area is a blank bracket pair so
//!         learners can recall the meaning from context.
//! Step 3: Two-column word list (idx / word / definition), proficiency-colored words,
//!         with header row, table borders, and chapter-end marker.
use super::matcher::{
    find_matches_in_line, find_matches_in_line_en, words_found_in_text, words_found_in_text_en,
};
use super::{
    table_border, table_header_bg, text_black, text_gray, text_light_gray, text_purple, text_red,
    text_color_for_proficiency, PdfContext,
};
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;
use printpdf::Color;

/// Step toggle flags for the intensive reading template.
/// All false → normalized to Step 1 so we never render an empty chapter.
#[derive(Clone, Copy, Debug, Default)]
pub struct IntensiveSteps {
    pub step1: bool,
    pub step2: bool,
    pub step3: bool,
}

impl IntensiveSteps {
    pub fn any(self) -> bool {
        self.step1 || self.step2 || self.step3
    }

    pub fn normalize(mut self) -> Self {
        if !self.any() {
            self.step1 = true;
        }
        self
    }
}

/// Parse the raw DB string for `pdf_intensive_steps` (expected JSON array like "[1,2,3]")
/// into flags. None / empty / invalid JSON / any array with no valid {1,2,3} elements
/// → fallback to all three steps enabled (so existing installs behave unchanged).
pub fn parse_steps_from_db(value: Option<&str>) -> IntensiveSteps {
    let v = match value {
        Some(s) if !s.trim().is_empty() => s,
        _ => {
            return IntensiveSteps {
                step1: true,
                step2: true,
                step3: true,
            }
        }
    };
    let arr: Vec<u8> = match serde_json::from_str(v) {
        Ok(xs) => xs,
        Err(_) => {
            return IntensiveSteps {
                step1: true,
                step2: true,
                step3: true,
            }
        }
    };
    let mut s = IntensiveSteps::default();
    for n in arr {
        match n {
            1 => s.step1 = true,
            2 => s.step2 = true,
            3 => s.step3 = true,
            _ => {}
        }
    }
    if !s.any() {
        s.step1 = true;
        s.step2 = true;
        s.step3 = true;
    }
    s
}

// ---------------------------------------------------------------------------
// Definition helpers: strip 【记忆】【搭配】 blocks + pick only sense(s) that
// actually appear in the surrounding line (a rough context-based disambiguation).
// Falls back to the first 2 senses if no match found or the definition is not
// from the CET4 preset (has no `\n` structure, e.g. a manual vocab_word).
// ---------------------------------------------------------------------------

/// Strips 【记忆】/【搭配】/【真题】 blocks and returns only the "词性+释义" lines.
/// Returns a Vec of (pos: Option<String>, cndefs: Vec<String>).
fn parse_sense_lines(raw: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for line in raw.split('\n') {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with("【记忆】") || t.starts_with("【搭配】") || t.starts_with("【真题】")
            || t.starts_with("【派生】") || t.starts_with("【例句】") {
            break; // auxiliary sections are terminal; stop after first sighting
        }
        // Lines that start with "· " (phrase bullets) belong to 【搭配】 but
        // occasionally a stray bullet appears after the section header — skip.
        if t.starts_with("· ") { continue; }
        lines.push(t.to_string());
    }
    lines
}

/// Breaks a sense string like "adj. 特别的，特殊的；讲究的，挑剔的" into
/// ("adj.", ["特别的", "特殊的", "讲究的", "挑剔的"]). If no pos prefix,
/// pos is empty string.
fn split_sense(s: &str) -> (String, Vec<String>) {
    // Find the first position after the longest leading ASCII-letter / '-'
    // segment followed by '.'. Examples: "adj. ", "n. ", "vt. ", "n./v. "
    let end = s.find('.')
        .filter(|&idx| s[..idx].chars().all(|c| c.is_ascii_alphabetic() || c == '/'));
    let (pos, rest) = match end {
        Some(i) => (s[..=i].trim().to_string(), s[i + 1..].trim()),
        None => (String::new(), s.trim()),
    };
    // Split Chinese text by common sense separators: / , ； 、 ; ，
    let items: Vec<String> = rest
        .split(|c: char| matches!(c, '/' | '、' | '；' | ';' | '，' | ','))
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect();
    (pos, items)
}

/// Returns a short sense string suitable for inline （parens） in Step 1/2.
/// Strategy:
///   1. Parse sense lines, drop 【记忆】【搭配】
///   2. If `context_line` contains Chinese (CN novel), score each CNEF item by
///      whether any of its chars appear as a substring inside context_line.
///   3. Keep matched senses (max 2). If no match, keep first 2 total senses.
///
/// Output format: "adj. 特别的；讲究的 / n. 讲究"  (semicolons between items of
/// same pos, slash between different parts of speech).
fn short_definition(definition: &str, context_line: &str) -> String {
    if definition.is_empty() {
        return "—".into();
    }
    let sense_lines = parse_sense_lines(definition);
    if sense_lines.is_empty() {
        // No structured format (hand-typed vocab). Return:
        // first paragraph (up to 1st \n【 or \n), max 30 chars.
        let top = definition.split('\n').next().unwrap_or(definition);
        let clean = if let Some(idx) = top.find("【记忆】").or_else(|| top.find("【搭配】")) {
            &top[..idx]
        } else { top };
        let clean = clean.trim();
        return clean.chars().take(30).collect::<String>();
    }

    // Build flat list of (pos, cndef) with original line-index ordering
    struct Flat { pos: String, cndef: String, line_no: usize }
    let mut flat: Vec<Flat> = Vec::with_capacity(sense_lines.len() * 2);
    for (li, sl) in sense_lines.iter().enumerate() {
        let (pos, items) = split_sense(sl);
        for it in items {
            flat.push(Flat { pos: pos.clone(), cndef: it, line_no: li });
        }
    }

    // Determine whether context is usable for matching: needs to contain CJK
    let has_cjk = context_line.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
    let mut selected: Vec<&Flat> = Vec::new();

    if has_cjk {
        // Score each flat item: cndef is a substring of context_line?
        for f in flat.iter() {
            // Skip 1-char CNFs (usually classifier fillers like "一") to reduce false positives
            if f.cndef.chars().count() <= 1 { continue; }
            if context_line.contains(&f.cndef) {
                selected.push(f);
                if selected.len() >= 2 { break; }
            }
        }
    }

    // If no match (or EN novel), take first 2 flat items as fallback
    if selected.is_empty() {
        for f in flat.iter().take(2) {
            selected.push(f);
        }
    }
    if selected.is_empty() {
        return sense_lines[0].clone();
    }

    // Render: group consecutive same-pos by line_no, semicolons within group, slashes between lines
    let mut out = String::new();
    let mut last_pos = "";
    for (i, f) in selected.iter().enumerate() {
        let is_new_line = i == 0 || f.line_no != selected[i - 1].line_no;
        if is_new_line {
            if i > 0 { out.push_str(" / "); }
            if !f.pos.is_empty() {
                out.push_str(&f.pos);
                out.push(' ');
            }
        } else if f.pos != last_pos {
            out.push_str("；");
            if !f.pos.is_empty() {
                out.push_str(&f.pos);
                out.push(' ');
            }
        } else {
            out.push_str("，");
        }
        out.push_str(&f.cndef);
        last_pos = &f.pos;
    }
    out
}

pub fn render(
    ctx: &mut PdfContext,
    chapters: &[Chapter],
    vocabs: &[VocabWord],
    steps: IntensiveSteps,
    language: &str,
    progress: Option<&dyn Fn(super::PdfProgress)>,
) {
    let steps = steps.normalize();
    let is_en = language == "en";
    let total_chapters = chapters.len().max(1);
    for (ci, chapter) in chapters.iter().enumerate() {
        if let Some(p) = progress {
            let title = if chapter.title.is_empty() {
                "全文".to_string()
            } else {
                chapter.title.clone()
            };
            let percent = ((ci as f32 / total_chapters as f32) * 88.0).round() as u32;
            p(super::PdfProgress {
                percent,
                message: format!("正在生成第 {}/{} 章：{}", ci + 1, total_chapters, title),
            });
        }

        if ci > 0 {
            ctx.new_page_for_chapter();
        }
        ctx.reset_chapter_page();

        let chapter_num = ci + 1;
        let chapter_words: Vec<&VocabWord> = if is_en {
            words_found_in_text_en(&chapter.content, vocabs)
        } else {
            words_found_in_text(&chapter.content, vocabs)
        };
        let chapter_word_count = chapter_words.len();

        // Chapter header area (always displayed — chapter headings are not part of steps)
        draw_chapter_header(ctx, chapter_num, &chapter.title, chapter_word_count);

        // ===== STEP 1 =====
        if steps.step1 {
            draw_step1_header(ctx);

            for para in super::split_paragraphs(&chapter.content) {
                if ctx.remaining_height() < ctx.line_height * 2.0 {
                    ctx.new_page();
                }
                if is_en {
                    render_annotated_paragraph_step1_en(ctx, &para, vocabs);
                } else {
                    render_annotated_paragraph_step1(ctx, &para, vocabs);
                }
                ctx.current_y -= ctx.line_height * 0.4;
            }

            draw_step1_end_marker(ctx);
        }

        // ===== STEP 2 =====
        if steps.step2 {
            draw_step2_header(ctx);

            for para in super::split_paragraphs(&chapter.content) {
                if ctx.remaining_height() < ctx.line_height * 2.0 {
                    ctx.new_page();
                }
                if is_en {
                    render_annotated_paragraph_step2_en(ctx, &para, vocabs);
                } else {
                    render_annotated_paragraph_step2(ctx, &para, vocabs);
                }
                ctx.current_y -= ctx.line_height * 0.4;
            }
        }

        // ===== STEP 3 =====
        if steps.step3 {
            if chapter_words.is_empty() {
                draw_step3_header(ctx, 0);
            } else {
                draw_step3_header(ctx, chapter_word_count);
                draw_step3_word_table(ctx, &chapter_words);
            }
        }

        // ===== Chapter end marker (always displayed) =====
        draw_chapter_end_marker(ctx, chapter_num);
    }
}

/// Draw the chapter title block:
///   Chapter XX (large, centered, black)
///   中文标题 (centered, black)
///   【第 N 章】(centered, red, smaller)
///   本章词汇：N 词 (centered, gray, smallest)
fn draw_chapter_header(
    ctx: &mut PdfContext,
    chapter_num: usize,
    chapter_title: &str,
    word_count: usize,
) {
    if ctx.remaining_height() < ctx.line_height * 5.0 {
        ctx.new_page();
    }

    let cx = ctx.margins.left + ctx.usable_width / 2.0;
    let big_size = ctx.font_size + 6.0;
    let mid_size = ctx.font_size + 2.0;
    let small_size = ctx.small_font_size;

    // "Chapter XX" — centered
    let ch_line = format!("Chapter {}", chapter_num);
    let w = ctx.measure_text_width(&ch_line, big_size);
    ctx.draw_text_colored(&ch_line, cx - w / 2.0, ctx.current_y, big_size, text_black());
    ctx.current_y -= ctx.line_height * 1.2;

    // 中文标题 — centered
    if !chapter_title.is_empty() {
        ctx.record_bookmark(chapter_title);
        let w = ctx.measure_text_width(chapter_title, mid_size);
        ctx.draw_text_colored(chapter_title, cx - w / 2.0, ctx.current_y, mid_size, text_black());
        ctx.current_y -= ctx.line_height * 1.0;
    }

    // 【第 N 章】 — centered, red
    let sub = format!("【第 {} 章】", chapter_num);
    let w = ctx.measure_text_width(&sub, small_size);
    ctx.draw_text_colored(&sub, cx - w / 2.0, ctx.current_y, small_size, text_red());
    ctx.current_y -= ctx.line_height * 0.8;

    // 本章词汇：N 词 — centered, gray
    let wc = format!("本章词汇：{} 词", word_count);
    let w = ctx.measure_text_width(&wc, small_size * 0.85);
    ctx.draw_text_colored(&wc, cx - w / 2.0, ctx.current_y, small_size * 0.85, text_gray());
    ctx.current_y -= ctx.line_height * 1.5;
}

/// Step 1 header: "Step 1：在语境中背单词" + gray description.
fn draw_step1_header(ctx: &mut PdfContext) {
    if ctx.remaining_height() < ctx.line_height * 3.0 {
        ctx.new_page();
    }
    let title = "Step 1：在语境中背单词";
    ctx.draw_text_colored(title, ctx.margins.left, ctx.current_y, ctx.font_size + 1.0, text_black());
    ctx.current_y -= ctx.line_height * 0.8;
    let desc = "请仔细阅读下文，注意英文单词及其对应的中文释义。红色=生疏，橙色=熟悉，灰色=已掌握。";
    ctx.draw_text_colored(desc, ctx.margins.left, ctx.current_y, ctx.small_font_size, text_gray());
    ctx.current_y -= ctx.line_height * 1.3;
}

/// Centered "—— Step 1 完 ——" marker.
fn draw_step1_end_marker(ctx: &mut PdfContext) {
    if ctx.remaining_height() < ctx.line_height * 2.5 {
        ctx.new_page();
    }
    let marker = "—— Step 1 完 ——";
    let w = ctx.measure_text_width(marker, ctx.small_font_size);
    let cx = ctx.margins.left + ctx.usable_width / 2.0;
    ctx.draw_text_colored(marker, cx - w / 2.0, ctx.current_y, ctx.small_font_size, text_gray());
    ctx.current_y -= ctx.line_height * 1.5;
}

/// Step 2 header: "Step 2：看单词回忆词义" + gray description.
fn draw_step2_header(ctx: &mut PdfContext) {
    if ctx.remaining_height() < ctx.line_height * 3.0 {
        ctx.new_page();
    }
    let title = "Step 2：看单词回忆词义";
    ctx.draw_text_colored(title, ctx.margins.left, ctx.current_y, ctx.font_size + 1.0, text_black());
    ctx.current_y -= ctx.line_height * 0.8;
    let desc = "请再次阅读下文，尝试回忆英文单词对应的中文意思。";
    ctx.draw_text_colored(desc, ctx.margins.left, ctx.current_y, ctx.small_font_size, text_gray());
    ctx.current_y -= ctx.line_height * 1.3;
}

/// Step 3 header: "Step 3：单词列表（本章 N 词）" + description.
fn draw_step3_header(ctx: &mut PdfContext, word_count: usize) {
    if ctx.remaining_height() < ctx.line_height * 3.5 {
        ctx.new_page();
    }
    let title = format!("Step 3：单词列表（本章 {} 词）", word_count);
    ctx.draw_text_colored(&title, ctx.margins.left, ctx.current_y, ctx.font_size + 1.0, text_black());
    ctx.current_y -= ctx.line_height * 0.8;
    let desc = if word_count == 0 {
        "本章没有匹配到词汇本中的单词。".to_string()
    } else {
        format!("复习本章出现的全部 {} 个单词，巩固记忆效果。", word_count)
    };
    ctx.draw_text_colored(&desc, ctx.margins.left, ctx.current_y, ctx.small_font_size, text_gray());
    ctx.current_y -= ctx.line_height * 1.2;
}

/// Centered "—— 第 X 章 完 ——" marker (slightly larger than step-end marker).
fn draw_chapter_end_marker(ctx: &mut PdfContext, chapter_num: usize) {
    if ctx.remaining_height() < ctx.line_height * 2.5 {
        ctx.new_page();
    }
    let marker = format!("—— 第 {} 章 完 ——", chapter_num);
    let w = ctx.measure_text_width(&marker, ctx.font_size + 1.0);
    let cx = ctx.margins.left + ctx.usable_width / 2.0;
    ctx.draw_text_colored(&marker, cx - w / 2.0, ctx.current_y, ctx.font_size + 1.0, text_black());
    ctx.current_y -= ctx.line_height * 1.5;
}

// ---------------------------------------------------------------------------
// Step 3: Two-column word table
// ---------------------------------------------------------------------------

/// Layout parameters for one column of the Step 3 table.
struct ColLayout {
    x_left: f32,        // left edge of column block (mm)
    col_w: f32,         // total column width (mm)
    idx_w: f32,         // 序号 cell width
    word_w: f32,        // 单词 cell width
    def_w: f32,         // 释义 cell width
}

/// Calculate column layout (two side-by-side tables) given current context.
fn compute_table_layout(ctx: &PdfContext) -> (ColLayout, ColLayout, f32 /* gap */) {
    let gap = 4.0; // mm between the two columns
    let usable = ctx.usable_width;
    let col_w = (usable - gap) / 2.0;

    let idx_w = (col_w * 0.15).max(8.0);
    let word_w = col_w * 0.34;
    let def_w = col_w - idx_w - word_w;

    let left = ColLayout {
        x_left: ctx.margins.left,
        col_w,
        idx_w,
        word_w,
        def_w,
    };
    let right = ColLayout {
        x_left: ctx.margins.left + col_w + gap,
        col_w,
        idx_w,
        word_w,
        def_w,
    };
    (left, right, gap)
}

/// Compute how many data rows (excluding header) fit in `avail_height` mm.
fn rows_that_fit(_ctx: &PdfContext, avail_height: f32, row_h: f32, header_h: f32) -> usize {
    if avail_height <= header_h {
        return 0;
    }
    let rem = avail_height - header_h;
    (rem / row_h).floor() as usize
}

/// Draw the header row (background + 3 labels + bottom border) for one column.
/// Returns the y-position of the bottom of the header (= top y minus header height).
fn draw_column_header(ctx: &mut PdfContext, col: &ColLayout, top_y: f32, header_h: f32) -> f32 {
    // Background rectangle (fill before text)
    ctx.fill_rect(col.x_left, top_y, col.col_w, header_h, table_header_bg());

    let border_thick = 0.4;
    let label_size = ctx.small_font_size;
    let text_y = top_y - header_h * 0.35; // baseline, approximate within the band

    let x_idx = col.x_left;
    let x_word = col.x_left + col.idx_w;
    let x_def = col.x_left + col.idx_w + col.word_w;

    // 序号 (centered)
    let label_idx = "序号";
    let w = ctx.measure_text_width(label_idx, label_size);
    ctx.draw_text_colored(label_idx, x_idx + (col.idx_w - w) / 2.0, text_y, label_size, text_black());
    // 单词 (centered)
    let label_word = "单词";
    let w = ctx.measure_text_width(label_word, label_size);
    ctx.draw_text_colored(label_word, x_word + (col.word_w - w) / 2.0, text_y, label_size, text_black());
    // 释义 (centered)
    let label_def = "释义";
    let w = ctx.measure_text_width(label_def, label_size);
    ctx.draw_text_colored(label_def, x_def + (col.def_w - w) / 2.0, text_y, label_size, text_black());

    // Bottom border of header
    let bottom_y = top_y - header_h;
    ctx.draw_hline(col.x_left, col.x_left + col.col_w, bottom_y, table_border(), border_thick);
    // Top border
    ctx.draw_hline(col.x_left, col.x_left + col.col_w, top_y, table_border(), border_thick);
    // Left outer border
    ctx.draw_vline(col.x_left, top_y, bottom_y, table_border(), border_thick);
    // Divider between 序号 & 单词
    ctx.draw_vline(x_word, top_y, bottom_y, table_border(), border_thick);
    // Divider between 单词 & 释义
    ctx.draw_vline(x_def, top_y, bottom_y, table_border(), border_thick);
    // Right outer border
    ctx.draw_vline(col.x_left + col.col_w, top_y, bottom_y, table_border(), border_thick);

    bottom_y
}

/// Draw one data row at baseline y = `row_top_y` (which is top edge of the row rectangle).
/// Row occupies row_top_y down to row_top_y - row_h.
fn draw_data_row(
    ctx: &mut PdfContext,
    col: &ColLayout,
    row_top_y: f32,
    row_h: f32,
    idx_num: usize,
    word: &VocabWord,
) {
    let font_size = ctx.small_font_size;
    let idx_font = font_size * 0.9;
    let text_y = row_top_y - row_h * 0.32;

    let x_idx = col.x_left;
    let x_word = col.x_left + col.idx_w;
    let x_def = col.x_left + col.idx_w + col.word_w;
    let right_x = col.x_left + col.col_w;
    let bottom_y = row_top_y - row_h;
    let border_thick = 0.4;

    // 序号 (two digits, gray, centered)
    let idx_str = format!("{:02}", idx_num);
    let w = ctx.measure_text_width(&idx_str, idx_font);
    ctx.draw_text_colored(&idx_str, x_idx + (col.idx_w - w) / 2.0, text_y, idx_font, text_light_gray());

    // 单词 (by proficiency color, left-aligned with small padding)
    let pad = 0.6;
    let en_max_w = col.word_w - pad * 2.0;
    let en = ctx.truncate_text(&word.word, en_max_w, font_size);
    ctx.draw_text_colored(&en, x_word + pad, text_y, font_size, text_color_for_proficiency(&word.proficiency));

    // 释义 (black, left-aligned with small padding, truncated)
    let def = if word.definition.is_empty() { "—" } else { &word.definition };
    let def_max_w = col.def_w - pad * 2.0;
    let def_short = ctx.truncate_text(def, def_max_w, font_size);
    ctx.draw_text_colored(&def_short, x_def + pad, text_y, font_size, text_black());

    // Row bottom border
    ctx.draw_hline(col.x_left, right_x, bottom_y, table_border(), border_thick);
    // Dividers
    ctx.draw_vline(x_word, row_top_y, bottom_y, table_border(), border_thick);
    ctx.draw_vline(x_def, row_top_y, bottom_y, table_border(), border_thick);
    // Left & right outer borders
    ctx.draw_vline(col.x_left, row_top_y, bottom_y, table_border(), border_thick);
    ctx.draw_vline(right_x, row_top_y, bottom_y, table_border(), border_thick);
}

/// Render a "page" of the Step 3 table: one column on the left (words L) and
/// one on the right (words R). Both columns start at the current `ctx.current_y`.
/// After drawing, `ctx.current_y` is updated to the y below the last row of the
/// smaller of the two columns (in number of rows — actually both columns use the same
/// baseline per row, so we just advance by max(count_left, count_right) rows).
fn render_table_page(
    ctx: &mut PdfContext,
    left: &ColLayout,
    right: &ColLayout,
    left_words: &[&VocabWord],
    right_words: &[&VocabWord],
    left_start_idx: usize,
    right_start_idx: usize,
) {
    let row_h = ctx.line_height * 0.95;
    let header_h = ctx.line_height * 1.0;
    let max_rows = left_words.len().max(right_words.len());
    if max_rows == 0 {
        return;
    }

    // Headers
    let header_bottom_left = draw_column_header(ctx, left, ctx.current_y, header_h);
    let header_bottom_right = draw_column_header(ctx, right, ctx.current_y, header_h);
    let header_bottom = header_bottom_left.min(header_bottom_right);

    // Move cursor below header to start drawing rows
    let mut row_top = header_bottom;

    for i in 0..max_rows {
        // Left row
        if i < left_words.len() {
            draw_data_row(ctx, left, row_top, row_h, left_start_idx + i, left_words[i]);
        } else {
            // Still draw left outer column border lines / bottom lines with empty content,
            // so the two columns visually match in height.
            let bottom_y = row_top - row_h;
            let bt = 0.4;
            let col = left;
            ctx.draw_hline(col.x_left, col.x_left + col.col_w, bottom_y, table_border(), bt);
            ctx.draw_vline(col.x_left, row_top, bottom_y, table_border(), bt);
            ctx.draw_vline(col.x_left + col.col_w, row_top, bottom_y, table_border(), bt);
            let x1 = col.x_left + col.idx_w;
            let x2 = col.x_left + col.idx_w + col.word_w;
            ctx.draw_vline(x1, row_top, bottom_y, table_border(), bt);
            ctx.draw_vline(x2, row_top, bottom_y, table_border(), bt);
        }
        // Right row
        if i < right_words.len() {
            draw_data_row(ctx, right, row_top, row_h, right_start_idx + i, right_words[i]);
        } else {
            let bottom_y = row_top - row_h;
            let bt = 0.4;
            let col = right;
            ctx.draw_hline(col.x_left, col.x_left + col.col_w, bottom_y, table_border(), bt);
            ctx.draw_vline(col.x_left, row_top, bottom_y, table_border(), bt);
            ctx.draw_vline(col.x_left + col.col_w, row_top, bottom_y, table_border(), bt);
            let x1 = col.x_left + col.idx_w;
            let x2 = col.x_left + col.idx_w + col.word_w;
            ctx.draw_vline(x1, row_top, bottom_y, table_border(), bt);
            ctx.draw_vline(x2, row_top, bottom_y, table_border(), bt);
        }
        row_top -= row_h;
    }

    ctx.current_y = row_top;
}

/// Core Step 3 table renderer.
/// Algorithm (sequential dual column):
///   * Given remaining space, compute R = rows per column that fit.
///   * Page 1: left = words[0..R], right = words[R..2R]
///   * new_page() if more words; repeat pattern with full-page R'
fn draw_step3_word_table<'a>(ctx: &mut PdfContext, words: &[&'a VocabWord]) {
    let (left_col, right_col, _gap) = compute_table_layout(ctx);
    let row_h = ctx.line_height * 0.95;
    let header_h = ctx.line_height * 1.0;

    let full_page_rows = {
        let total_available = ctx.paper_height - ctx.margins.top - ctx.margins.bottom - 5.0;
        rows_that_fit(ctx, total_available, row_h, header_h)
    };
    let rows_first_page = rows_that_fit(ctx, ctx.remaining_height(), row_h, header_h);

    let mut i = 0;
    let n = words.len();
    let mut is_first = true;

    while i < n {
        let rows_per_col: usize = if is_first {
            if rows_first_page < 3 {
                ctx.new_page();
                full_page_rows
            } else {
                rows_first_page
            }
        } else {
            full_page_rows
        };
        is_first = false;

        if rows_per_col == 0 {
            ctx.new_page();
            continue;
        }

        let left_end = (i + rows_per_col).min(n);
        let left: &[&VocabWord] = &words[i..left_end];
        let left_len = left.len();
        let r_start = i + rows_per_col;
        let right: &[&VocabWord] = if r_start < n {
            &words[r_start..(r_start + rows_per_col).min(n)]
        } else {
            &[]
        };

        render_table_page(ctx, &left_col, &right_col, left, right, i, r_start);

        i += left_len + right.len();
        if i < n {
            ctx.current_y -= ctx.line_height * 0.8;
            ctx.new_page();
        }
    }

    ctx.current_y -= ctx.line_height * 0.8;
}

// ---------------------------------------------------------------------------
// Paragraph rendering
// ---------------------------------------------------------------------------

fn draw_segment(ctx: &mut PdfContext, x: &mut f32, max_x: f32, seg: &str, color: Color) {
    for ch in seg.chars() {
        let cw = ctx.measure_text_width(&ch.to_string(), ctx.font_size);
        if *x + cw > max_x {
            ctx.current_y -= ctx.line_height;
            *x = ctx.margins.left;
            if ctx.remaining_height() < ctx.line_height * 2.0 {
                ctx.new_page();
            }
        }
        ctx.draw_text_colored(&ch.to_string(), *x, ctx.current_y, ctx.font_size, color.clone());
        *x += cw;
    }
}

/// Step 1 paragraph: matched Chinese → English (red) + （definition） purple.
fn render_annotated_paragraph_step1(ctx: &mut PdfContext, line: &str, vocabs: &[VocabWord]) {
    let matches = find_matches_in_line(line, vocabs);
    if matches.is_empty() {
        ctx.draw_text_wrapped(line, ctx.margins.left, ctx.usable_width, ctx.font_size);
        return;
    }

    let max_x = ctx.margins.left + ctx.usable_width;
    let mut x = ctx.margins.left;
    let mut last = 0usize;

    for m in &matches {
        if m.start > last {
            let pre = &line[last..m.start];
            draw_segment(ctx, &mut x, max_x, pre, text_black());
        }
        // Skip the original Chinese term — it's replaced by English + definition.

        // English word (by proficiency)
        let en = &m.word.word;
        draw_segment(ctx, &mut x, max_x, en, text_color_for_proficiency(&m.word.proficiency));

        // Full-width left paren (black)
        draw_segment(ctx, &mut x, max_x, "（", text_black());

        // Definition (purple) — sense-picked using the surrounding CN line
        let def = short_definition(&m.word.definition, line);
        draw_segment(ctx, &mut x, max_x, &def, text_purple());

        // Full-width right paren (black)
        draw_segment(ctx, &mut x, max_x, "）", text_black());

        last = m.end;
    }

    if last < line.len() {
        let rest = &line[last..];
        draw_segment(ctx, &mut x, max_x, rest, text_black());
    }
    ctx.current_y -= ctx.line_height;
}

/// Step 2 paragraph: matched Chinese → English (red) + （          ） blank.
fn render_annotated_paragraph_step2(ctx: &mut PdfContext, line: &str, vocabs: &[VocabWord]) {
    let matches = find_matches_in_line(line, vocabs);
    if matches.is_empty() {
        ctx.draw_text_wrapped(line, ctx.margins.left, ctx.usable_width, ctx.font_size);
        return;
    }

    let max_x = ctx.margins.left + ctx.usable_width;
    let mut x = ctx.margins.left;
    let mut last = 0usize;

    for m in &matches {
        if m.start > last {
            let pre = &line[last..m.start];
            draw_segment(ctx, &mut x, max_x, pre, text_black());
        }
        // Skip the original Chinese term.

        // English word (by proficiency)
        let en = &m.word.word;
        draw_segment(ctx, &mut x, max_x, en, text_color_for_proficiency(&m.word.proficiency));

        // Blank full-width parens: estimate width using *shortened* definition
        let def_estimate = short_definition(&m.word.definition, line);
        let def_len = if def_estimate.is_empty() || def_estimate == "—" {
            4
        } else {
            def_estimate.chars().count().max(4)
        };
        let blank: String = std::iter::repeat('　').take(def_len).collect();
        let bracket_content = format!("（{}）", blank);
        draw_segment(ctx, &mut x, max_x, &bracket_content, text_black());

        last = m.end;
    }

    if last < line.len() {
        let rest = &line[last..];
        draw_segment(ctx, &mut x, max_x, rest, text_black());
    }
    ctx.current_y -= ctx.line_height;
}

// ---------------------------------------------------------------------------
// English-novel paragraph rendering: the body is English, so matched words
// are colored in place (not swapped in) and （definition）/（　　） appended.
// ---------------------------------------------------------------------------

/// Step 1 paragraph (English novel): matched English word colored by proficiency
/// + （definition） purple appended right after the word.
fn render_annotated_paragraph_step1_en(ctx: &mut PdfContext, line: &str, vocabs: &[VocabWord]) {
    let matches = find_matches_in_line_en(line, vocabs);
    if matches.is_empty() {
        ctx.draw_text_wrapped(line, ctx.margins.left, ctx.usable_width, ctx.font_size);
        return;
    }

    let max_x = ctx.margins.left + ctx.usable_width;
    let mut x = ctx.margins.left;
    let mut last = 0usize;

    for m in &matches {
        if m.start > last {
            let pre = &line[last..m.start];
            draw_segment(ctx, &mut x, max_x, pre, text_black());
        }
        // The English word as it appears in the body, colored by proficiency
        let en = &line[m.start..m.end];
        draw_segment(ctx, &mut x, max_x, en, text_color_for_proficiency(&m.word.proficiency));

        // Full-width left paren (black)
        draw_segment(ctx, &mut x, max_x, "（", text_black());
        // Definition (purple) — first 2 senses stripped of 记忆/搭配 blocks
        let def = short_definition(&m.word.definition, "");
        draw_segment(ctx, &mut x, max_x, &def, text_purple());
        // Full-width right paren (black)
        draw_segment(ctx, &mut x, max_x, "）", text_black());

        last = m.end;
    }

    if last < line.len() {
        let rest = &line[last..];
        draw_segment(ctx, &mut x, max_x, rest, text_black());
    }
    ctx.current_y -= ctx.line_height;
}

/// Step 2 paragraph (English novel): matched English word colored by proficiency
/// + （　　　） blank appended right after the word.
fn render_annotated_paragraph_step2_en(ctx: &mut PdfContext, line: &str, vocabs: &[VocabWord]) {
    let matches = find_matches_in_line_en(line, vocabs);
    if matches.is_empty() {
        ctx.draw_text_wrapped(line, ctx.margins.left, ctx.usable_width, ctx.font_size);
        return;
    }

    let max_x = ctx.margins.left + ctx.usable_width;
    let mut x = ctx.margins.left;
    let mut last = 0usize;

    for m in &matches {
        if m.start > last {
            let pre = &line[last..m.start];
            draw_segment(ctx, &mut x, max_x, pre, text_black());
        }
        // The English word as it appears in the body, colored by proficiency
        let en = &line[m.start..m.end];
        draw_segment(ctx, &mut x, max_x, en, text_color_for_proficiency(&m.word.proficiency));

        // Blank full-width parens sized to the SHORTENED definition length
        let def_estimate = short_definition(&m.word.definition, "");
        let def_len = if def_estimate.is_empty() || def_estimate == "—" {
            4
        } else {
            def_estimate.chars().count().max(4)
        };
        let blank: String = std::iter::repeat('　').take(def_len).collect();
        let bracket_content = format!("（{}）", blank);
        draw_segment(ctx, &mut x, max_x, &bracket_content, text_black());

        last = m.end;
    }

    if last < line.len() {
        let rest = &line[last..];
        draw_segment(ctx, &mut x, max_x, rest, text_black());
    }
    ctx.current_y -= ctx.line_height;
}
