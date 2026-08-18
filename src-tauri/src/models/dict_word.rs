use serde::Serialize;

#[derive(Serialize, Clone, Default)]
pub struct DictWord {
    pub word: String,
    pub phonetic_uk: String,
    pub phonetic_us: String,
    pub translation: String,
    pub frequency: f64,
    pub difficulty: i64,
}
