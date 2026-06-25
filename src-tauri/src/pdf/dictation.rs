//! Dictation template: Chinese text with blank lines replacing vocab words.
use super::PdfContext;
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;
use std::collections::{HashMap, HashSet};

pub fn render(ctx: &mut PdfContext, chapters: &[Chapter], vocabs: &[VocabWord]) {
    let word_map: HashMap<String, &VocabWord> = vocabs
        .iter()
        .map(|v| (v.word.to_lowercase(), v))
        .collect();

    let mut sorted: Vec<&VocabWord> = vocabs.iter().collect();
    sorted.sort_by(|a, b| b.word.len().cmp(&a.word.len()));

    for chapter in chapters {
        for para in chapter.content.split("\n\n") {
            let trimmed = para.trim();
            if trimmed.is_empty() { continue; }
            let single_line = trimmed.replace('\n', " ");

            render_dictation_line(ctx, &single_line, &word_map, &sorted);
            ctx.current_y -= ctx.line_height;
            // Extra space for handwriting
            ctx.current_y -= ctx.line_height * 0.5;

            if ctx.remaining_height() < ctx.line_height * 3.0 {
                ctx.new_page();
            }
        }
    }

    // Answer key on new page
    ctx.new_page();
    ctx.draw_text("参考答案", ctx.margins.left, ctx.current_y, ctx.font_size + 4.0);
    ctx.current_y -= ctx.line_height * 2.0;

    let mut seen = HashSet::new();
    let unique: Vec<&VocabWord> = vocabs
        .iter()
        .filter(|v| seen.insert(v.word.to_lowercase()))
        .collect();

    for w in &unique {
        let ph = if w.phonetic.is_empty() { "—" } else { &w.phonetic };
        let def = if w.definition.is_empty() { "—" } else { &w.definition };
        let line = format!("{}  {}  {}", w.word, ph, def);
        ctx.draw_text(&line, ctx.margins.left, ctx.current_y, ctx.small_font_size);
        ctx.current_y -= ctx.line_height * 0.7;
        if ctx.remaining_height() < ctx.line_height {
            ctx.new_page();
        }
    }
}

fn render_dictation_line(
    ctx: &mut PdfContext,
    line: &str,
    _word_map: &HashMap<String, &VocabWord>,
    sorted: &[&VocabWord],
) {
    let lower = line.to_lowercase();
    let mut matches: Vec<(usize, usize, &VocabWord)> = Vec::new();

    // Find all word matches
    for v in sorted {
        let word_lower = v.word.to_lowercase();
        let mut start = 0;
        while let Some(pos) = lower[start..].find(&word_lower) {
            let abs_pos = start + pos;
            let end = abs_pos + v.word.len();
            let left_ok = abs_pos == 0;
            let right_ok = end >= lower.len();
            if left_ok && right_ok {
                matches.push((abs_pos, end, v));
            }
            start = end;
        }
    }
    matches.sort_by_key(|m| m.0);

    // Remove overlapping
    let mut filtered: Vec<(usize, usize, &VocabWord)> = Vec::new();
    for m in matches {
        if !filtered.iter().any(|f| m.0 < f.1 && f.0 < m.1) {
            filtered.push(m);
        }
    }

    let mut x = ctx.margins.left;
    let mut last = 0;

    for (start, end, _v) in &filtered {
        // Text before match
        if *start > last {
            let pre = &line[last..*start];
            ctx.draw_text(pre, x, ctx.current_y, ctx.font_size);
            x += ctx.measure_text_width(pre, ctx.font_size);
        }

        // Blank underline
        let blank = "_".repeat((end - start) * 2); // ~2 underscores per char
        ctx.draw_text(&blank, x, ctx.current_y, ctx.font_size);
        x += ctx.measure_text_width(&blank, ctx.font_size);

        last = *end;
    }

    // Remaining text
    if last < line.len() {
        ctx.draw_text(&line[last..], x, ctx.current_y, ctx.font_size);
    }
}
