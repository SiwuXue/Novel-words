//! Intensive reading template: inline English-word annotation after the Chinese
//! term whose (chosen) definition matches. Body text wraps to page width.
use super::matcher::find_matches_in_line;
use super::PdfContext;
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    for (ci, chapter) in chapters.iter().enumerate() {
        if ci > 0 {
            ctx.new_page_for_chapter();
        }
        if !chapter.title.is_empty() {
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
    }
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
