use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VocabBook {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}
