//! Intensive reading template: inline small-font annotation after each vocab word.
use super::PdfContext;
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    // Sort vocabs by word length descending for longest-match-first
    let mut sorted: Vec<&VocabWord> = vocabs.iter().collect();
    sorted.sort_by(|a, b| b.word.len().cmp(&a.word.len()));

    let word_map: std::collections::HashMap<String, &VocabWord> = sorted
        .iter()
        .map(|v| (v.word.to_lowercase(), *v))
        .collect();

    for chapter in chapters {
        for para in chapter.content.split("\n\n") {
            let trimmed = para.trim();
            if trimmed.is_empty() {
                continue;
            }
            let single_line = trimmed.replace('\n', " ");

            render_annotated_line(ctx, &single_line, &word_map, &sorted);
            ctx.current_y -= ctx.line_height;

            if ctx.remaining_height() < ctx.line_height * 2.0 {
                ctx.new_page();
            }
        }
    }
}

fn render_annotated_line(
    ctx: &mut PdfContext,
    line: &str,
    word_map: &std::collections::HashMap<String, &VocabWord>,
    _sorted: &[&VocabWord],
) {
    let lower = line.to_lowercase();
    let mut matches: Vec<(usize, usize, &VocabWord)> = Vec::new();

    // Find all word matches
    for v in word_map.values() {
        let word_lower = v.word.to_lowercase();
        if word_lower.is_empty() {
            continue;
        }
        let mut start = 0;
        while let Some(pos) = lower[start..].find(&word_lower) {
            let abs_pos = start + pos;
            let end = abs_pos + v.word.len();
            // For CJK: treat as boundary-free, match at any position
            let left_ok = abs_pos == 0
                || !lower[..abs_pos].ends_with(|c: char| c.is_alphanumeric());
            let right_ok = end >= lower.len()
                || !lower[end..].starts_with(|c: char| c.is_alphanumeric());
            if left_ok && right_ok {
                matches.push((abs_pos, end, v));
            }
            // Guard against empty-word infinite loop (end == start)
            if end <= start {
                break;
            }
            start = end;
        }
    }
    matches.sort_by_key(|m| m.0);

    // Remove overlapping matches
    let mut filtered: Vec<(usize, usize, &VocabWord)> = Vec::new();
    for m in matches {
        if !filtered.iter().any(|f| m.0 < f.1 && f.0 < m.1) {
            filtered.push(m);
        }
    }

    if filtered.is_empty() {
        ctx.draw_text(line, ctx.margins.left, ctx.current_y, ctx.font_size);
        return;
    }

    let mut x = ctx.margins.left;
    let mut last = 0;
    for (start, end, v) in &filtered {
        // Draw text before match
        if *start > last {
            let pre = &line[last..*start];
            let w = ctx.measure_text_width(pre, ctx.font_size);
            ctx.draw_text(pre, x, ctx.current_y, ctx.font_size);
            x += w;
        }

        // Draw the matched word
        let matched_text = &line[*start..*end];
        let mw = ctx.measure_text_width(matched_text, ctx.font_size);
        ctx.draw_text(matched_text, x, ctx.current_y, ctx.font_size);

        // Draw annotation as small superscript
        let def = if v.phonetic.is_empty() {
            v.definition.clone()
        } else {
            format!("/{}/ {}", v.phonetic, v.definition)
        };
        let ann = format!("【{}】", def);
        ctx.draw_text(&ann, x + mw, ctx.current_y + 1.0, ctx.small_font_size);
        x += mw + ctx.measure_text_width(&ann, ctx.small_font_size);

        last = *end;
    }

    // Draw remaining text
    if last < line.len() {
        ctx.draw_text(&line[last..], x, ctx.current_y, ctx.font_size);
    }
}
