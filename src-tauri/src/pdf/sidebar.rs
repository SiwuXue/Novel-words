//! Sidebar annotation template: left 68% body text, right 32% per-page word list.
use super::PdfContext;
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;
use std::collections::HashMap;

const SIDEBAR_RATIO: f32 = 0.32;
const GAP: f32 = 5.0;

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    let text_width = ctx.usable_width * (1.0 - SIDEBAR_RATIO) - GAP;
    let sidebar_x = ctx.margins.left + text_width + GAP;
    let sidebar_w = ctx.usable_width * SIDEBAR_RATIO;
    let divider_x = sidebar_x - GAP / 2.0;

    let word_map: HashMap<String, &VocabWord> = vocabs
        .iter()
        .map(|v| (v.word.to_lowercase(), v))
        .collect();

    for chapter in chapters {
        let mut page_words: Vec<&VocabWord> = Vec::new();

        for para in chapter.content.split("\n\n") {
            let trimmed = para.trim();
            if trimmed.is_empty() { continue; }
            let single_line = trimmed.replace('\n', " ");

            // Find words in this line
            let found = find_words_in_line(&single_line, &word_map);
            for w in &found {
                let key = w.word.to_lowercase();
                if !page_words.iter().any(|pw| pw.word.to_lowercase() == key) {
                    page_words.push(w);
                }
            }

            // Draw body text in left region
            ctx.draw_text(&single_line, ctx.margins.left, ctx.current_y, ctx.font_size);
            ctx.current_y -= ctx.line_height;

            // Check for page break
            if ctx.remaining_height() < ctx.line_height * 3.0 {
                draw_sidebar(ctx, sidebar_x, sidebar_w, divider_x, &page_words);
                ctx.new_page();
                page_words.clear();
            }
        }

        // Draw remaining words for this chapter
        if !page_words.is_empty() {
            draw_sidebar(ctx, sidebar_x, sidebar_w, divider_x, &page_words);
            page_words.clear();
        }
    }
}

fn draw_sidebar(
    ctx: &mut PdfContext,
    x: f32,
    _width: f32,
    divider_x: f32,
    words: &[&VocabWord],
) {
    // Vertical divider
    ctx.draw_line(
        divider_x,
        ctx.paper_height - ctx.margins.top,
        divider_x,
        ctx.margins.bottom,
    );

    // Sidebar title
    let mut sy = ctx.paper_height - ctx.margins.top;
    ctx.draw_text("本页词汇", x, sy, ctx.small_font_size);
    sy -= 5.0;

    for w in words {
        if sy < ctx.margins.bottom + 5.0 { break; }
        let line = format!("{} — {}", w.word, w.definition);
        ctx.draw_text(&line, x, sy, ctx.small_font_size);
        sy -= ctx.line_height * 0.7;
    }
}

fn find_words_in_line<'a>(
    line: &str,
    word_map: &HashMap<String, &'a VocabWord>,
) -> Vec<&'a VocabWord> {
    let lower = line.to_lowercase();
    let mut found: Vec<&VocabWord> = Vec::new();
    for (key, v) in word_map {
        if lower.contains(key.as_str()) {
            found.push(*v);
        }
    }
    found
}
