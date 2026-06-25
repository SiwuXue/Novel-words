//! Appendix template: clean body text + per-chapter word table at end of each chapter.
use super::PdfContext;
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;
use std::collections::HashSet;

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    for chapter in chapters {
        // Render clean body text
        for para in chapter.content.split("\n\n") {
            let trimmed = para.trim();
            if trimmed.is_empty() { continue; }
            let single_line = trimmed.replace('\n', " ");
            ctx.draw_text(&single_line, ctx.margins.left, ctx.current_y, ctx.font_size);
            ctx.current_y -= ctx.line_height;

            if ctx.remaining_height() < ctx.line_height * 2.0 {
                ctx.new_page();
            }
        }

        // Find words for this chapter
        let chapter_words: Vec<&VocabWord> = vocabs
            .iter()
            .filter(|v| v.chapter_id.map_or(false, |cid| cid == chapter.id))
            .collect();

        if !chapter_words.is_empty() {
            ctx.new_page();
            let heading = format!("{} — 单词表", chapter.title);
            ctx.draw_text(&heading, ctx.margins.left, ctx.current_y, ctx.font_size + 2.0);
            ctx.current_y -= ctx.line_height * 2.0;
            draw_vocab_table(ctx, &chapter_words);
        }
    }

    // Final appendix: all words deduplicated
    ctx.new_page();
    ctx.draw_text("全文总词汇表", ctx.margins.left, ctx.current_y, ctx.font_size + 4.0);
    ctx.current_y -= ctx.line_height * 2.0;

    let mut seen = HashSet::new();
    let unique: Vec<&VocabWord> = vocabs
        .iter()
        .filter(|v| seen.insert(v.word.to_lowercase()))
        .collect();
    draw_vocab_table(ctx, &unique);
}

fn draw_vocab_table(ctx: &mut PdfContext, words: &[&VocabWord]) {
    let col1_w = 50.0; // 单词
    let col2_w = 40.0; // 音标
    let col3_w = ctx.usable_width - col1_w - col2_w; // 释义
    let row_h = 8.0;
    let x = ctx.margins.left;
    let mut y = ctx.current_y;

    // Header
    ctx.draw_rect_border(x, y, col1_w, row_h);
    ctx.draw_text("单词", x + 2.0, y - 2.0, ctx.small_font_size);
    ctx.draw_rect_border(x + col1_w, y, col2_w, row_h);
    ctx.draw_text("音标", x + col1_w + 2.0, y - 2.0, ctx.small_font_size);
    ctx.draw_rect_border(x + col1_w + col2_w, y, col3_w, row_h);
    ctx.draw_text("释义", x + col1_w + col2_w + 2.0, y - 2.0, ctx.small_font_size);
    y -= row_h;

    // Data rows
    for w in words {
        if y - row_h < ctx.margins.bottom {
            break; // Stop if out of space
        }
        ctx.draw_rect_border(x, y, col1_w, row_h);
        ctx.draw_text(&w.word, x + 2.0, y - 2.0, ctx.small_font_size);
        ctx.draw_rect_border(x + col1_w, y, col2_w, row_h);
        let ph = if w.phonetic.is_empty() { "—" } else { &w.phonetic };
        ctx.draw_text(ph, x + col1_w + 2.0, y - 2.0, ctx.small_font_size);
        ctx.draw_rect_border(x + col1_w + col2_w, y, col3_w, row_h);
        let def = if w.definition.is_empty() { "—" } else { &w.definition };
        ctx.draw_text(def, x + col1_w + col2_w + 2.0, y - 2.0, ctx.small_font_size);
        y -= row_h;
    }
}
