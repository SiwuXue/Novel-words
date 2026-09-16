//! Existing SM-2 scheduling and the compatible memory-tag wire envelope.
//! Personal schedules live in `user_vocab`; each membership keeps its local tag.

use serde::{Deserialize, Serialize};

pub const RATING_AGAIN: &str = "again";
pub const RATING_EASY: &str = "easy";

const MIN_EASE: f64 = 1.3;
const MAX_EASE: f64 = 3.5;
const DEFAULT_EASE: f64 = 2.5;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for SrsState {
    fn default() -> Self {
        Self {
            ease: DEFAULT_EASE,
            interval: 0,
            reps: 0,
            lapses: 0,
            due: String::new(),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct MemoryTagEnvelope {
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    srs: Option<SrsState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_reviewed_at: Option<u64>,
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

/// Serialize with an explicit `last_reviewed_at` timestamp (Unix seconds).
/// Used by the review command to record when the card was last studied, so
/// the frontend can compute "reviewed today" progress.
pub fn serialize_memory_tag_reviewed(tag: &str, srs: &SrsState, last_reviewed_at: u64) -> String {
    if srs.reps == 0 && srs.interval == 0 && srs.due.is_empty() && last_reviewed_at == 0 {
        return tag.to_string();
    }
    serde_json::to_string(&MemoryTagEnvelope {
        tag: Some(tag.to_string()),
        srs: Some(srs.clone()),
        last_reviewed_at: Some(last_reviewed_at),
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
                ((state.interval as f64) * state.ease * 1.3)
                    .round()
                    .max(1.0) as u32
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

#[cfg(test)]
mod tests {
    #[test]
    fn unscheduled_cards_use_the_algorithm_default_ease() {
        let (_, mut state) = super::parse_memory_tag("custom");
        assert_eq!(state.ease, 2.5);
        super::apply_rating(&mut state, "easy");
        assert!((state.ease - 2.65).abs() < 0.0001);
        assert_eq!(state.interval, 4);
    }
}
