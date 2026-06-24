use crate::db::DbState;
use crate::models::{HighlightWord, VocabWord};
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub fn create_vocab_word(
    state: State<DbState>,
    vocab_book_id: i64,
    word: String,
    definition: String,
    phonetic: String,
    example_sentence: String,
    novel_id: Option<i64>,
    proficiency: String,
    memory_tag: String,
) -> Result<VocabWord, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO vocab_word (vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag],
    )
    .map_err(|e| format!("创建单词失败: {}", e))?;

    let id = db.last_insert_rowid();
    get_vocab_word_by_id(&db, id)
}

#[tauri::command]
pub fn get_vocab_words(
    state: State<DbState>,
    vocab_book_id: i64,
) -> Result<Vec<VocabWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at FROM vocab_word WHERE vocab_book_id=?1 ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let words = stmt
        .query_map(rusqlite::params![vocab_book_id], row_to_vocab_word)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(words)
}

#[tauri::command]
pub fn update_vocab_word(
    state: State<DbState>,
    id: i64,
    word: String,
    definition: String,
    phonetic: String,
    example_sentence: String,
    proficiency: String,
    memory_tag: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let affected = db
        .execute(
            "UPDATE vocab_word SET word=?1, definition=?2, phonetic=?3, example_sentence=?4, proficiency=?5, memory_tag=?6 WHERE id=?7",
            rusqlite::params![word, definition, phonetic, example_sentence, proficiency, memory_tag, id],
        )
        .map_err(|e| format!("更新单词失败: {}", e))?;

    if affected == 0 {
        return Err("单词不存在".into());
    }
    Ok(())
}

#[tauri::command]
pub fn delete_vocab_word(state: State<DbState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM vocab_word WHERE id=?1", rusqlite::params![id])
        .map_err(|e| format!("删除单词失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn search_vocab_words(
    state: State<DbState>,
    vocab_book_id: i64,
    query: String,
) -> Result<Vec<VocabWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let pattern = format!("%{}%", query);
    let mut stmt = db
        .prepare(
            "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at FROM vocab_word WHERE vocab_book_id=?1 AND word LIKE ?2 ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let words = stmt
        .query_map(rusqlite::params![vocab_book_id, pattern], row_to_vocab_word)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(words)
}

fn get_vocab_word_by_id(db: &rusqlite::Connection, id: i64) -> Result<VocabWord, String> {
    db.query_row(
        "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at FROM vocab_word WHERE id=?1",
        rusqlite::params![id],
        row_to_vocab_word,
    )
    .map_err(|e| format!("未找到该单词: {}", e))
}

fn row_to_vocab_word(row: &rusqlite::Row) -> rusqlite::Result<VocabWord> {
    Ok(VocabWord {
        id: row.get(0)?,
        vocab_book_id: row.get(1)?,
        word: row.get(2)?,
        definition: row.get(3)?,
        phonetic: row.get(4)?,
        example_sentence: row.get(5)?,
        novel_id: row.get(6)?,
        proficiency: row.get(7)?,
        memory_tag: row.get(8)?,
        created_at: row.get(9)?,
    })
}

#[tauri::command]
pub fn get_highlight_words(
    state: State<DbState>,
    vocab_book_id: i64,
) -> Result<Vec<HighlightWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT word, definition, phonetic, example_sentence, proficiency FROM vocab_word WHERE vocab_book_id=?1 ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<HighlightWord> = stmt
        .query_map(rusqlite::params![vocab_book_id], |row| {
            Ok(HighlightWord {
                word: row.get(0)?,
                definition: row.get(1)?,
                phonetic: row.get(2)?,
                example_sentence: row.get(3)?,
                proficiency: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Deduplicate by word, keeping the first occurrence
    let mut seen: HashMap<String, HighlightWord> = HashMap::new();
    for hw in rows {
        seen.entry(hw.word.clone()).or_insert(hw);
    }
    Ok(seen.into_values().collect())
}
