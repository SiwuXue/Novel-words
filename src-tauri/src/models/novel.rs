use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Novel {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub category: String,
    pub raw_text: String,
    pub cleaned_text: String,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub title: String,
    pub content: String,
    pub start_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub chapters: Vec<Chapter>,
    pub raw_text: String,
    pub cleaned_text: String,
    pub detected_title: String,
}
