use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: i64,
    pub novel_id: i64,
    pub title: String,
    pub content: String,
    pub sort_order: i32,
    pub start_index: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub chapters: Vec<Chapter>,
    pub raw_text: String,
    pub cleaned_text: String,
    pub detected_title: String,
}
