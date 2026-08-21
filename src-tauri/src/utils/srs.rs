//! Spaced-repetition (SM-2) scheduling stored inside the existing
//! `vocab_word.memory_tag` column, so no schema change is required.
//!
//! `memory_tag` doubles as the SRS state container: when a card has scheduling
//! data it is serialized as `{"tag":"<user tag>","srs":{...}}`; legacy plain-text
//! tags are preserved and treated as cards with no SRS state yet.

use serde::{Deserialize, Serialize};

pub const RATING_AGAIN: &str = "again";
pub const RATING_EASY: &str = "easy";

const MIN_EASE: f64 = 1.3;
const MAX_EASE: f64 = 3.5;
const DEFAULT_EASE: f64 = 2.5;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SrsState {
    #[serde(default = "default_ease")]
    pub ease: f64,
    #[serde(default)]
    pub interval: u32, // days
    #[serde(default)]
    pub reps: u32,
    #[serde(default)]
    pub lapses: u32,
    #[serde(default)]
    pub due: String, // YYYY-MM-DD (local)
}

fn default_ease() -> f64 {
    DEFAULT_EASE
}

#[derive(Deserialize, Serialize)]
struct MemoryTagEnvelope {
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    srs: Option<SrsState>,
}

/// Parse the `memory_tag` column into `(user_tag, srs_state)`.
/// Legacy plain-text tags (or empty) have no SRS state and are treated as new cards.
pub fn parse_memory_tag(raw: &str) -> (String, SrsState) {
    if raw.is_empty() {
        return (String::new(), SrsState::default());
    }
    match serde_json::from_str::<MemoryTagEnvelope>(raw) {
        Ok(env) => {
            let tag = env.tag.unwrap_or_default();
            let srs = env.srs.unwrap_or_default();
            (tag, srs)
        }
        Err(_) => (raw.to_string(), SrsState::default()),
    }
}

/// Serialize `(tag, srs)` back into the `memory_tag` column.
/// Cards with no meaningful SRS state keep the plain tag for backward compat.
pub fn serialize_memory_tag(tag: &str, srs: &SrsState) -> String {
    if srs.reps == 0 && srs.interval == 0 && srs.due.is_empty() {
        return tag.to_string();
    }
    serde_json::to_string(&MemoryTagEnvelope {
        tag: Some(tag.to_string()),
        srs: Some(srs.clone()),
    })
    .unwrap_or_else(|_| tag.to_string())
}

/// A card is due when it has never been scheduled, or its due date is today or earlier.
pub fn is_due(srs: &SrsState) -> bool {
    if srs.due.is_empty() {
        return true;
    }
    let today = chrono::Local::now().date_naive().to_string();
    srs.due <= today
}

/// Apply an SM-2 rating. Mutates the state and returns the new proficiency
/// (`unknown` / `familiar` / `mastered`), which doubles as the maturity signal.
pub fn apply_rating(state: &mut SrsState, rating: &str) -> String {
    let today = chrono::Local::now().date_naive();

    match rating {
        RATING_AGAIN => {
            state.lapses += 1;
            state.reps = 0;
            state.interval = 1;
            state.ease = (state.ease - 0.2).max(MIN_EASE);
            state.due = (today + chrono::Duration::days(1)).to_string();
            "unknown".to_string()
        }
        RATING_EASY => {
            state.reps += 1;
            state.interval = if state.reps == 1 {
                4
            } else {
                ((state.interval as f64) * state.ease * 1.3).round().max(1.0) as u32
            };
            state.ease = (state.ease + 0.15).min(MAX_EASE);
            state.due = (today + chrono::Duration::days(state.interval as i64)).to_string();
            "mastered".to_string()
        }
        _ => {
            // RATING_GOOD (default)
            state.reps += 1;
            state.interval = match state.reps {
                1 => 1,
                2 => 6,
                _ => ((state.interval as f64) * state.ease).round().max(1.0) as u32,
            };
            state.due = (today + chrono::Duration::days(state.interval as i64)).to_string();
            if state.interval >= 30 {
                "mastered".to_string()
            } else {
                "familiar".to_string()
            }
        }
    }
}
