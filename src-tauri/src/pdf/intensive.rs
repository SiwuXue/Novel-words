//! Intensive reading template: inline English-word annotation after the Chinese
//! term whose (chosen) definition matches, plus a full vocab table at the end.
use super::matcher::{find_matches_in_line, words_found_in_text};
use super::PdfContext;
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;
use std::collections::HashSet;

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    for (ci, chapter) in chapters.iter().enumerate() {
        if ci > 0 {
            ctx.new_page_for_chapter();
        }
        if !chapter.title.is_empty() {
            ctx.record_bookmark(&chapter.title);
            ctx.draw_text(&chapter.title, ctx.margins.left, ctx.current_y, ctx.font_size + 2.0);
            ctx.current_y -= ctx.line_height * 1.5;
        }
        for para in super::split_paragraphs(&chapter.content) {
            if ctx.remaining_height() < ctx.line_height * 2.0 {
                ctx.new_page();
            }
            render_annotated_paragraph(ctx, &para, vocabs);
            ctx.current_y -= ctx.line_height * 0.4;
        }

        // Per-chapter word list (words whose Chinese meaning appears in this chapter)
        let chapter_words = words_found_in_text(&chapter.content, vocabs);
        if !chapter_words.is_empty() {
            if ctx.remaining_height() < ctx.line_height * 6.0 {
                ctx.new_page();
            }
            ctx.current_y -= ctx.line_height;
            let heading = format!("{} — 生词表", chapter.title);
            ctx.draw_text(&heading, ctx.margins.left, ctx.current_y, ctx.font_size + 1.0);
            ctx.current_y -= ctx.line_height * 1.5;
            draw_vocab_table(ctx, &chapter_words);
        }
    }

    // Final appendix: all words deduplicated
    ctx.new_page();
    ctx.draw_text("全文总词汇表", ctx.margins.left, ctx.current_y, ctx.font_size + 4.0);
    ctx.record_bookmark("全文总词汇表");
    ctx.current_y -= ctx.line_height * 2.0;

    let mut seen = HashSet::new();
    let unique: Vec<&VocabWord> = vocabs
        .iter()
        .filter(|v| seen.insert(v.word.to_lowercase()))
        .collect();
    draw_vocab_table(ctx, &unique);
}

/// Render one paragraph, wrapping to page width, drawing a small superscript
/// English word after each matched Chinese term.
fn render_annotated_paragraph(ctx: &mut PdfContext, line: &str, vocabs: &[VocabWord]) {
    let matches = find_matches_in_line(line, vocabs);

    // No matches: fall back to plain wrapped text.
    if matches.is_empty() {
        ctx.draw_text_wrapped(line, ctx.margins.left, ctx.usable_width, ctx.font_size);
        return;
    }

    let max_x = ctx.margins.left + ctx.usable_width;
    let mut x = ctx.margins.left;
    let mut last = 0usize;

    // Helper closure can't borrow ctx mutably twice, so inline the wrapping logic.
    let draw_segment = |ctx: &mut PdfContext, x: &mut f32, seg: &str| {
        // Draw char by char so we can wrap mid-segment.
        for ch in seg.chars() {
            let cw = ctx.measure_text_width(&ch.to_string(), ctx.font_size);
            if *x + cw > max_x {
                ctx.current_y -= ctx.line_height;
                *x = ctx.margins.left;
                if ctx.remaining_height() < ctx.line_height * 2.0 {
                    ctx.new_page();
                }
            }
            ctx.draw_text(&ch.to_string(), *x, ctx.current_y, ctx.font_size);
            *x += cw;
        }
    };

    for m in &matches {
        // Text before the match
        if m.start > last {
            let pre = &line[last..m.start];
            draw_segment(ctx, &mut x, pre);
        }
        // The matched Chinese term
        let matched = &line[m.start..m.end];
        draw_segment(ctx, &mut x, matched);

        // Annotation: the English word (+ phonetic) as small superscript
        let ann = if m.word.phonetic.is_empty() {
            format!("[{}]", m.word.word)
        } else {
            format!("[{} /{}/]", m.word.word, m.word.phonetic)
        };
        let aw = ctx.measure_text_width(&ann, ctx.small_font_size);
        if x + aw > max_x {
            ctx.current_y -= ctx.line_height;
            x = ctx.margins.left;
            if ctx.remaining_height() < ctx.line_height * 2.0 {
                ctx.new_page();
            }
        }
        ctx.draw_text(&ann, x, ctx.current_y + 1.0, ctx.small_font_size);
        x += aw;

        last = m.end;
    }

    // Remaining text after the last match
    if last < line.len() {
        let rest = &line[last..];
        draw_segment(ctx, &mut x, rest);
    }
    ctx.current_y -= ctx.line_height;
}

/// Simple 3-column word table: 单词 | 音标 | 释义
fn draw_vocab_table(ctx: &mut PdfContext, words: &[&VocabWord]) {
    let col1_w = 45.0;
    let col2_w = 35.0;
    let col3_w = ctx.usable_width - col1_w - col2_w;
    let row_h = 8.0;
    let x = ctx.margins.left;

    let draw_header = |ctx: &mut PdfContext| {
        let y = ctx.current_y;
        ctx.draw_rect_border(x, y, col1_w, row_h);
        ctx.draw_text("单词", x + 2.0, y - 2.0, ctx.small_font_size);
        ctx.draw_rect_border(x + col1_w, y, col2_w, row_h);
        ctx.draw_text("音标", x + col1_w + 2.0, y - 2.0, ctx.small_font_size);
        ctx.draw_rect_border(x + col1_w + col2_w, y, col3_w, row_h);
        ctx.draw_text("释义", x + col1_w + col2_w + 2.0, y - 2.0, ctx.small_font_size);
        ctx.current_y -= row_h;
    };

    draw_header(ctx);

    for w in words {
        if ctx.current_y - row_h < ctx.margins.bottom {
            ctx.new_page();
            draw_header(ctx);
        }
        let y = ctx.current_y;
        ctx.draw_rect_border(x, y, col1_w, row_h);
        let word_disp = ctx.truncate_text(&w.word, col1_w - 4.0, ctx.small_font_size);
        ctx.draw_text(&word_disp, x + 2.0, y - 2.0, ctx.small_font_size);

        ctx.draw_rect_border(x + col1_w, y, col2_w, row_h);
        let ph_raw = if w.phonetic.is_empty() { "—" } else { &w.phonetic };
        let ph = ctx.truncate_text(ph_raw, col2_w - 4.0, ctx.small_font_size);
        ctx.draw_text(&ph, x + col1_w + 2.0, y - 2.0, ctx.small_font_size);

        ctx.draw_rect_border(x + col1_w + col2_w, y, col3_w, row_h);
        let def_raw = if w.definition.is_empty() { "—" } else { &w.definition };
        let def = ctx.truncate_text(def_raw, col3_w - 4.0, ctx.small_font_size);
        ctx.draw_text(&def, x + col1_w + col2_w + 2.0, y - 2.0, ctx.small_font_size);
        ctx.current_y -= row_h;
    }
}
