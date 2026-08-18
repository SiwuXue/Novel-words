//! Intensive reading template: two-pass learning structure.
//! Step 1: English word (by proficiency color) + Chinese definition (purple) inline after the
//!         matched Chinese term is replaced.
//! Step 2: Same content but the definition area is a blank bracket pair so
//!         learners can recall the meaning from context.
use super::matcher::{find_matches_in_line, words_found_in_text};
use super::{text_black, text_gray, text_purple, text_red, text_color_for_proficiency, PdfContext};
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
        let chapter_words = words_found_in_text(&chapter.content, vocabs);

        // Chapter header area
        draw_chapter_header(ctx, chapter_num, &chapter.title, chapter_words.len());

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
