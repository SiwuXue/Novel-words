//! Recitation template: left column Chinese text, right column vocab words.
use super::PdfContext;
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;
use std::collections::HashMap;

const GAP: f32 = 10.0;

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    let col_width = (ctx.usable_width - GAP) / 2.0;
    let left_x = ctx.margins.left;
    let right_x = left_x + col_width + GAP;
    let divider_x = left_x + col_width + GAP / 2.0;

    // Build word lookup
    let word_map: HashMap<String, &VocabWord> = vocabs
        .iter()
        .map(|v| (v.word.to_lowercase(), v))
        .collect();

    for chapter in chapters {
        // Chapter heading
        let heading = format!("{} · 单词对照背诵", chapter.title);
        ctx.draw_text(&heading, ctx.margins.left, ctx.current_y, ctx.font_size + 2.0);
        ctx.current_y -= ctx.line_height * 2.0;

        // Vertical divider
        ctx.draw_line(
            divider_x,
            ctx.margins.top,
            divider_x,
            ctx.paper_height - ctx.margins.bottom,
        );

        for para in chapter.content.split("\n\n") {
            let trimmed = para.trim();
            if trimmed.is_empty() { continue; }
            let single_line = trimmed.replace('\n', " ");

            // Left: Chinese paragraph
            let left_y = ctx.current_y;
            ctx.draw_text(&single_line, left_x, left_y, ctx.font_size);

            // Right: words found in this paragraph
            let found = find_words_in_line(&single_line, &word_map);
            let mut ry = left_y;
            for w in &found {
                let line = format!("{}  {}", w.word, w.definition);
                ctx.draw_text(&line, right_x, ry, ctx.small_font_size);
                ry -= ctx.line_height * 0.8;
            }

            let max_h = ctx.line_height.max((found.len() as f32) * ctx.line_height * 0.8);
            ctx.current_y -= max_h;

            if ctx.remaining_height() < ctx.line_height * 3.0 {
                ctx.new_page();
                // Redraw divider on new page
                ctx.draw_line(
                    divider_x,
                    ctx.margins.top,
                    divider_x,
                    ctx.paper_height - ctx.margins.bottom,
                );
            }
        }
    }
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
            found.push(*v);
        }
    }
    found
}
