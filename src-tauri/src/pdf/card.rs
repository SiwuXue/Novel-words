//! Word-card reading template ("单词卡片版").
//!
//! Mirrors the reference vocabulary sheet layout:
//!   * Left column  — the English novel text, target words colored by proficiency
//!     with a short Chinese gloss noted inline (no whole-sentence translation).
//!   * Right column — a vertical stack of pastel "word cards" (word / phonetic /
//!     part-of-speech + senses), cycling a rainbow of background tints.
//!   * Page-1 header — book title + a stat strip (单词数 / 生疏 / 熟悉 / 掌握).
//!
//! English-only: the Chinese terms that the matcher works from aren't present in
//! an English novel, so this template requires `language == "en"`. The command
//! layer rejects Chinese novels before rendering reaches here.

use super::matcher::{find_matches_in_line_en, words_found_in_text_en};
use super::{
    split_paragraphs, table_border, text_black, text_gray, text_light_gray,
    text_color_for_proficiency, wrap_text_to_lines, PdfContext,
};
use crate::models::novel::Chapter;
use crate::models::vocab_word::VocabWord;
use printpdf::{Color, Rgb};

/// Vertical gap (mm) between adjacent word cards.
const CARD_GAP: f32 = 2.0;
/// Horizontal padding (mm) inside a card.
const CARD_PAD: f32 = 1.6;
/// Corner radius (mm) for the rounded word cards.
const CARD_RADIUS: f32 = 2.0;

/// Accent blue used for titles / chapter labels.
fn accent() -> Color {
    Color::Rgb(Rgb::new(
        0x1A as f32 / 255.0,
        0x56 as f32 / 255.0,
        0xDB as f32 / 255.0,
        None,
    ))
}

/// Pastel background for a card, cycling by index so the column gets the
/// rainbow look from the reference screenshot. Saturated enough to read as a
/// visible card box (not near-white).
fn card_color(i: usize) -> Color {
    let palette: [(u8, u8, u8); 8] = [
        (0xBB, 0xDE, 0xFB), // blue
        (0xC8, 0xE6, 0xC9), // green
        (0xF8, 0xBB, 0xD0), // pink
        (0xE1, 0xBE, 0xE7), // purple
        (0xFF, 0xCC, 0x80), // orange
        (0xB2, 0xDF, 0xDB), // teal
        (0xFF, 0xE0, 0x82), // yellow
        (0xD7, 0xCC, 0xC8), // gray-brown
    ];
    let (r, g, b) = palette[i % palette.len()];
    Color::Rgb(Rgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, None))
}

/// Saturated accent color for the word itself (same palette index as the card
/// background), so each word reads as clearly highlighted.
fn card_word_color(i: usize) -> Color {
    let palette: [(u8, u8, u8); 8] = [
        (0x15, 0x65, 0xC0), // blue
        (0x2E, 0x7D, 0x32), // green
        (0xC2, 0x18, 0x5B), // pink
        (0x7B, 0x1F, 0xA2), // purple
        (0xE6, 0x51, 0x00), // orange
        (0x00, 0x83, 0x8F), // teal
        (0xF9, 0xA8, 0x25), // amber
        (0x5D, 0x40, 0x37), // brown
    ];
    let (r, g, b) = palette[i % palette.len()];
    Color::Rgb(Rgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, None))
}

/// One renderable piece of left-column text.
#[derive(Clone)]
enum Seg {
    /// Plain English text.
    Text(String),
    /// A matched target word plus its inline Chinese gloss, drawn in `color`.
    Word { text: String, gloss: String, color: Color },
}

/// Heuristic: does this paragraph look like a chapter heading?
/// Matches "Chapter 1: Title", "CHAPTER 2", "第 1 章 前言", "第3章" etc.
fn is_chapter_marker(para: &str) -> bool {
    let t = para.trim();
    if t.is_empty() || t.len() < 3 {
        return false;
    }
    let lower = t.to_lowercase();
    if lower.starts_with("chapter") {
        let rest = &lower["chapter".len()..].trim_start();
        return rest.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false);
    }
    if t.starts_with('第') {
        return t.chars().any(|c| c == '章')
            && t.chars().any(|c| c.is_ascii_digit() || "一二三四五六七八九十".contains(c));
    }
    false
}

/// Split a chapter's paragraphs into sections at chapter-marker paragraphs.
/// Returns (section_title, section_body_paragraphs). The title is the marker
/// line; the intro (before the first marker) has an empty title.
fn split_marker_sections(paras: &[String]) -> Vec<(String, Vec<String>)> {
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut cur_title = String::new();
    let mut cur_body: Vec<String> = Vec::new();
    for para in paras {
        if is_chapter_marker(para) {
            if !cur_title.is_empty() || !cur_body.is_empty() {
                sections.push((std::mem::take(&mut cur_title), std::mem::take(&mut cur_body)));
            }
            cur_title = para.clone();
        } else {
            cur_body.push(para.clone());
        }
    }
    if !cur_title.is_empty() || !cur_body.is_empty() {
        sections.push((cur_title, cur_body));
    }
    sections
}

/// Light grid-line color for the page background.
fn grid_color() -> Color {
    Color::Rgb(Rgb::new(0xEC as f32 / 255.0, 0xEF as f32 / 255.0, 0xF1 as f32 / 255.0, None))
}

/// Draw a subtle grid across the whole page (notebook feel).
fn draw_grid(ctx: &mut PdfContext) {
    let spacing = 5.0; // mm
    let color = grid_color();
    let mut x = 0.0f32;
    while x <= ctx.paper_width + 0.01 {
        ctx.draw_vline(x, 0.0, ctx.paper_height, color.clone(), 0.3);
        x += spacing;
    }
    let mut y = 0.0f32;
    while y <= ctx.paper_height + 0.01 {
        ctx.draw_hline(0.0, ctx.paper_width, y, color.clone(), 0.3);
        y += spacing;
    }
}

/// Start a fresh page and lay down the grid background behind the content.
fn new_card_page(ctx: &mut PdfContext) {
    ctx.new_page();
    draw_grid(ctx);
}

/// Split a paragraph into sentences (breaking on `.`/`!`/`?` followed by
/// whitespace/end, skipping decimals and common abbreviations).
fn split_sentences(para: &str) -> Vec<String> {
    let chars: Vec<char> = para.chars().collect();
    let mut sentences = Vec::new();
    let mut cur = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        cur.push(c);
        if c == '.' || c == '!' || c == '?' {
            let next_is_ws = i + 1 >= chars.len()
                || chars[i + 1] == ' '
                || chars[i + 1] == '\t'
                || chars[i + 1] == '\n';
            let prev_is_digit = i > 0 && chars[i - 1].is_ascii_digit();
            if next_is_ws && !prev_is_digit {
                let s = cur.trim().to_string();
                if !s.is_empty() {
                    sentences.push(s);
                }
                cur.clear();
            }
        }
        i += 1;
    }
    let s = cur.trim().to_string();
    if !s.is_empty() {
        sentences.push(s);
    }
    sentences
}

/// Group sentences 1-2 per block for the airy reading layout.
fn sentence_groups(para: &str) -> Vec<String> {
    let sentences = split_sentences(para);
    if sentences.len() <= 1 {
        return sentences;
    }
    let mut groups = Vec::new();
    let mut i = 0;
    while i < sentences.len() {
        if i + 1 < sentences.len() {
            groups.push(format!("{} {}", sentences[i], sentences[i + 1]));
            i += 2;
        } else {
            groups.push(sentences[i].clone());
            i += 1;
        }
    }
    groups
}

pub fn render(
    ctx: &mut PdfContext,
    chapters: &[Chapter],
    vocabs: &[VocabWord],
    _language: &str,
    progress: Option<&dyn Fn(super::PdfProgress)>,
) {
    // Whole-book matched-word stats for the page-1 header strip.
    let mut all_matched: Vec<&VocabWord> = Vec::new();
    for ch in chapters {
        for w in words_found_in_text_en(&ch.content, vocabs) {
            let key = w.word.to_lowercase();
            if !all_matched.iter().any(|x| x.word.to_lowercase() == key) {
                all_matched.push(w);
            }
        }
    }
    let total = all_matched.len();
    let unknown = all_matched.iter().filter(|w| w.proficiency == "unknown").count();
    let familiar = all_matched.iter().filter(|w| w.proficiency == "familiar").count();
    let mastered = all_matched.iter().filter(|w| w.proficiency == "mastered").count();

    // Page-1 header. `top` is a bottom-based baseline cursor; we advance it down.
    let mut top = ctx.paper_height - ctx.margins.top;
    draw_grid(ctx);
    draw_global_header(ctx, &mut top, total, unknown, familiar, mastered);

    let mut is_first_section = true;
    for (_ci, chapter) in chapters.iter().enumerate() {
        let paras = split_paragraphs(&chapter.content);
        let mut sections = split_marker_sections(&paras);
        if sections.is_empty() {
            sections.push((chapter.title.clone(), paras.clone()));
        }

        for (sec_title, sec_body) in sections.iter() {
            if let Some(p) = progress {
                let shown = if sec_title.is_empty() {
                    chapter.title.clone()
                } else {
                    sec_title.clone()
                };
                let shown = if shown.is_empty() { "全文".to_string() } else { shown };
                p(super::PdfProgress {
                    percent: 20,
                    message: format!("正在生成：{}", shown),
                });
            }

            // The very first section continues on page 1 below the global header.
            if !is_first_section {
                new_card_page(ctx);
                top = ctx.paper_height - ctx.margins.top;
            }
            is_first_section = false;

            let body_str = sec_body.join("\n");
            let section_words = words_found_in_text_en(&body_str, vocabs);
            let header_title = if sec_title.is_empty() {
                chapter.title.clone()
            } else {
                sec_title.clone()
            };

            draw_chapter_header(ctx, &mut top, &header_title, section_words.len());
            if !header_title.is_empty() {
                ctx.record_bookmark(&header_title);
            }

            // Break the left text into 1-2 sentence blocks for an airy layout.
            let display_paras: Vec<String> = sec_body.iter().flat_map(|p| sentence_groups(p)).collect();
            render_dual_columns(ctx, &display_paras, &section_words, vocabs, top);
        }
    }
}

/// Page-1 header: title + author/tagline + a stat strip, then a rule.
fn draw_global_header(
    ctx: &mut PdfContext,
    y: &mut f32,
    total: usize,
    unknown: usize,
    familiar: usize,
    mastered: usize,
) {
    let left = ctx.margins.left;
    let right = ctx.margins.left + ctx.usable_width;
    let title_size = ctx.font_size + 8.0;
    let sub_size = ctx.small_font_size;

    let title = if ctx.novel_title.is_empty() {
        "未命名".to_string()
    } else {
        ctx.novel_title.clone()
    };
    ctx.draw_text_colored(&title, left, *y, title_size, accent());
    *y -= title_size * 0.3528 * 1.9;

    let sub = if ctx.novel_author.is_empty() {
        "单词卡片 · 语境记忆".to_string()
    } else {
        format!("作者：{}　　单词卡片 · 语境记忆", ctx.novel_author)
    };
    ctx.draw_text_colored(&sub, left, *y, sub_size, text_gray());
    *y -= sub_size * 0.3528 * 2.2;

    // Stat strip: 单词数 N | 生疏 u | 熟悉 f | 掌握 m
    let stats = [
        format!("单词数 {}", total),
        format!("生疏 {}", unknown),
        format!("熟悉 {}", familiar),
        format!("掌握 {}", mastered),
    ];
    let mut x = left;
    for (i, s) in stats.iter().enumerate() {
        let w = ctx.measure_text_width(s, sub_size);
        ctx.draw_text_colored(s, x, *y, sub_size, text_black());
        x += w;
        if i < stats.len() - 1 {
            let sep = "　|　";
            let sw = ctx.measure_text_width(sep, sub_size);
            ctx.draw_text_colored(sep, x, *y, sub_size, text_light_gray());
            x += sw;
        }
    }
    *y -= sub_size * 0.3528 * 3.0;

    let rule_y = *y + sub_size * 0.3528 * 0.8;
    ctx.draw_hline(left, right, rule_y, table_border(), 0.6);
    *y -= sub_size * 0.3528 * 1.0;
}

/// Per-section heading block: section title / word count / rule.
fn draw_chapter_header(
    ctx: &mut PdfContext,
    y: &mut f32,
    section_title: &str,
    word_count: usize,
) {
    let left = ctx.margins.left;
    let right = ctx.margins.left + ctx.usable_width;
    let mid = ctx.font_size + 4.0;
    let small = ctx.small_font_size;

    if !section_title.is_empty() {
        ctx.draw_text_colored(section_title, left, *y, mid, accent());
        *y -= mid * 0.3528 * 1.9;
    }

    let wc = format!("本章词汇：{} 词", word_count);
    ctx.draw_text_colored(&wc, left, *y, small, text_gray());
    *y -= small * 0.3528 * 2.2;

    let rule_y = *y + small * 0.3528 * 0.8;
    ctx.draw_hline(left, right, rule_y, table_border(), 0.5);
    *y -= small * 0.3528 * 0.6;
}

/// Column geometry: left = 55% of usable width, right = remainder, gap between.
fn column_geometry(ctx: &PdfContext) -> (f32, f32, f32, f32) {
    let usable = ctx.usable_width;
    let left_w = usable * 0.55;
    let gap = usable * 0.05;
    let right_w = usable - left_w - gap;
    (
        ctx.margins.left,
        left_w,
        ctx.margins.left + left_w + gap,
        right_w,
    )
}

/// Build the flat left-column segment list for one paragraph, inserting a
/// highlighted target-word segment (with inline gloss) at each match.
fn para_segments(para: &str, words: &[VocabWord]) -> Vec<Seg> {
    let matches = find_matches_in_line_en(para, words);
    if matches.is_empty() {
        return vec![Seg::Text(para.to_string())];
    }
    let mut segs = Vec::new();
    let mut last = 0usize;
    for m in &matches {
        if m.start > last {
            segs.push(Seg::Text(para[last..m.start].to_string()));
        }
        let word_text = para[m.start..m.end].to_string();
        let gloss = short_gloss(&m.word.definition);
        segs.push(Seg::Word {
            text: word_text,
            gloss,
            color: text_color_for_proficiency(&m.word.proficiency),
        });
        last = m.end;
    }
    if last < para.len() {
        segs.push(Seg::Text(para[last..].to_string()));
    }
    segs
}

/// Width (mm) of a segment: word at `font_size`, gloss at `small`, plus a
/// leading/trailing space so the gloss doesn't jam into neighbouring words.
fn seg_width(ctx: &PdfContext, seg: &Seg, font_size: f32, small: f32) -> f32 {
    match seg {
        Seg::Text(s) => ctx.measure_text_width(s, font_size),
        Seg::Word { text, gloss, .. } => {
            let w = ctx.measure_text_width(text, font_size);
            if gloss.is_empty() {
                w
            } else {
                w
                    + ctx.measure_text_width(" ", small)
                    + ctx.measure_text_width(gloss, small)
                    + ctx.measure_text_width(" ", font_size)
            }
        }
    }
}

/// Tokenize a paragraph's segments into word-sized tokens (each word carries
/// its trailing space). This lets the wrapper break only at word boundaries so
/// paragraphs flow cleanly instead of emitting over-wide chunks that get cut.
fn tokenize(segs: &[Seg]) -> Vec<Seg> {
    let mut out = Vec::new();
    for seg in segs {
        match seg {
            Seg::Text(s) => {
                let parts: Vec<&str> = s.split(' ').collect();
                for (i, p) in parts.iter().enumerate() {
                    if p.is_empty() {
                        continue;
                    }
                    let mut tok = p.to_string();
                    if i < parts.len() - 1 {
                        tok.push(' ');
                    }
                    out.push(Seg::Text(tok));
                }
            }
            Seg::Word { .. } => out.push(seg.clone()),
        }
    }
    out
}

/// Wrap segments into physical lines that each fit within `col_w` mm. Breaks at
/// token (word) boundaries; a single over-wide token is emitted on its own line.
fn wrap_segments(ctx: &PdfContext, segs: &[Seg], col_w: f32, font_size: f32, small: f32) -> Vec<Vec<Seg>> {
    let toks = tokenize(segs);
    let mut lines: Vec<Vec<Seg>> = Vec::new();
    let mut cur: Vec<Seg> = Vec::new();
    let mut w = 0.0f32;
    for tok in toks {
        let tw = seg_width(ctx, &tok, font_size, small);
        if tw > col_w {
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
                w = 0.0;
            }
            lines.push(vec![tok]);
            continue;
        }
        if w + tw > col_w && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
            w = 0.0;
        }
        cur.push(tok);
        w += tw;
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Draw one physical line of segments left-to-right within `[x_left, x_right]`.
fn draw_line(
    ctx: &mut PdfContext,
    segs: &[Seg],
    x_left: f32,
    x_right: f32,
    y: f32,
    font_size: f32,
    small: f32,
) {
    let mut x = x_left;
    for seg in segs {
        match seg {
            Seg::Text(s) => {
                for ch in s.chars() {
                    let cw = ctx.measure_text_width(&ch.to_string(), font_size);
                    if x + cw > x_right {
                        break;
                    }
                    ctx.draw_text_colored(&ch.to_string(), x, y, font_size, text_black());
                    x += cw;
                }
            }
            Seg::Word { text, gloss, color } => {
                for ch in text.chars() {
                    let cw = ctx.measure_text_width(&ch.to_string(), font_size);
                    if x + cw > x_right {
                        break;
                    }
                    ctx.draw_text_colored(&ch.to_string(), x, y, font_size, color.clone());
                    x += cw;
                }
                if !gloss.is_empty() {
                    // A little space before the gloss, then it, then a space after
                    // so the following English word isn't jammed against it.
                    x += ctx.measure_text_width(" ", small);
                    let gy = y + small * 0.3528 * 0.4;
                    for ch in gloss.chars() {
                        let cw = ctx.measure_text_width(&ch.to_string(), small);
                        if x + cw > x_right {
                            break;
                        }
                        ctx.draw_text_colored(&ch.to_string(), x, gy, small, text_gray());
                        x += cw;
                    }
                    x += ctx.measure_text_width(" ", font_size);
                }
            }
        }
    }
}

/// A short Chinese meaning for the inline gloss (first sense, POS stripped).
fn short_gloss(def: &str) -> String {
    if def.is_empty() {
        return String::new();
    }
    let line = def.split('\n').next().unwrap_or(def);
    let first = line
        .split(|c: char| matches!(c, '/' | '；' | '，' | '、' | ';' | ','))
        .next()
        .unwrap_or(line)
        .trim();
    // Strip a leading POS prefix like "n." / "adj." / "n.[C]".
    let cleaned = first.split('.').last().unwrap_or(first).trim();
    cleaned.chars().take(12).collect()
}

/// Definition lines for a card: POS/sense lines, wrapped to card width.
fn card_def_lines(def: &str, card_w: f32, small: f32) -> Vec<String> {
    let mut out = Vec::new();
    let max_w = (card_w - CARD_PAD * 2.0).max(4.0);
    for line in def.split('\n') {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("【记忆】")
            || t.starts_with("【搭配】")
            || t.starts_with("【真题】")
            || t.starts_with("【派生】")
            || t.starts_with("【例句】")
        {
            break;
        }
        if t.starts_with("· ") {
            continue;
        }
        for sub in wrap_text_to_lines(t, max_w, small) {
            if !sub.is_empty() {
                out.push(sub);
            }
        }
    }
    if out.is_empty() && !def.trim().is_empty() {
        for sub in wrap_text_to_lines(def.trim(), max_w, small) {
            if !sub.is_empty() {
                out.push(sub);
            }
        }
    }
    out
}

/// Prominent word size inside a card (slightly larger than body text).
fn card_word_size(ctx: &PdfContext) -> f32 {
    ctx.font_size + 1.0
}

/// Estimated height (mm) of one card for the given word.
fn card_height(ctx: &PdfContext, word: &VocabWord, card_w: f32) -> f32 {
    let small = ctx.small_font_size;
    let word_row = card_word_size(ctx) * 0.3528 * 1.5;
    let phon_row = if word.phonetic.is_empty() {
        0.0
    } else {
        small * 0.3528 * 1.6
    };
    let defs = card_def_lines(&word.definition, card_w, small).len() as f32;
    let def_h = if defs > 0.0 {
        defs * small * 0.3528 * 1.6
    } else {
        small * 0.3528 * 1.2
    };
    CARD_PAD * 2.0 + word_row + phon_row + def_h + 1.0
}

/// Draw one pastel word card at the top edge `top_y` (bottom-based mm).
fn draw_card(
    ctx: &mut PdfContext,
    word: &VocabWord,
    x: f32,
    w: f32,
    top_y: f32,
    index: usize,
) {
    let small = ctx.small_font_size;
    let word_size = card_word_size(ctx);
    let word_row = word_size * 0.3528 * 1.5;
    let phon_row = if word.phonetic.is_empty() {
        0.0
    } else {
        small * 0.3528 * 1.6
    };
    let def_row = small * 0.3528 * 1.6;
    let defs = card_def_lines(&word.definition, w, small);
    let def_h = if defs.is_empty() {
        small * 0.3528 * 1.2
    } else {
        defs.len() as f32 * def_row
    };
    let h = CARD_PAD * 2.0 + word_row + phon_row + def_h + 1.0;

    ctx.fill_rounded_rect(x, top_y, w, h, CARD_RADIUS, card_color(index));
    let inner_x = x + CARD_PAD;

    // Word, in a vivid per-card accent color (clearly highlighted).
    let mut y = top_y - CARD_PAD - word_size * 0.3528;
    let mut xc = inner_x;
    let word_col = card_word_color(index);
    for ch in word.word.chars() {
        let cw = ctx.measure_text_width(&ch.to_string(), word_size);
        ctx.draw_text_colored(&ch.to_string(), xc, y, word_size, word_col.clone());
        xc += cw;
    }
    y -= word_row;

    // Phonetic, in brackets.
    if !word.phonetic.is_empty() {
        let ph = format!("[{}]", word.phonetic);
        ctx.draw_text_colored(&ph, inner_x, y, small, text_black());
        y -= phon_row;
    }

    // Definition lines.
    for line in defs {
        ctx.draw_text_colored(&line, inner_x, y, small, text_black());
        y -= def_row;
    }
}

/// Dual-column reading flow for one chapter. Both columns start at `top` on the
/// current page and fill it independently; when either would cross the bottom
/// margin, a new page is started (and `page_top` resets to content top).
fn render_dual_columns(
    ctx: &mut PdfContext,
    paras: &[String],
    chapter_words: &[&VocabWord],
    vocabs: &[VocabWord],
    top: f32,
) {
    let font_size = ctx.font_size;
    let small = ctx.small_font_size;
    let line_h = ctx.line_height;
    let bottom = ctx.margins.bottom;
    let (lx, lw, rx, rw) = column_geometry(ctx);

    // Pre-wrap the whole chapter's left column into physical lines.
    let mut left_lines: Vec<Vec<Seg>> = Vec::new();
    for para in paras {
        let segs = para_segments(para, vocabs);
        let lines = wrap_segments(ctx, &segs, lw, font_size, small);
        left_lines.extend(lines);
        // Blank separator line between paragraphs for readability.
        left_lines.push(Vec::new());
    }

    let mut li = 0usize;
    let mut ci = 0usize;
    let mut page_top = top;

    loop {
        if li >= left_lines.len() && ci >= chapter_words.len() {
            break;
        }
        let avail = page_top - bottom;

        // Cards that fit in the right column.
        let mut c_count = 0usize;
        let mut c_h = 0.0f32;
        while ci + c_count < chapter_words.len() {
            let extra = if c_count > 0 { CARD_GAP } else { 0.0 };
            let hh = card_height(ctx, chapter_words[ci + c_count], rw) + extra;
            if c_h + hh > avail && c_count > 0 {
                break;
            }
            c_h += hh;
            c_count += 1;
        }
        if c_count == 0 && ci < chapter_words.len() {
            c_count = 1;
        }

        // Text lines that fit in the left column.
        let mut l_count = 0usize;
        while li + l_count < left_lines.len() && (l_count + 1) as f32 * line_h <= avail {
            l_count += 1;
        }
        if l_count == 0 && li < left_lines.len() {
            l_count = 1;
        }

        // Draw the right-column cards.
        let mut cy = page_top;
        for k in 0..c_count {
            draw_card(ctx, chapter_words[ci + k], rx, rw, cy, ci + k);
            cy -= card_height(ctx, chapter_words[ci + k], rw) + CARD_GAP;
        }

        // Draw the left-column lines.
        let mut ly = page_top;
        for k in 0..l_count {
            draw_line(ctx, &left_lines[li + k], lx, lx + lw, ly, font_size, small);
            ly -= line_h;
        }

        li += l_count;
        ci += c_count;

        if li < left_lines.len() || ci < chapter_words.len() {
            new_card_page(ctx);
            page_top = ctx.paper_height - ctx.margins.top;
        }
    }
}
