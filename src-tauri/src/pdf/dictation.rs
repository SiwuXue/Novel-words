//! Dictation template: Chinese body text with matched terms replaced by blanks,
//! plus an answer key (English word + Chinese term). Body wraps to page width.
use super::matcher::{extract_cn_terms, find_matches_in_line};
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
            ctx.draw_text(&chapter.title, ctx.margins.left, ctx.current_y, ctx.font_size + 2.0);
            ctx.current_y -= ctx.line_height * 1.5;
        }
        for para in super::split_paragraphs(&chapter.content) {
            if ctx.remaining_height() < ctx.line_height * 3.0 {
                ctx.new_page();
            }
            render_dictation_paragraph(ctx, &para, vocabs);
            ctx.current_y -= ctx.line_height * 0.6;
        }
    }

    // Answer key on a new page
    ctx.new_page();
    ctx.draw_text("参考答案", ctx.margins.left, ctx.current_y, ctx.font_size + 4.0);
    ctx.current_y -= ctx.line_height * 2.0;

    let mut seen = HashSet::new();
    let unique: Vec<&VocabWord> = vocabs
        .iter()
        .filter(|v| seen.insert(v.word.to_lowercase()))
        .collect();

    for w in &unique {
        let term = extract_cn_terms(&w.definition).join("/");
        let term = if term.is_empty() { w.definition.clone() } else { term };
        let line = format!("{}  ({})  {}", w.word, w.phonetic, term);
        ctx.draw_text_wrapped(&line, ctx.margins.left, ctx.usable_width, ctx.small_font_size);
        if ctx.remaining_height() < ctx.line_height * 1.5 {
            ctx.new_page();
        }
    }
}

fn render_dictation_paragraph(ctx: &mut PdfContext, line: &str, vocabs: &[VocabWord]) {
    let matches = find_matches_in_line(line, vocabs);

    if matches.is_empty() {
        ctx.draw_text_wrapped(line, ctx.margins.left, ctx.usable_width, ctx.font_size);
        return;
    }

    let max_x = ctx.margins.left + ctx.usable_width;
    let mut x = ctx.margins.left;
    let mut last = 0usize;

    let draw_segment = |ctx: &mut PdfContext, x: &mut f32, seg: &str| {
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
        if m.start > last {
            let pre = &line[last..m.start];
            draw_segment(ctx, &mut x, pre);
        }
        // Blank underline sized ~ to the term length
        let blank = "_".repeat(m.term_len * 2);
        draw_segment(ctx, &mut x, &blank);
        last = m.end;
    }

    if last < line.len() {
        let rest = &line[last..];
        draw_segment(ctx, &mut x, rest);
    }
    ctx.current_y -= ctx.line_height;
}
