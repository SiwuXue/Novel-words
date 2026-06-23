use crate::db::DbState;
use crate::models::Novel;
use tauri::State;

#[tauri::command]
pub fn create_novel(
    state: State<DbState>,
    title: String,
    author: String,
    category: String,
    raw_text: String,
    cleaned_text: String,
) -> Result<Novel, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO novel (title, author, category, raw_text, cleaned_text) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![title, author, category, raw_text, cleaned_text],
    )
    .map_err(|e| format!("创建小说失败: {}", e))?;

    let id = db.last_insert_rowid();
    get_novel_by_id(&db, id)
}

#[tauri::command]
pub fn get_all_novels(state: State<DbState>) -> Result<Vec<Novel>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, title, author, category, raw_text, cleaned_text, is_favorite, created_at, updated_at FROM novel ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;

    let novels = stmt
        .query_map([], row_to_novel)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(novels)
}

#[tauri::command]
pub fn get_novel(state: State<DbState>, id: i64) -> Result<Novel, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    get_novel_by_id(&db, id)
}

#[tauri::command]
pub fn update_novel(
    state: State<DbState>,
    id: i64,
    title: String,
    author: String,
    category: String,
    raw_text: String,
    cleaned_text: String,
    is_favorite: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let affected = db
        .execute(
            "UPDATE novel SET title=?1, author=?2, category=?3, raw_text=?4, cleaned_text=?5, is_favorite=?6, updated_at=datetime('now','localtime') WHERE id=?7",
            rusqlite::params![title, author, category, raw_text, cleaned_text, is_favorite as i32, id],
        )
        .map_err(|e| format!("更新小说失败: {}", e))?;

    if affected == 0 {
        return Err("小说不存在".into());
    }
    Ok(())
}

#[tauri::command]
pub fn delete_novel(state: State<DbState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM novel WHERE id=?1", rusqlite::params![id])
        .map_err(|e| format!("删除小说失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn search_novels(state: State<DbState>, query: String) -> Result<Vec<Novel>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let pattern = format!("%{}%", query);
    let mut stmt = db
        .prepare(
            "SELECT id, title, author, category, raw_text, cleaned_text, is_favorite, created_at, updated_at \
             FROM novel WHERE title LIKE ?1 OR author LIKE ?1 OR category LIKE ?1 ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let novels = stmt
        .query_map(rusqlite::params![pattern], row_to_novel)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(novels)
}

fn get_novel_by_id(db: &rusqlite::Connection, id: i64) -> Result<Novel, String> {
    db.query_row(
        "SELECT id, title, author, category, raw_text, cleaned_text, is_favorite, created_at, updated_at FROM novel WHERE id=?1",
        rusqlite::params![id],
        row_to_novel,
    )
    .map_err(|e| format!("未找到该小说: {}", e))
}

fn row_to_novel(row: &rusqlite::Row) -> rusqlite::Result<Novel> {
    Ok(Novel {
        id: row.get(0)?,
        title: row.get(1)?,
        author: row.get(2)?,
        category: row.get(3)?,
        raw_text: row.get(4)?,
        cleaned_text: row.get(5)?,
        is_favorite: row.get::<_, i32>(6)? != 0,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}
