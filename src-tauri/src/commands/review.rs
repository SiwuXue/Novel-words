use crate::commands::vocab_word::row_to_vocab_word;
use crate::db::DbState;
use crate::models::VocabWord;
use crate::utils::srs::{apply_rating, is_due, parse_memory_tag, serialize_memory_tag};
use tauri::State;

/// Return all words in a book that are due for review today.
/// A card without any SRS state is considered new and therefore due.
#[tauri::command]
pub fn get_due_words(state: State<DbState>, vocab_book_id: i64) -> Result<Vec<VocabWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at \
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
            "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at \
             FROM vocab_word WHERE id=?1",
            rusqlite::params![id],
            row_to_vocab_word,
        )
        .map_err(|e| format!("未找到该单词: {}", e))?;

    let (tag, mut srs) = parse_memory_tag(&word.memory_tag);
    let proficiency = apply_rating(&mut srs, &rating);
    let new_tag = serialize_memory_tag(&tag, &srs);

    db.execute(
        "UPDATE vocab_word SET proficiency=?1, memory_tag=?2 WHERE id=?3",
        rusqlite::params![proficiency, new_tag, id],
    )
    .map_err(|e| format!("更新单词失败: {}", e))?;

    let updated = db
        .query_row(
            "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at \
             FROM vocab_word WHERE id=?1",
            rusqlite::params![id],
            row_to_vocab_word,
        )
        .map_err(|e| format!("未找到该单词: {}", e))?;

    Ok(updated)
}
