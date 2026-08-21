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

    // Reject duplicates within the same book (case-insensitive on trimmed word)
    let exists: bool = db
        .prepare("SELECT COUNT(*) > 0 FROM vocab_word WHERE vocab_book_id = ?1 AND word = ?2")
        .and_then(|mut s| s.query_row(rusqlite::params![vocab_book_id, word.trim()], |r| r.get(0)))
        .unwrap_or(false);
    if exists {
        return Err(format!("单词「{}」已存在", word.trim()));
    }

    db.execute(
        "INSERT INTO vocab_word (vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![vocab_book_id, word.trim(), definition, phonetic, example_sentence, novel_id, proficiency, memory_tag],
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
pub fn delete_vocab_words(state: State<DbState>, ids: Vec<i64>) -> Result<u32, String> {
    if ids.is_empty() {
        return Ok(0);
    }
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let mut count: u32 = 0;
    {
        let mut stmt = tx
            .prepare("DELETE FROM vocab_word WHERE id = ?1")
            .map_err(|e| e.to_string())?;
        for id in &ids {
            count += stmt
                .execute(rusqlite::params![id])
                .map_err(|e| format!("批量删除失败: {}", e))? as u32;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
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

pub(crate) fn row_to_vocab_word(row: &rusqlite::Row) -> rusqlite::Result<VocabWord> {
    Ok(VocabWord {
        id: row.get(0)?,
        vocab_book_id: row.get(1)?,
        word: row.get(2)?,
        definition: row.get(3)?,
        phonetic: row.get(4)?,
        example_sentence: row.get(5)?,
        novel_id: row.get(6)?,
        chapter_id: None,
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

#[tauri::command]
pub fn export_vocab_words_csv(
    state: State<DbState>,
    vocab_book_id: i64,
    file_path: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT word, definition, phonetic, example_sentence, proficiency, memory_tag FROM vocab_word WHERE vocab_book_id=?1 ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let mut wtr = csv::Writer::from_path(&file_path)
        .map_err(|e| format!("无法创建文件: {}", e))?;

    wtr.write_record(&["word", "definition", "phonetic", "example_sentence", "proficiency", "memory_tag"])
        .map_err(|e| format!("写入 CSV 失败: {}", e))?;

    let rows = stmt
        .query_map(rusqlite::params![vocab_book_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (w, d, p, es, prof, mt) = row.map_err(|e| e.to_string())?;
        wtr.write_record(&[&w, &d, &p, &es, &prof, &mt])
            .map_err(|e| format!("写入 CSV 失败: {}", e))?;
    }

    wtr.flush().map_err(|e| format!("CSV flush 失败: {}", e))?;
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: u32,
    pub skipped: u32,
}

#[tauri::command]
pub fn import_vocab_words_csv(
    state: State<DbState>,
    vocab_book_id: i64,
    file_path: String,
) -> Result<ImportResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut rdr =
        csv::Reader::from_path(&file_path).map_err(|e| format!("无法打开文件: {}", e))?;

    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;

    for result in rdr.records() {
        let record = result.map_err(|e| format!("CSV 解析失败: {}", e))?;

        let word = record.get(0).unwrap_or("").trim();
        if word.is_empty() {
            continue;
        }

        let definition = record.get(1).unwrap_or("").trim();
        let phonetic = record.get(2).unwrap_or("").trim();
        let example_sentence = record.get(3).unwrap_or("").trim();
        let proficiency_raw = record.get(4).unwrap_or("").trim();
        let memory_tag = record.get(5).unwrap_or("").trim();

        let proficiency = match proficiency_raw {
            "familiar" | "mastered" => proficiency_raw,
            _ => "unknown",
        };

        // INSERT OR IGNORE relies on the unique (vocab_book_id, word) index to
        // silently skip duplicates (both against existing rows and earlier rows
        // in the same file).
        let n = db
            .execute(
                "INSERT OR IGNORE INTO vocab_word (vocab_book_id, word, definition, phonetic, example_sentence, proficiency, memory_tag) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![vocab_book_id, word, definition, phonetic, example_sentence, proficiency, memory_tag],
            )
            .map_err(|e| format!("导入单词 '{}' 失败: {}", word, e))?;

        if n > 0 {
            imported += 1;
        } else {
            skipped += 1;
        }
    }

    Ok(ImportResult { imported, skipped })
}
