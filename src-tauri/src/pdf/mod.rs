mod font;
mod matcher;
mod intensive;
mod sidebar;
mod recitation;
mod dictation;

use printpdf::*;
use std::fs::File;
use std::io::Write;

/// Measure the width (mm) of a single character at the given font size.
fn measure_char_width(ch: char, font_size: f32) -> f32 {
    if ch == '…' { font_size * 0.3528 } // approx same as a CJK char
    else if ch.is_ascii() { font_size * 0.55 * 0.3528 }
    else { font_size * 0.3528 }
}

/// Split `text` into lines that each fit within `max_width` mm.
/// Returns owned strings to avoid borrow conflicts.
fn wrap_text_to_lines(text: &str, max_width: f32, font_size: f32) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        let (line_end, rest_start) = split_line_at_width(remaining, max_width, font_size);
        if line_end == 0 {
            // Fallback: emit one char at a time
            lines.push(remaining[..remaining.chars().next().map(|c| c.len_utf8()).unwrap_or(1)].to_string());
            remaining = &remaining[remaining.chars().next().map(|c| c.len_utf8()).unwrap_or(1)..];
            continue;
        }
        lines.push(remaining[..line_end].to_string());
        if rest_start >= remaining.len() {
            break;
        }
        remaining = &remaining[rest_start..];
    }
    lines
}

/// Find the split point for one line of `text` that fits within `max_width` mm.
/// Returns (line_end_byte_index, rest_start_byte_index).
fn split_line_at_width(text: &str, max_width: f32, font_size: f32) -> (usize, usize) {
    let mut last_break = 0;
    let mut last_width_ok = 0;
    let mut cum_width = 0.0f32;

    for (i, ch) in text.char_indices() {
        let ch_w = measure_char_width(ch, font_size);
        cum_width += ch_w;
        if cum_width <= max_width {
            last_width_ok = i + ch.len_utf8();
            // CJK chars can break anywhere; spaces are natural break points
            if !ch.is_ascii() || ch == ' ' {
                last_break = i + ch.len_utf8();
            }
        } else {
            let break_at = if last_break > 0 { last_break } else { last_width_ok };
            let break_at = if break_at == 0 { text.len() } else { break_at };
            // Trim trailing space from line
            let line_end = if break_at > 0 && text[..break_at].ends_with(' ') {
                break_at - 1
            } else {
                break_at
            };
            // Skip leading space on remainder
            let rest_start = if break_at < text.len() && text[break_at..].starts_with(' ') {
                break_at + 1
            } else {
                break_at
            };
            return (line_end, rest_start);
        }
    }
    // Entire text fits on one line
    (text.len(), text.len())
}

use crate::models::novel::{Chapter, Novel};
use crate::models::pdf_template::PdfTemplate;
use crate::models::vocab_word::VocabWord;

/// Margins in millimeters.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Margins {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl Margins {
    fn from_json(raw: &str) -> Self {
        serde_json::from_str(raw).unwrap_or(Margins {
            top: 25.0,
            bottom: 25.0,
            left: 20.0,
            right: 20.0,
        })
    }
}

fn make_point(x_mm: f32, y_mm: f32) -> Point {
    Point::new(Mm(x_mm), Mm(y_mm))
}

fn make_line_point(x_mm: f32, y_mm: f32) -> LinePoint {
    LinePoint {
        p: make_point(x_mm, y_mm),
        bezier: false,
    }
}

/// PDF rendering context.
pub struct PdfContext {
    pub doc: PdfDocument,
    pub font_id: FontId,
    pub latin_font_id: FontId,
    /// Parsed CJK font, used to test glyph coverage per character.
    pub cjk_parsed: ParsedFont,
    pub font_size: f32,
    pub small_font_size: f32,
    pub line_height: f32,
    pub margins: Margins,
    pub usable_width: f32,
    pub usable_height: f32,
    pub current_y: f32,     // mm from TOP of page
    pub page_count: usize,
    /// Pairs of (chapter_title, page_index) recorded during rendering, used to
    /// add PDF bookmarks at the end.
    bookmarks: Vec<(String, usize)>,
    pub paper_width: f32,
    pub paper_height: f32,
    current_ops: Vec<Op>,
}

impl PdfContext {
    pub fn new_page(&mut self) {
        let ops = std::mem::take(&mut self.current_ops);
        let page = PdfPage::new(Mm(self.paper_width), Mm(self.paper_height), ops);
        self.doc.pages.push(page);
        self.page_count += 1;
        self.current_y = self.paper_height - self.margins.top;
    }

    /// Start a fresh page for a new chapter, but avoid emitting a blank page when
    /// the current page has nothing drawn on it yet (e.g. right after another
    /// chapter already forced a page break).
    pub fn new_page_for_chapter(&mut self) {
        if self.current_ops.is_empty() {
            // Current page is empty — just reset the cursor to the top, don't push
            // an empty page.
            self.current_y = self.paper_height - self.margins.top;
        } else {
            self.new_page();
        }
    }

    /// Record the current page as the start of a chapter with the given title,
    /// so a PDF bookmark (outline entry) can be created later.
    pub fn record_bookmark(&mut self, title: &str) {
        let page_idx = self.doc.pages.len().saturating_sub(1);
        self.bookmarks.push((title.to_string(), page_idx));
    }

    pub fn remaining_height(&self) -> f32 {
        self.current_y - self.margins.bottom
    }

    /// Draw text at (x_mm, y_mm) where y is the distance from the BOTTOM of the
    /// page (matching `current_y`; larger y = higher up). printpdf uses a
    /// bottom-left origin, so this maps straight through without flipping.
    /// Splits the text into runs by font: characters the CJK font can render use
    /// the CJK font; the rest (Latin letters, IPA phonetic symbols) use the Latin
    /// font, so symbols like ˈ ə ʌ ð ʃ don't turn into tofu boxes.
    pub fn draw_text(&mut self, text: &str, x_mm: f32, y_mm: f32, size: f32) {
        if text.is_empty() {
            return;
        }
        let bottom_y = y_mm;
        let mut cursor_x = x_mm;

        // Group consecutive chars that share the same font into runs.
        let mut run = String::new();
        let mut run_is_latin = false;
        let mut run_started = false;

        for ch in text.chars() {
            let use_latin = self.prefer_latin(ch);
            if run_started && use_latin != run_is_latin {
                self.emit_run(&run, cursor_x, bottom_y, size, run_is_latin);
                cursor_x += self.run_width(&run, size, run_is_latin);
                run.clear();
            }
            run.push(ch);
            run_is_latin = use_latin;
            run_started = true;
        }
        if !run.is_empty() {
            self.emit_run(&run, cursor_x, bottom_y, size, run_is_latin);
        }
    }

    /// Should this char be drawn with the Latin font rather than the CJK font?
    /// Non-CJK codepoints (ASCII, IPA, Latin punctuation) go to the Latin font.
    fn prefer_latin(&self, ch: char) -> bool {
        // CJK ideographs & common CJK punctuation always use the CJK font.
        let c = ch as u32;
        let is_cjk = matches!(c,
            0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF | 0x3000..=0x303F | 0xFF00..=0xFFEF
        );
        if is_cjk {
            return false;
        }
        // For everything else, prefer Latin if the CJK font lacks the glyph.
        self.cjk_parsed.lookup_glyph_index(c).is_none()
    }

    fn emit_run(&mut self, run: &str, x_mm: f32, bottom_y: f32, size: f32, is_latin: bool) {
        let font = if is_latin { self.latin_font_id.clone() } else { self.font_id.clone() };
        self.current_ops.push(Op::StartTextSection);
        self.current_ops.push(Op::SetFont {
            font: PdfFontHandle::External(font),
            size: Pt(size),
        });
        self.current_ops.push(Op::SetTextCursor {
            pos: make_point(x_mm, bottom_y),
        });
        self.current_ops.push(Op::ShowText {
            items: vec![TextItem::Text(run.to_string())],
        });
        self.current_ops.push(Op::EndTextSection);
    }

    fn run_width(&self, run: &str, size: f32, is_latin: bool) -> f32 {
        let mut w = 0.0f32;
        for ch in run.chars() {
            w += if is_latin || ch.is_ascii() { size * 0.55 } else { size };
        }
        w * 0.3528
    }

    pub fn measure_text_width(&self, text: &str, font_size: f32) -> f32 {
        let mut w = 0.0f32;
        for ch in text.chars() {
            w += if ch.is_ascii() { font_size * 0.55 } else { font_size };
        }
        w * 0.3528
    }

    /// Wrap text within `max_width` mm, drawing each line at `x_mm` from the left,
    /// advancing `current_y` downward by `line_height` per line. Automatically
    /// starts a new page when the text runs past the bottom margin, so long
    /// paragraphs (or whole chapters with no blank-line breaks) don't overflow
    /// off the page.
    /// Returns the number of lines drawn.
    pub fn draw_text_wrapped(
        &mut self,
        text: &str,
        x_mm: f32,
        max_width: f32,
        font_size: f32,
    ) -> usize {
        if text.is_empty() {
            return 0;
        }
        // Split into owned strings first to avoid borrow conflicts with draw_text
        let lines = wrap_text_to_lines(text, max_width, font_size);
        let count = lines.len();
        for line in &lines {
            // Page break when the next line would cross the bottom margin.
            if self.current_y - self.line_height < self.margins.bottom {
                self.new_page();
            }
            self.draw_text(line, x_mm, self.current_y, font_size);
            self.current_y -= self.line_height;
        }
        count
    }

    /// Truncate text to fit within `max_width` mm, appending "…" if truncated.
    /// Returns an owned String.
    pub fn truncate_text(&self, text: &str, max_width: f32, font_size: f32) -> String {
        let mut cum_w = 0.0f32;
        let mut end_idx = 0;
        for (i, ch) in text.char_indices() {
            let ch_w = measure_char_width(ch, font_size);
            if cum_w + ch_w > max_width {
                break;
            }
            cum_w += ch_w;
            end_idx = i + ch.len_utf8();
        }
        if end_idx < text.len() {
            let mut s = text[..end_idx].to_string();
            let ellipsis_w = measure_char_width('…', font_size);
            if cum_w + ellipsis_w <= max_width {
                s.push('…');
            }
            s
        } else {
            text.to_string()
        }
    }

    /// Draw rectangle border. `y` is distance from the bottom of the page (top
    /// edge of the row); the rectangle extends downward by `h`.
    pub fn draw_rect_border(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let bottom = y;
        let black = Color::Greyscale(Greyscale { percent: 0.0, icc_profile: None });
        self.current_ops.push(Op::SetOutlineColor { col: black.clone() });
        self.current_ops.push(Op::SetOutlineThickness { pt: Pt(0.5) });
        let mm_to_pt = 2.8346;
        self.current_ops.push(Op::DrawRectangle {
            rectangle: Rect {
                x: Pt(x * mm_to_pt),
                y: Pt((bottom - h) * mm_to_pt),
                width: Pt(w * mm_to_pt),
                height: Pt(h * mm_to_pt),
                mode: None,
                winding_order: None,
            },
        });
    }

    /// Draw line. `y1` and `y2` are distances from the bottom of the page.
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let by1 = y1;
        let by2 = y2;
        let black = Color::Greyscale(Greyscale { percent: 0.0, icc_profile: None });
        self.current_ops.push(Op::SetOutlineColor { col: black });
        self.current_ops.push(Op::SetOutlineThickness { pt: Pt(0.5) });
        self.current_ops.push(Op::DrawLine {
            line: Line {
                points: vec![
                    make_line_point(x1, by1),
                    make_line_point(x2, by2),
                ],
                is_closed: false,
            },
        });
    }
}

fn paper_dims(size: &str) -> (f32, f32) {
    match size {
        "A5" => (148.0, 210.0),
        _ => (210.0, 297.0),
    }
}

/// Split a chapter's body into paragraphs. Novels vary: some separate paragraphs
/// with blank lines ("\n\n"), others with a single "\n" per paragraph. If there
/// are no blank-line breaks, fall back to splitting on every newline so each
/// line becomes its own paragraph (instead of the whole chapter collapsing into
/// one giant block).
pub fn split_paragraphs(content: &str) -> Vec<String> {
    let has_blank_line = content.contains("\n\n") || content.contains("\r\n\r\n");
    let parts: Vec<String> = if has_blank_line {
        content
            .split("\n\n")
            .map(|p| p.replace('\r', "").replace('\n', " "))
            .collect()
    } else {
        content.lines().map(|l| l.to_string()).collect()
    };
    parts
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

pub fn generate_pdf(
    novel: &Novel,
    template: &PdfTemplate,
    vocabs: &[VocabWord],
    chapters: &[Chapter],
    output_path: &str,
) -> Result<(), String> {
    // 1. Find + load font
    let font_path = font::find_chinese_font()
        .ok_or_else(|| "未找到系统中文字体".to_string())?;
    let font_bytes = std::fs::read(&font_path)
        .map_err(|e| format!("读取字体文件失败: {}", e))?;
    let mut warnings = Vec::new();
    let parsed_font = ParsedFont::from_bytes(&font_bytes, 0, &mut warnings)
        .ok_or_else(|| format!("解析字体失败: {}", font_path))?;

    // Latin/IPA font (for English words + phonetic symbols). Fall back to the CJK
    // font if none found, so rendering still works (just with tofu for IPA).
    let latin_path = font::find_latin_font();
    let parsed_latin = latin_path
        .as_ref()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| {
            let mut w = Vec::new();
            ParsedFont::from_bytes(&b, 0, &mut w)
        });

    // 2. Create document
    let mut doc = PdfDocument::new(
        if novel.title.is_empty() { "未命名" } else { &novel.title },
    );
    let font_id = doc.add_font(&parsed_font);
    let latin_font_id = match &parsed_latin {
        Some(pf) => doc.add_font(pf),
        None => font_id.clone(),
    };

    let margins = Margins::from_json(&template.margins);
    let (paper_w, paper_h) = paper_dims(&template.paper_size);
    let usable_w = paper_w - margins.left - margins.right;
    let usable_h = paper_h - margins.top - margins.bottom;

    let font_size = template.font_size.max(8).min(24) as f32;
    let line_height = template.line_spacing.max(1.0).min(3.0) as f32 * font_size * 0.3528;

    let mut ctx = PdfContext {
        doc,
        font_id,
        latin_font_id,
        cjk_parsed: parsed_font,
        font_size,
        small_font_size: font_size * 0.65,
        line_height,
        margins: margins.clone(),
        usable_width: usable_w,
        usable_height: usable_h,
        current_y: paper_h - margins.top - 60.0,
        page_count: 0,
        paper_width: paper_w,
        paper_height: paper_h,
        current_ops: Vec::new(),
        bookmarks: Vec::new(),
    };

    // 3. Render title (y is now bottom-based, matching current_y)
    let title_y = paper_h - margins.top - 5.0;
    let author_y = paper_h - margins.top - 25.0;
    let title_str = if novel.title.is_empty() { "未命名" } else { &novel.title };
    ctx.draw_text(title_str, margins.left, title_y, font_size + 4.0);
    if !novel.author.is_empty() {
        ctx.draw_text(&novel.author, margins.left, author_y, font_size);
    }

    ctx.new_page();

    // 4. Dispatch
    match template.template_type.as_str() {
        "intensive" => intensive::render(&mut ctx, chapters, vocabs),
        "sidebar" => sidebar::render(&mut ctx, chapters, vocabs),
        "recitation" => recitation::render(&mut ctx, chapters, vocabs),
        "dictation" => dictation::render(&mut ctx, chapters, vocabs),
        _ => intensive::render(&mut ctx, chapters, vocabs),
    };

    // 5. Add PDF bookmarks for chapter navigation
    for (title, page) in &ctx.bookmarks {
        ctx.doc.add_bookmark(title, *page);
    }

    // 6. Finalize last page
    if !ctx.current_ops.is_empty() {
        let ops = std::mem::take(&mut ctx.current_ops);
        ctx.doc.pages.push(PdfPage::new(Mm(paper_w), Mm(paper_h), ops));
    }

    // 6. Save
    let opts = PdfSaveOptions::default();
    let mut save_warnings = Vec::new();
    let bytes = ctx.doc.save(&opts, &mut save_warnings);
    let mut file = File::create(output_path)
        .map_err(|e| format!("创建文件失败: {}", e))?;
    file.write_all(&bytes)
        .map_err(|e| format!("写入 PDF 失败: {}", e))?;

    Ok(())
}
