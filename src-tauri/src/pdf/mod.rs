mod font;
pub mod matcher;
mod intensive;
mod card;

pub use intensive::{parse_steps_from_db, IntensiveSteps};

use printpdf::*;

/// Measure the width (mm) of a single character at the given font size.
pub(crate) fn measure_char_width(ch: char, font_size: f32) -> f32 {
    if ch == '…' { font_size * 0.3528 } // approx same as a CJK char
    else if ch.is_ascii() { font_size * 0.55 * 0.3528 }
    else { font_size * 0.3528 }
}

/// Text colors for the intensive reading template.
/// `Rgb::new` is not a const fn, so these are exposed as helper functions.
/// NOTE: When changing these, mirror the change in `src/utils/proficiencyColors.ts::PROFICIENCY_TEXT`.
#[inline]
pub fn text_red() -> Color {
    Color::Rgb(Rgb::new(0xCC as f32 / 255.0, 0x00 as f32 / 255.0, 0x00 as f32 / 255.0, None))
}
#[inline]
pub fn text_orange() -> Color {
    Color::Rgb(Rgb::new(0xE6 as f32 / 255.0, 0x7E as f32 / 255.0, 0x22 as f32 / 255.0, None))
}
#[inline]
pub fn text_gray_color() -> Color {
    Color::Rgb(Rgb::new(0x66 as f32 / 255.0, 0x66 as f32 / 255.0, 0x66 as f32 / 255.0, None))
}
#[inline]
pub fn text_purple() -> Color {
    Color::Rgb(Rgb::new(0x99 as f32 / 255.0, 0x00 as f32 / 255.0, 0x99 as f32 / 255.0, None))
}
#[inline]
pub fn text_black() -> Color {
    Color::Greyscale(Greyscale { percent: 0.0, icc_profile: None })
}
#[inline]
pub fn text_gray() -> Color {
    // printpdf Greyscale expects 0.0 (black) ..= 1.0 (white).
    Color::Greyscale(Greyscale { percent: 0.4, icc_profile: None })
}
#[inline]
pub fn text_light_gray() -> Color {
    Color::Greyscale(Greyscale { percent: 0.7, icc_profile: None })
}
#[inline]
pub fn table_border() -> Color {
    Color::Rgb(Rgb::new(0xC8 as f32 / 255.0, 0xD1 as f32 / 255.0, 0xD9 as f32 / 255.0, None))
}
#[inline]
pub fn table_header_bg() -> Color {
    Color::Rgb(Rgb::new(0xE0 as f32 / 255.0, 0xE8 as f32 / 255.0, 0xEF as f32 / 255.0, None))
}

/// Return the text color for a given proficiency level (intensive reading).
/// - unknown → red (needs study)
/// - familiar → orange (familiar but needs reinforcement)
/// - mastered → gray (already mastered, low priority)
#[inline]
pub fn text_color_for_proficiency(proficiency: &str) -> Color {
    match proficiency {
        "familiar" => text_orange(),
        "mastered" => text_gray_color(),
        _ => text_red(), // unknown or any other value → red
    }
}

/// Split `text` into lines that each fit within `max_width` mm.
/// Returns owned strings to avoid borrow conflicts.
pub(crate) fn wrap_text_to_lines(text: &str, max_width: f32, font_size: f32) -> Vec<String> {
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

/// PDF rendering context.
pub struct PdfContext {
    pub doc: PdfDocument,
    pub font_id: FontId,
    pub latin_font_id: FontId,
    /// Parsed CJK font, used to test glyph coverage per character.
    pub cjk_parsed: ParsedFont,
    /// Parsed Latin font, used for accurate English/digit vertical metrics.
    pub latin_parsed: ParsedFont,
    pub font_size: f32,
    pub small_font_size: f32,
    pub line_height: f32,
    pub margins: Margins,
    pub usable_width: f32,
    pub current_y: f32,     // mm from TOP of page
    pub page_count: usize,
    /// Whether to render page header/footer (intensive template only).
    pub show_chrome: bool,
    /// Page number within the current chapter (resets per chapter).
    pub chapter_page: usize,
    /// Novel title for page headers.
    pub novel_title: String,
    /// Novel author for page headers.
    pub novel_author: String,
    /// Pairs of (chapter_title, page_index) recorded during rendering, used to
    /// add PDF bookmarks at the end.
    bookmarks: Vec<(String, usize)>,
    pub paper_width: f32,
    pub paper_height: f32,
    current_ops: Vec<Op>,
}

impl PdfContext {
    pub fn new_page(&mut self) {
        if self.show_chrome {
            self.render_page_chrome();
        }
        let ops = std::mem::take(&mut self.current_ops);
        let page = PdfPage::new(Mm(self.paper_width), Mm(self.paper_height), ops);
        self.doc.pages.push(page);
        self.page_count += 1;
        self.chapter_page += 1;
        self.current_y = self.paper_height - self.margins.top;
    }

    /// Start a fresh page for a new chapter, but avoid emitting a blank page when
    /// the current page has nothing drawn on it yet (e.g. right after another
    /// chapter already forced a page break).
    pub fn new_page_for_chapter(&mut self) {
        if self.current_ops.is_empty() {
            self.current_y = self.paper_height - self.margins.top;
        } else {
            self.new_page();
        }
    }

    /// Reset the per-chapter page counter to 1.
    pub fn reset_chapter_page(&mut self) {
        self.chapter_page = 1;
    }

    /// Draw page header (copyright info) and footer (page number).
    fn render_page_chrome(&mut self) {
        let pn = self.chapter_page;
        let header_y = self.paper_height - self.margins.top + 8.0;
        let copyright = if self.novel_author.is_empty() {
            "已申请知识产权！禁止倒卖".to_string()
        } else {
            format!("作者：{}  已申请知识产权！禁止倒卖", self.novel_author)
        };
        self.draw_text_colored(&copyright, self.margins.left, header_y, self.small_font_size * 0.75, text_red());
        let title_line = if self.novel_title.is_empty() {
            "词学习小说".to_string()
        } else {
            format!("《{}》— 词学习小说", self.novel_title)
        };
        self.draw_text_colored(&title_line, self.margins.left, header_y - 3.5, self.small_font_size * 0.75, text_red());
        let page_str = format!("第 {} 页", pn);
        let pw = self.measure_text_width(&page_str, self.small_font_size);
        let cx = (self.paper_width - pw) / 2.0;
        self.draw_text(&page_str, cx, self.margins.bottom - 5.0, self.small_font_size);
    }

    /// Record the current page as the start of a chapter with the given title,
    /// so a PDF bookmark (outline entry) can be created later.
    pub fn record_bookmark(&mut self, title: &str) {
        let page_idx = self.doc.pages.len();
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

    /// Draw text with a specific fill color, resetting to black afterwards.
    pub fn draw_text_colored(&mut self, text: &str, x_mm: f32, y_mm: f32, size: f32, color: Color) {
        if text.is_empty() {
            return;
        }
        self.current_ops.push(Op::SetFillColor { col: color });
        self.draw_text(text, x_mm, y_mm, size);
        self.current_ops.push(Op::SetFillColor { col: text_black() });
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

    /// Return the PDF baseline that visually centers the font's ascender/
    /// descender box inside a cell. PDF positions text by its baseline, not by
    /// the glyph bounding box, so using a fixed percentage always looks high.
    pub fn centered_text_baseline(
        &self,
        cell_top_y: f32,
        cell_height: f32,
        font_size: f32,
        use_latin_metrics: bool,
    ) -> f32 {
        let metrics = if use_latin_metrics {
            &self.latin_parsed.pdf_font_metrics
        } else {
            &self.cjk_parsed.pdf_font_metrics
        };
        let units_per_em = metrics.units_per_em.max(1) as f32;
        let scale_mm = font_size * 0.3528 / units_per_em;
        let font_visual_mid = (metrics.ascender as f32 + metrics.descender as f32) / 2.0;
        let cell_center_y = cell_top_y - cell_height / 2.0;
        cell_center_y - font_visual_mid * scale_mm
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

    /// Fill a rectangle with a solid color. x, y are distance from left / bottom.
    ///
    /// NOTE: printpdf 0.9's `Op::DrawRectangle` emits `re ... n` and IGNORES
    /// `PaintMode::Fill`, so rectangles are never actually filled (just an empty
    /// path). We draw a 4-corner filled polygon instead, which emits `f`.
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let poly: Polygon = vec![
            (Point::new(Mm(x), Mm(y - h)), false),
            (Point::new(Mm(x + w), Mm(y - h)), false),
            (Point::new(Mm(x + w), Mm(y)), false),
            (Point::new(Mm(x), Mm(y)), false),
        ]
        .into_iter()
        .collect();
        self.current_ops.push(Op::SetFillColor { col: color });
        self.current_ops.push(Op::DrawPolygon { polygon: poly });
        self.current_ops.push(Op::SetFillColor { col: text_black() });
    }

    /// Fill a rounded rectangle. x, y are the top edge (bottom-based mm), w, h
    /// the size, r the corner radius. Uses cubic-bezier corner arcs so printpdf
    /// renders genuinely rounded corners.
    pub fn fill_rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        let k = 0.552284749831f32;
        let mut pts: Vec<(Point, bool)> = Vec::with_capacity(17);
        // start on the left edge, just above the bottom-left corner arc
        pts.push((Point::new(Mm(x), Mm(y - h + r)), false));
        // left edge up
        pts.push((Point::new(Mm(x), Mm(y - r)), false));
        // top-left corner arc
        pts.push((Point::new(Mm(x), Mm(y - r + k * r)), true));
        pts.push((Point::new(Mm(x + r - k * r), Mm(y)), true));
        pts.push((Point::new(Mm(x + r), Mm(y)), false));
        // top edge
        pts.push((Point::new(Mm(x + w - r), Mm(y)), false));
        // top-right corner arc
        pts.push((Point::new(Mm(x + w - r + k * r), Mm(y)), true));
        pts.push((Point::new(Mm(x + w), Mm(y - r + k * r)), true));
        pts.push((Point::new(Mm(x + w), Mm(y - r)), false));
        // right edge down
        pts.push((Point::new(Mm(x + w), Mm(y - h + r)), false));
        // bottom-right corner arc
        pts.push((Point::new(Mm(x + w), Mm(y - h + r - k * r)), true));
        pts.push((Point::new(Mm(x + w - r + k * r), Mm(y - h)), true));
        pts.push((Point::new(Mm(x + w - r), Mm(y - h)), false));
        // bottom edge
        pts.push((Point::new(Mm(x + r), Mm(y - h)), false));
        // bottom-left corner arc
        pts.push((Point::new(Mm(x + r - k * r), Mm(y - h)), true));
        pts.push((Point::new(Mm(x), Mm(y - h + r - k * r)), true));
        pts.push((Point::new(Mm(x), Mm(y - h + r)), false));

        let polygon: Polygon = pts.into_iter().collect();
        self.current_ops.push(Op::SetFillColor { col: color });
        self.current_ops.push(Op::DrawPolygon { polygon });
        self.current_ops.push(Op::SetFillColor { col: text_black() });
    }

    /// Draw a horizontal line from (x1,y) to (x2,y). y = bottom-based mm.
    pub fn draw_hline(&mut self, x1: f32, x2: f32, y: f32, color: Color, thickness_pt: f32) {
        self.current_ops.push(Op::SetOutlineColor { col: color });
        self.current_ops.push(Op::SetOutlineThickness { pt: Pt(thickness_pt) });
        self.current_ops.push(Op::DrawLine {
            line: Line {
                points: vec![
                    LinePoint { p: Point::new(Mm(x1), Mm(y)), bezier: false },
                    LinePoint { p: Point::new(Mm(x2), Mm(y)), bezier: false },
                ],
                is_closed: false,
            },
        });
    }

    /// Draw a vertical line from (x,y1) to (x,y2). y = bottom-based mm.
    pub fn draw_vline(&mut self, x: f32, y1: f32, y2: f32, color: Color, thickness_pt: f32) {
        self.current_ops.push(Op::SetOutlineColor { col: color });
        self.current_ops.push(Op::SetOutlineThickness { pt: Pt(thickness_pt) });
        self.current_ops.push(Op::DrawLine {
            line: Line {
                points: vec![
                    LinePoint { p: Point::new(Mm(x), Mm(y1)), bezier: false },
                    LinePoint { p: Point::new(Mm(x), Mm(y2)), bezier: false },
                ],
                is_closed: false,
            },
        });
    }

    /// Truncate text to fit within `max_width` mm, appending "…" if truncated.
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
            } else if s.len() > 0 {
                s.pop();
                s.push('…');
            }
            s
        } else {
            text.to_string()
        }
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

/// Progress payload emitted to the frontend during PDF generation.
#[derive(Clone, serde::Serialize)]
pub struct PdfProgress {
    pub percent: u32,
    pub message: String,
}

/// Draw a full cover page (shared by the intensive and card templates):
/// centered book title, author, export date and a vocab-stat strip.
pub(crate) fn draw_cover_page(
    ctx: &mut PdfContext,
    total: usize,
    unknown: usize,
    familiar: usize,
    mastered: usize,
) {
    let cx = ctx.paper_width / 2.0;
    let small = ctx.small_font_size;
    let mut y = ctx.paper_height * 0.72;

    // Title.
    let title = if ctx.novel_title.is_empty() {
        "未命名".to_string()
    } else {
        ctx.novel_title.clone()
    };
    let title_size = ctx.font_size + 12.0;
    let tw = ctx.measure_text_width(&title, title_size);
    ctx.draw_text_colored(&title, cx - tw / 2.0, y, title_size, text_black());
    y -= title_size * 0.3528 * 2.4;

    // Author / tagline.
    let author = if ctx.novel_author.is_empty() {
        "词阅 · 外语学习".to_string()
    } else {
        format!("作者：{}", ctx.novel_author)
    };
    let sub_size = ctx.font_size + 1.0;
    let aw = ctx.measure_text_width(&author, sub_size);
    ctx.draw_text_colored(&author, cx - aw / 2.0, y, sub_size, text_gray());
    y -= sub_size * 0.3528 * 2.6;

    // Export date.
    let date = format!("导出日期：{}", crate::utils::date::today_ymd());
    let dw = ctx.measure_text_width(&date, small);
    ctx.draw_text_colored(&date, cx - dw / 2.0, y, small, text_light_gray());
    y -= small * 0.3528 * 3.2;

    // Stat strip (centered as a group).
    let stats = [
        format!("单词数 {}", total),
        format!("生疏 {}", unknown),
        format!("熟悉 {}", familiar),
        format!("掌握 {}", mastered),
    ];
    let gap = small * 0.3528 * 2.4;
    let total_w: f32 = stats
        .iter()
        .map(|s| ctx.measure_text_width(s, small) + gap)
        .sum::<f32>()
        - gap;
    let mut x = cx - total_w / 2.0;
    for (i, s) in stats.iter().enumerate() {
        ctx.draw_text_colored(s, x, y, small, text_black());
        x += ctx.measure_text_width(s, small);
        if i < stats.len() - 1 {
            let sep = "  |  ";
            ctx.draw_text_colored(sep, x, y, small, text_light_gray());
            x += ctx.measure_text_width(sep, small);
        }
    }
}

pub fn generate_pdf(
    novel: &Novel,
    template: &PdfTemplate,
    vocabs: &[VocabWord],
    chapters: &[Chapter],
    steps: IntensiveSteps,
    background: &str,
    cover: bool,
    page_numbers: bool,
    progress: Option<&dyn Fn(PdfProgress)>,
) -> Result<Vec<u8>, String> {
    if let Some(p) = progress {
        p(PdfProgress {
            percent: 0,
            message: "正在加载中文字体…".to_string(),
        });
    }
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

    if let Some(p) = progress {
        p(PdfProgress {
            percent: 2,
            message: "正在创建文档…".to_string(),
        });
    }

    // 2. Create document
    let mut doc = PdfDocument::new(
        if novel.title.is_empty() { "未命名" } else { &novel.title },
    );
    let font_id = doc.add_font(&parsed_font);
    let latin_font_id = match &parsed_latin {
        Some(pf) => doc.add_font(pf),
        None => font_id.clone(),
    };
    let latin_metrics_font = parsed_latin.unwrap_or_else(|| parsed_font.clone());

    let margins = Margins::from_json(&template.margins);
    let (paper_w, paper_h) = paper_dims(&template.paper_size);
    let usable_w = paper_w - margins.left - margins.right;

    let font_size = template.font_size.max(8).min(24) as f32;
    let line_height = template.line_spacing.max(1.0).min(3.0) as f32 * font_size * 0.3528;

    let mut ctx = PdfContext {
        doc,
        font_id,
        latin_font_id,
        cjk_parsed: parsed_font,
        latin_parsed: latin_metrics_font,
        font_size,
        small_font_size: font_size * 0.65,
        line_height,
        margins: margins.clone(),
        usable_width: usable_w,
        current_y: paper_h - margins.top - 5.0,
        page_count: 0,
        paper_width: paper_w,
        paper_height: paper_h,
        current_ops: Vec::new(),
        bookmarks: Vec::new(),
        show_chrome: template.template_type.as_str() != "card",
        chapter_page: 1,
        novel_title: if novel.title.is_empty() { String::new() } else { novel.title.clone() },
        novel_author: if novel.author.is_empty() { String::new() } else { novel.author.clone() },
    };

    // 3. Render — the template type picks which renderer to run.
    match template.template_type.as_str() {
        "card" => card::render(&mut ctx, chapters, vocabs, &novel.language, background, cover, page_numbers, progress),
        _ => intensive::render(&mut ctx, chapters, vocabs, steps, &novel.language, cover, progress),
    }

    // 4. Add PDF bookmarks for chapter navigation
    for (title, page) in &ctx.bookmarks {
        ctx.doc.add_bookmark(title, *page);
    }

    // 5. Finalize last page
    if !ctx.current_ops.is_empty() {
        if ctx.show_chrome {
            ctx.render_page_chrome();
        }
        let ops = std::mem::take(&mut ctx.current_ops);
        ctx.doc.pages.push(PdfPage::new(Mm(paper_w), Mm(paper_h), ops));
    }

    // Soft memory safeguard: warn (not fail) for very long exports. Long PDFs
    // can spike memory due to printpdf buffering all pages until save.
    let page_count = ctx.doc.pages.len();
    if page_count > 500 {
        eprintln!(
            "[pdf] 注意：本次导出共 {} 页，属于较长 PDF，导出过程内存占用较高",
            page_count
        );
        if let Some(p) = progress {
            p(PdfProgress {
                percent: 91,
                message: format!("文档较长（{} 页），正在写入 PDF 文件……", page_count),
            });
        }
    }

    // 6. Save
    if let Some(p) = progress {
        p(PdfProgress {
            percent: 92,
            message: "正在生成并写入 PDF 文件…".to_string(),
        });
    }
    let opts = PdfSaveOptions::default();
    let mut save_warnings = Vec::new();
    let bytes = ctx.doc.save(&opts, &mut save_warnings);
    Ok(bytes)
}
