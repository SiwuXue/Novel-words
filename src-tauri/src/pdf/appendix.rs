//! Appendix template: clean wrapped body text + a full vocab table at the end.
//!
//! Novels are Chinese, vocab words are English. We render the body cleanly and
//! append a complete word list. Per-chapter tables list the words whose (chosen)
//! Chinese definition actually appears in that chapter's text.
use super::matcher::words_found_in_text;
use super::PdfContext;
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;
use std::collections::HashSet;

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    for (ci, chapter) in chapters.iter().enumerate() {
        // Each chapter starts on a fresh page (except the first, which follows the title page).
        if ci > 0 {
            ctx.new_page();
        }
        // Chapter heading
        if !chapter.title.is_empty() {
            ctx.draw_text(&chapter.title, ctx.margins.left, ctx.current_y, ctx.font_size + 2.0);
            ctx.current_y -= ctx.line_height * 1.5;
            if ctx.remaining_height() < ctx.line_height * 2.0 {
                ctx.new_page();
            }
        }

        // Render clean, wrapped body text
        for para in chapter.content.split("\n\n") {
            let trimmed = para.trim();
            if trimmed.is_empty() { continue; }
            let single_line = trimmed.replace('\n', " ");
            ctx.draw_text_wrapped(&single_line, ctx.margins.left, ctx.usable_width, ctx.font_size);
            ctx.current_y -= ctx.line_height * 0.4;

            if ctx.remaining_height() < ctx.line_height * 2.0 {
                ctx.new_page();
            }
        }

        // Words whose Chinese meaning appears in this chapter
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
    ctx.current_y -= ctx.line_height * 2.0;

    let mut seen = HashSet::new();
    let unique: Vec<&VocabWord> = vocabs
        .iter()
        .filter(|v| seen.insert(v.word.to_lowercase()))
        .collect();
    draw_vocab_table(ctx, &unique);
}

fn draw_vocab_table(ctx: &mut PdfContext, words: &[&VocabWord]) {
    let col1_w = 45.0; // 单词
    let col2_w = 35.0; // 音标
    let col3_w = ctx.usable_width - col1_w - col2_w; // 释义
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
