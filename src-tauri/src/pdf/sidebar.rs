//! Sidebar annotation template: left ~65% wrapped body text, right ~35% lists the
//! vocab words whose (chosen) Chinese definition appears in each paragraph.
use super::matcher::words_found_in_text;
use super::PdfContext;
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;

const SIDEBAR_RATIO: f32 = 0.35;
const GAP: f32 = 4.0;

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    let text_width = ctx.usable_width * (1.0 - SIDEBAR_RATIO) - GAP;
    let sidebar_x = ctx.margins.left + ctx.usable_width * (1.0 - SIDEBAR_RATIO) + GAP;
    let sidebar_w = ctx.usable_width * SIDEBAR_RATIO;
    let divider_x = ctx.margins.left + ctx.usable_width * (1.0 - SIDEBAR_RATIO) + GAP / 2.0;

    draw_divider(ctx, divider_x);

    for (ci, chapter) in chapters.iter().enumerate() {
        if ci > 0 {
            ctx.new_page_for_chapter();
            draw_divider(ctx, divider_x);
        }
        if !chapter.title.is_empty() {
            ctx.draw_text(&chapter.title, ctx.margins.left, ctx.current_y, ctx.font_size + 2.0);
            ctx.current_y -= ctx.line_height * 1.5;
        }
        for single_line in super::split_paragraphs(&chapter.content) {
            // Words whose meaning appears in this paragraph
            let found = words_found_in_text(&single_line, vocabs);

            // Draw wrapped body text on the left
            let start_y = ctx.current_y;
            ctx.draw_text_wrapped(&single_line, ctx.margins.left, text_width, ctx.font_size);
            let end_y = ctx.current_y;

            // Draw vocab annotations on the right, aligned with the body block
            let mut sy = start_y;
            for w in &found {
                if sy < ctx.margins.bottom {
                    break;
                }
                let label = format!("{} {}", w.word, w.definition);
                let display = ctx.truncate_text(&label, sidebar_w, ctx.small_font_size);
                ctx.draw_text(&display, sidebar_x, sy, ctx.small_font_size);
                sy -= ctx.line_height * 0.8;
            }

            // If the sidebar list is taller than the body block, advance further.
            if !found.is_empty() {
                let vocab_end_y = start_y - found.len() as f32 * ctx.line_height * 0.8;
                if vocab_end_y < end_y {
                    ctx.current_y = vocab_end_y;
                }
            }

            ctx.current_y -= ctx.line_height * 0.4;

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
        ctx.paper_height - ctx.margins.top,
        divider_x,
        ctx.margins.bottom,
    );
}
