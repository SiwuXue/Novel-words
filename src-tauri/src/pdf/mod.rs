mod font;
mod intensive;
mod sidebar;
mod appendix;
mod recitation;
mod dictation;

use printpdf::*;
use std::fs::File;
use std::io::Write;

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

/// Helper: convert top-left Y coordinate (mm from top) to bottom-left (mm from bottom).
/// printpdf uses bottom-left origin.
fn bl_y(y_mm: f32, paper_h: f32) -> f32 {
    paper_h - y_mm
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
    pub font_size: f32,
    pub small_font_size: f32,
    pub line_height: f32,
    pub margins: Margins,
    pub usable_width: f32,
    pub usable_height: f32,
    pub current_y: f32,     // mm from TOP of page
    pub page_count: usize,
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

    pub fn remaining_height(&self) -> f32 {
        self.current_y - self.margins.bottom
    }

    /// Draw text at top-left (x_mm, y_mm) where y is distance from top.
    pub fn draw_text(&mut self, text: &str, x_mm: f32, y_mm: f32, size: f32) {
        let bottom_y = bl_y(y_mm, self.paper_height);
        self.current_ops.push(Op::StartTextSection);
        self.current_ops.push(Op::SetFont {
            font: PdfFontHandle::External(self.font_id.clone()),
            size: Pt(size),
        });
        self.current_ops.push(Op::SetTextCursor {
            pos: make_point(x_mm, bottom_y),
        });
        self.current_ops.push(Op::ShowText {
            items: vec![TextItem::Text(text.to_string())],
        });
        self.current_ops.push(Op::EndTextSection);
    }

    pub fn measure_text_width(&self, text: &str, font_size: f32) -> f32 {
        let mut w = 0.0f32;
        for ch in text.chars() {
            w += if ch.is_ascii() { font_size * 0.55 } else { font_size };
        }
        w * 0.3528
    }

    /// Draw rectangle border at top-left coordinates.
    pub fn draw_rect_border(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let bottom = bl_y(y, self.paper_height);
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

    /// Draw line at top-left coordinates.
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let by1 = bl_y(y1, self.paper_height);
        let by2 = bl_y(y2, self.paper_height);
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

    // 2. Create document
    let mut doc = PdfDocument::new(
        if novel.title.is_empty() { "未命名" } else { &novel.title },
    );
    let font_id = doc.add_font(&parsed_font);

    let margins = Margins::from_json(&template.margins);
    let (paper_w, paper_h) = paper_dims(&template.paper_size);
    let usable_w = paper_w - margins.left - margins.right;
    let usable_h = paper_h - margins.top - margins.bottom;

    let font_size = template.font_size.max(8).min(24) as f32;
    let line_height = template.line_spacing.max(1.0).min(3.0) as f32 * font_size * 0.3528;

    let mut ctx = PdfContext {
        doc,
        font_id,
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
    };

    // 3. Render title
    let title_str = if novel.title.is_empty() { "未命名" } else { &novel.title };
    ctx.draw_text(title_str, margins.left, margins.top + 5.0, font_size + 4.0);
    if !novel.author.is_empty() {
        ctx.draw_text(&novel.author, margins.left, margins.top + 25.0, font_size);
    }

    ctx.new_page();

    // 4. Dispatch
    match template.template_type.as_str() {
        "intensive" => intensive::render(&mut ctx, chapters, vocabs),
        "sidebar" => sidebar::render(&mut ctx, chapters, vocabs),
        "appendix" => appendix::render(&mut ctx, chapters, vocabs),
        "recitation" => recitation::render(&mut ctx, chapters, vocabs),
        "dictation" => dictation::render(&mut ctx, chapters, vocabs),
        _ => appendix::render(&mut ctx, chapters, vocabs),
    };

    // 5. Finalize last page
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
