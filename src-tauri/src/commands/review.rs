use crate::commands::vocab_word::row_to_vocab_word;
use crate::db::DbState;
use crate::models::VocabWord;
use crate::utils::srs::{
    apply_rating, is_due, parse_last_reviewed_at, parse_memory_tag, serialize_memory_tag_reviewed,
};
use serde::Serialize;
use tauri::State;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Start of "today" in the local time zone, as Unix seconds.
fn local_today_start_secs() -> u64 {
    use chrono::{Local, TimeZone};
    let secs_per_day: u64 = 86_400;
    let now = now_secs();
    // Approximate "seconds since today's local midnight" by walking back at
    // most a couple of 1-day steps until the local date matches today.
    let mut candidate = now;
    for _ in 0..2 {
        let secs_into_day = candidate % secs_per_day;
        let candidate_local_date = Local
            .timestamp_opt(candidate as i64, 0)
            .single()
            .map(|dt| dt.date_naive());
        let today_local_date = Local::now().date_naive();
        if candidate_local_date == Some(today_local_date) {
            return candidate.saturating_sub(secs_into_day);
        }
        candidate = candidate.saturating_sub(secs_into_day + 1);
    }
    now.saturating_sub(now % secs_per_day)
}

/// Return all words in a book that are due for review today.
/// A card without any SRS state is considered new and therefore due.
#[tauri::command]
pub fn get_due_words(state: State<DbState>, vocab_book_id: i64) -> Result<Vec<VocabWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at, match_terms \
             FROM vocab_word WHERE vocab_book_id=?1 ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let all: Vec<VocabWord> = stmt
        .query_map(rusqlite::params![vocab_book_id], row_to_vocab_word)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let due: Vec<VocabWord> = all
        .into_iter()
        .filter(|w| {
            let (_, srs) = parse_memory_tag(&w.memory_tag);
            is_due(&srs)
        })
        .collect();

    Ok(due)
}

/// Total number of words due for review today across ALL user vocab books
/// (preset/bundled books are excluded — they carry no personal SRS state).
#[tauri::command]
pub fn get_due_words_count(state: State<DbState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT w.memory_tag FROM vocab_word w \
             JOIN vocab_book b ON b.id = w.vocab_book_id \
             WHERE b.is_preset = 0",
        )
        .map_err(|e| e.to_string())?;
    let tags: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    let count = tags
        .iter()
        .filter(|t| {
            let (_, srs) = parse_memory_tag(t);
            is_due(&srs)
        })
        .count() as i64;
    Ok(count)
}

/// Record a review rating for one card. Applies SM-2 and persists the new
/// proficiency + SRS state (inside the existing `memory_tag` column).
#[tauri::command]
pub fn review_vocab_word(
    state: State<DbState>,
    id: i64,
    rating: String,
) -> Result<VocabWord, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let word = db
        .query_row(
            "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at, match_terms \
             FROM vocab_word WHERE id=?1",
            rusqlite::params![id],
            row_to_vocab_word,
        )
        .map_err(|e| format!("未找到该单词: {}", e))?;

    let (tag, mut srs) = parse_memory_tag(&word.memory_tag);
    let proficiency = apply_rating(&mut srs, &rating);
    let new_tag = serialize_memory_tag_reviewed(&tag, &srs, now_secs());

    db.execute(
        "UPDATE vocab_word SET proficiency=?1, memory_tag=?2 WHERE id=?3",
        rusqlite::params![proficiency, new_tag, id],
    )
    .map_err(|e| format!("更新单词失败: {}", e))?;

    let updated = db
        .query_row(
            "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at, match_terms \
             FROM vocab_word WHERE id=?1",
            rusqlite::params![id],
            row_to_vocab_word,
        )
        .map_err(|e| format!("未找到该单词: {}", e))?;

    Ok(updated)
}

#[derive(Debug, Clone, Serialize)]
pub struct ReviewProgress {
    /// Vocabulary book id (or None for "all books").
    pub vocab_book_id: Option<i64>,
    /// Words due for review today (or earlier).
    pub due_total: i64,
    /// Words reviewed since the start of today (local).
    pub reviewed_today: i64,
    /// Daily review goal (read from app_settings; defaults to 20).
    pub goal: i64,
}

/// Review progress for a single book (or all books). Powers the
/// "今日复习 X/Y" header on the Review page.
#[tauri::command]
pub fn get_review_progress(
    state: State<DbState>,
    vocab_book_id: Option<i64>,
) -> Result<ReviewProgress, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Load words in a scoped block so the statement/rows borrow is released
    // before we query the daily goal setting.
    let words: Vec<VocabWord> = {
        let load = |book_id: Option<i64>| -> Result<Vec<VocabWord>, String> {
            match book_id {
                Some(id) => {
                    let mut stmt = db
                        .prepare(
                            "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at, match_terms \
                             FROM vocab_word WHERE vocab_book_id=?1",
                        )
                        .map_err(|e| e.to_string())?;
                    let rows = stmt
                        .query_map(rusqlite::params![id], row_to_vocab_word)
                        .map_err(|e| e.to_string())?;
                    Ok(rows.filter_map(|r| r.ok()).collect())
                }
                None => {
                    // All user books (exclude presets — no personal SRS state).
                    let mut stmt = db
                        .prepare(
                            "SELECT w.id, w.vocab_book_id, w.word, w.definition, w.phonetic, w.example_sentence, w.novel_id, w.proficiency, w.memory_tag, w.created_at, w.match_terms \
                             FROM vocab_word w JOIN vocab_book b ON b.id = w.vocab_book_id \
                             WHERE b.is_preset = 0",
                        )
                        .map_err(|e| e.to_string())?;
                    let rows = stmt
                        .query_map([], row_to_vocab_word)
                        .map_err(|e| e.to_string())?;
                    Ok(rows.filter_map(|r| r.ok()).collect())
                }
            }
        };
        load(vocab_book_id)?
    };

    let today_start = local_today_start_secs();
    let mut due_total: i64 = 0;
    let mut reviewed_today: i64 = 0;
    for w in &words {
        let (_, srs) = parse_memory_tag(&w.memory_tag);
        if is_due(&srs) {
            due_total += 1;
        }
        if let Some(ts) = parse_last_reviewed_at(&w.memory_tag) {
            if ts >= today_start {
                reviewed_today += 1;
            }
        }
    }

    let goal: i64 = db
        .query_row(
            "SELECT value FROM app_settings WHERE key='review_daily_goal'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(20);

    Ok(ReviewProgress {
        vocab_book_id,
        due_total,
        reviewed_today,
        goal,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyReviewCount {
    pub date: String,   // YYYY-MM-DD (local)
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LearningStats {
    pub total_words: i64,
    pub total_books: i64,
    pub total_novels: i64,
    pub by_proficiency: std::collections::BTreeMap<String, i64>,
    pub reviewed_today: i64,
    pub total_reviews: i64,
    pub reviews_last_7_days: Vec<DailyReviewCount>,
}

/// Aggregate learning statistics for the dashboard / stats page.
#[tauri::command]
pub fn get_learning_stats(state: State<DbState>) -> Result<LearningStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let total_words: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM vocab_word w JOIN vocab_book b ON b.id = w.vocab_book_id WHERE b.is_preset = 0",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let total_books: i64 = db
        .query_row("SELECT COUNT(*) FROM vocab_book WHERE is_preset = 0", [], |r| r.get(0))
        .unwrap_or(0);
    let total_novels: i64 = db
        .query_row("SELECT COUNT(*) FROM novel", [], |r| r.get(0))
        .unwrap_or(0);

    let mut by_proficiency: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for prof in ["unknown", "familiar", "mastered"] {
        let n: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM vocab_word w JOIN vocab_book b ON b.id = w.vocab_book_id \
                 WHERE b.is_preset = 0 AND w.proficiency=?1",
                rusqlite::params![prof],
                |r| r.get(0),
            )
            .unwrap_or(0);
        by_proficiency.insert(prof.to_string(), n);
    }

    let today_start = local_today_start_secs();
    let secs_per_day: u64 = 86_400;
    let seven_days_ago = today_start.saturating_sub(secs_per_day * 6); // include today = 7 days

    let mut stmt = db
        .prepare(
            "SELECT w.memory_tag FROM vocab_word w JOIN vocab_book b ON b.id = w.vocab_book_id WHERE b.is_preset = 0",
        )
        .map_err(|e| e.to_string())?;
    let tags: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut reviewed_today: i64 = 0;
    let mut total_reviews: i64 = 0;
    let mut daily_buckets: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    use chrono::{Local, TimeZone};
    for tag in &tags {
        if let Some(ts) = parse_last_reviewed_at(tag) {
            total_reviews += 1;
            if ts >= today_start {
                reviewed_today += 1;
            }
            if ts >= seven_days_ago {
                // Convert Unix seconds to local date string for bucketing.
                if let Some(dt) = Local.timestamp_opt(ts as i64, 0).single() {
                    let d = dt.date_naive().to_string();
                    *daily_buckets.entry(d).or_insert(0) += 1;
                }
            }
        }
    }

    // Build a dense last-7-days list (today and 6 prior days), filling 0.
    let mut reviews_last_7_days: Vec<DailyReviewCount> = Vec::new();
    let today_date = Local::now().date_naive();
    for i in (0..7).rev() {
        let d = today_date - chrono::Duration::days(i);
        let key = d.to_string();
        let count = *daily_buckets.get(&key).unwrap_or(&0);
        reviews_last_7_days.push(DailyReviewCount { date: key, count });
    }

    Ok(LearningStats {
        total_words,
        total_books,
        total_novels,
        by_proficiency,
        reviewed_today,
        total_reviews,
        reviews_last_7_days,
    })
}
