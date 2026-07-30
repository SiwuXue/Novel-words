//! Recitation template: left column Chinese text (wrapped), right column the vocab
//! words whose (chosen) Chinese definition appears in that paragraph.
use super::matcher::words_found_in_text;
use super::PdfContext;
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;

const GAP: f32 = 10.0;

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    let col_width = (ctx.usable_width - GAP) / 2.0;
    let left_x = ctx.margins.left;
    let right_x = left_x + col_width + GAP;
    let divider_x = left_x + col_width + GAP / 2.0;

    for (ci, chapter) in chapters.iter().enumerate() {
        if ci > 0 {
            ctx.new_page();
        }
        let heading = format!("{} · 单词对照背诵", chapter.title);
        ctx.draw_text(&heading, ctx.margins.left, ctx.current_y, ctx.font_size + 2.0);
        ctx.current_y -= ctx.line_height * 2.0;

        draw_divider(ctx, divider_x);

        for para in chapter.content.split("\n\n") {
            let trimmed = para.trim();
            if trimmed.is_empty() { continue; }
            let single_line = trimmed.replace('\n', " ");

            let found = words_found_in_text(&single_line, vocabs);

            // Left: wrapped Chinese paragraph within the left column
            let start_y = ctx.current_y;
            ctx.draw_text_wrapped(&single_line, left_x, col_width, ctx.font_size);
            let left_end_y = ctx.current_y;

            // Right: words found in this paragraph, aligned to the paragraph top
            let mut ry = start_y;
            for w in &found {
                if ry < ctx.margins.bottom {
                    break;
                }
                let label = format!("{}  {}", w.word, w.definition);
                let display = ctx.truncate_text(&label, col_width, ctx.small_font_size);
                ctx.draw_text(&display, right_x, ry, ctx.small_font_size);
                ry -= ctx.line_height * 0.9;
            }

            // Advance current_y past whichever column is taller
            let right_end_y = start_y - found.len() as f32 * ctx.line_height * 0.9;
            ctx.current_y = left_end_y.min(right_end_y) - ctx.line_height * 0.4;

            if ctx.remaining_height() < ctx.line_height * 3.0 {
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
