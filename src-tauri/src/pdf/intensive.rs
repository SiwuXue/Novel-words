//! Intensive reading template: three-stage learning structure.
//! Step 1: English word (by proficiency color) + Chinese definition (purple) inline after the
//!         matched Chinese term is replaced.
//! Step 2: Same content but the definition area is a blank bracket pair so
//!         learners can recall the meaning from context.
//! Step 3: Two-column word list (idx / word / definition), proficiency-colored words,
//!         with header row, table borders, and chapter-end marker.
use super::matcher::{find_matches_in_line, words_found_in_text};
use super::{
    table_border, table_header_bg, text_black, text_gray, text_light_gray, text_purple, text_red,
    text_color_for_proficiency, PdfContext,
};
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;
use printpdf::Color;

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    for (ci, chapter) in chapters.iter().enumerate() {
        if ci > 0 {
            ctx.new_page_for_chapter();
        }
        ctx.reset_chapter_page();

        let chapter_num = ci + 1;
        let chapter_words: Vec<&VocabWord> = words_found_in_text(&chapter.content, vocabs);
        let chapter_word_count = chapter_words.len();

        // Chapter header area
        draw_chapter_header(ctx, chapter_num, &chapter.title, chapter_word_count);

        // ===== STEP 1 =====
        draw_step1_header(ctx);

        for para in super::split_paragraphs(&chapter.content) {
            if ctx.remaining_height() < ctx.line_height * 2.0 {
                ctx.new_page();
            }
            render_annotated_paragraph_step1(ctx, &para, vocabs);
            ctx.current_y -= ctx.line_height * 0.4;
        }

        draw_step1_end_marker(ctx);

        // ===== STEP 2 =====
        draw_step2_header(ctx);

        for para in super::split_paragraphs(&chapter.content) {
            if ctx.remaining_height() < ctx.line_height * 2.0 {
                ctx.new_page();
            }
            render_annotated_paragraph_step2(ctx, &para, vocabs);
            ctx.current_y -= ctx.line_height * 0.4;
        }

        // ===== STEP 3 =====
        if chapter_words.is_empty() {
            draw_step3_header(ctx, 0);
        } else {
            draw_step3_header(ctx, chapter_word_count);
            draw_step3_word_table(ctx, &chapter_words);
        }

        // ===== Chapter end marker =====
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

        // Definition (purple)
        let def = if m.word.definition.is_empty() { "—" } else { &m.word.definition };
        draw_segment(ctx, &mut x, max_x, def, text_purple());

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

        // Blank full-width parens: estimate width from definition length
        let def_len = if m.word.definition.is_empty() {
            4
        } else {
            m.word.definition.chars().count().max(4)
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
