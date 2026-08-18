use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct PdfExportResponse {
    pub path: String,
    pub total_vocab: usize,
    pub matched_words: usize,
    pub chapter_count: usize,
    pub steps_used: String,
}
