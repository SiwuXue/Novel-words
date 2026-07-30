//! Sidebar annotation template: left ~65% body text, right ~35% inline word annotations.
//! Each paragraph's found vocab words are listed beside the paragraph on the right side,
//! avoiding overlap between body text and sidebar annotations.
use super::PdfContext;
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;
use std::collections::HashMap;

const SIDEBAR_RATIO: f32 = 0.35;
const GAP: f32 = 4.0;

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    let text_width = ctx.usable_width * (1.0 - SIDEBAR_RATIO) - GAP;
    let sidebar_x = ctx.margins.left + ctx.usable_width * (1.0 - SIDEBAR_RATIO) + GAP;
    let sidebar_w = ctx.usable_width * SIDEBAR_RATIO;
    let divider_x = ctx.margins.left + ctx.usable_width * (1.0 - SIDEBAR_RATIO) + GAP / 2.0;

    let word_map: HashMap<String, &VocabWord> = vocabs
        .iter()
        .map(|v| (v.word.to_lowercase(), v))
        .collect();

    // Draw divider on the first content page
    draw_divider(ctx, divider_x);

    for chapter in chapters {
        for para in chapter.content.split("\n\n") {
            let trimmed = para.trim();
            if trimmed.is_empty() {
                continue;
            }
            let single_line = trimmed.replace('\n', " ");

            // Find words in this paragraph (deduplicated)
            let found = find_words_in_line(&single_line, &word_map);

            // Calculate how many sidebar lines the vocab words will need
            let sidebar_lines_needed = found.len();

            // Draw wrapped body text on the left, remembering the y range it occupies
            let start_y = ctx.current_y;
            let lines_drawn = ctx.draw_text_wrapped(&single_line, ctx.margins.left, text_width, ctx.font_size);
            let end_y = ctx.current_y;

            // Draw vocab annotations on the right side, aligned with the body text block
            let mut sy = start_y;
            for w in &found {
                if sy < ctx.margins.bottom {
                    break;
                }
                let label = format!("{} {}", w.word, w.definition);
                let display = ctx.truncate_text(&label, sidebar_w, ctx.small_font_size);
                ctx.draw_text(&display, sidebar_x, sy, ctx.small_font_size);
                sy -= ctx.line_height * 0.7;
            }

            // If vocab words extend below the body text block, advance current_y further
            if sidebar_lines_needed > 0 {
                let vocab_end_y = start_y - sidebar_lines_needed as f32 * ctx.line_height * 0.7;
                if vocab_end_y < end_y {
                    ctx.current_y = vocab_end_y;
                }
            }

            // Add small spacing between paragraphs
            ctx.current_y -= ctx.line_height * 0.3;

            // Check for page break — need enough space for at least 2 body lines
            if ctx.remaining_height() < ctx.line_height * 2.5 {
                ctx.new_page();
                draw_divider(ctx, divider_x);
            }
        }
    }
}

fn draw_divider(ctx: &mut PdfContext, divider_x: f32) {
    ctx.draw_line(
        divider_x,
        ctx.margins.top,
        divider_x,
        ctx.paper_height - ctx.margins.bottom,
    );
}

fn find_words_in_line<'a>(
    line: &str,
    word_map: &HashMap<String, &'a VocabWord>,
) -> Vec<&'a VocabWord> {
    let lower = line.to_lowercase();
    let mut found: Vec<&VocabWord> = Vec::new();
    for (key, v) in word_map {
        if key.is_empty() {
            continue;
        }
        if lower.contains(key.as_str()) {
            let key_lower = v.word.to_lowercase();
            if !found.iter().any(|f| f.word.to_lowercase() == key_lower) {
                found.push(*v);
            }
        }
    }
    found
}
