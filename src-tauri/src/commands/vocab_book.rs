use crate::db::DbState;
use crate::models::VocabBook;
use tauri::State;

#[tauri::command]
pub fn create_vocab_book(
    state: State<DbState>,
    name: String,
    description: String,
) -> Result<VocabBook, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO vocab_book (name, description) VALUES (?1, ?2)",
        rusqlite::params![name, description],
    )
    .map_err(|e| format!("创建词汇本失败: {}", e))?;

    let id = db.last_insert_rowid();
    get_vocab_book_by_id(&db, id)
}

#[tauri::command]
pub fn get_all_vocab_books(state: State<DbState>) -> Result<Vec<VocabBook>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, name, description, created_at, updated_at FROM vocab_book ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let books = stmt
        .query_map([], row_to_vocab_book)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(books)
}

#[tauri::command]
pub fn update_vocab_book(
    state: State<DbState>,
    id: i64,
    name: String,
    description: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let affected = db
        .execute(
            "UPDATE vocab_book SET name=?1, description=?2, updated_at=datetime('now','localtime') WHERE id=?3",
            rusqlite::params![name, description, id],
        )
        .map_err(|e| format!("更新词汇本失败: {}", e))?;

    if affected == 0 {
        return Err("词汇本不存在".into());
    }
    Ok(())
}

#[tauri::command]
pub fn delete_vocab_book(state: State<DbState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM vocab_book WHERE id=?1", rusqlite::params![id])
        .map_err(|e| format!("删除词汇本失败: {}", e))?;
    Ok(())
}

fn get_vocab_book_by_id(db: &rusqlite::Connection, id: i64) -> Result<VocabBook, String> {
    db.query_row(
        "SELECT id, name, description, created_at, updated_at FROM vocab_book WHERE id=?1",
        rusqlite::params![id],
        row_to_vocab_book,
    )
    .map_err(|e| format!("未找到该词汇本: {}", e))
}

fn row_to_vocab_book(row: &rusqlite::Row) -> rusqlite::Result<VocabBook> {
    Ok(VocabBook {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}
