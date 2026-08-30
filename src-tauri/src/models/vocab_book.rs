use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VocabBook {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub is_preset: bool,
    pub preset_key: String,
    pub cloned_from_preset_key: String,
    pub created_at: String,
    pub updated_at: String,
}
