use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfTemplate {
    pub id: i64,
    pub name: String,
    pub paper_size: String,
    pub font_family: String,
    pub font_size: i32,
    pub line_spacing: f64,
    pub margins: String,
    pub annotation_mode: String,     // deprecated, kept for backward compat
    pub template_type: String,       // intensive|sidebar|appendix|recitation|dictation
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}
