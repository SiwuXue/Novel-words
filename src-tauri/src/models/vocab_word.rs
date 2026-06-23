use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabWord {
    pub id: i64,
    pub vocab_book_id: i64,
    pub word: String,
    pub definition: String,
    pub phonetic: String,
    pub example_sentence: String,
    pub novel_id: Option<i64>,
    pub proficiency: String,
    pub memory_tag: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightWord {
    pub word: String,
    pub definition: String,
    pub phonetic: String,
    pub example_sentence: String,
    pub proficiency: String,
}
