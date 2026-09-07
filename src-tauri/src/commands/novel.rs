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
    language: Option<String>,
) -> Result<Novel, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let lang = language.as_deref().unwrap_or("zh");
    db.execute(
        "INSERT INTO novel (title, author, category, raw_text, cleaned_text, language) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![title, author, category, raw_text, cleaned_text, lang],
    )
    .map_err(|e| format!("创建小说失败: {}", e))?;

    let id = db.last_insert_rowid();
    get_novel_by_id(&db, id)
}

#[tauri::command]
pub fn get_all_novels(state: State<DbState>) -> Result<Vec<Novel>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, title, author, category, raw_text, cleaned_text, is_favorite, language, created_at, updated_at FROM novel ORDER BY updated_at DESC")
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

/// Load novel metadata without transferring the full text columns.
#[tauri::command]
pub fn get_novel_meta(state: State<DbState>, id: i64) -> Result<Novel, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.query_row(
        "SELECT id, title, author, category, is_favorite, language, created_at, updated_at FROM novel WHERE id=?1",
        rusqlite::params![id],
        |row| {
            Ok(Novel {
                id: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                category: row.get(3)?,
                raw_text: String::new(),
                cleaned_text: String::new(),
                is_favorite: row.get(4)?,
                language: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .map_err(|e| format!("未找到该小说: {}", e))
}

/// Load the legacy full text only for novels that do not have saved chapters.
#[tauri::command]
pub fn get_novel_content(
    state: State<DbState>,
    id: i64,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.query_row(
        "SELECT CASE WHEN cleaned_text != '' THEN cleaned_text ELSE raw_text END FROM novel WHERE id=?1",
        rusqlite::params![id],
        |row| row.get(0),
    )
    .map_err(|e| format!("读取小说正文失败: {}", e))
}

/// Update only the legacy full-text fallback column.
#[tauri::command]
pub fn update_novel_content(
    state: State<DbState>,
    id: i64,
    cleaned_text: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let affected = db
        .execute(
            "UPDATE novel SET cleaned_text=?1, updated_at=datetime('now','localtime') WHERE id=?2",
            rusqlite::params![cleaned_text, id],
        )
        .map_err(|e| format!("保存小说正文失败: {}", e))?;
    if affected == 0 {
        return Err("小说不存在".into());
    }
    Ok(())
}

/// Update metadata without requiring the caller to send the full novel body.
#[tauri::command]
pub fn update_novel_metadata(
    state: State<DbState>,
    id: i64,
    title: String,
    author: String,
    category: String,
    is_favorite: bool,
    language: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let lang = language.unwrap_or_else(|| "zh".to_string());
    let affected = db
        .execute(
            "UPDATE novel SET title=?1, author=?2, category=?3, is_favorite=?4, language=?5, updated_at=datetime('now','localtime') WHERE id=?6",
            rusqlite::params![title, author, category, is_favorite as i32, lang, id],
        )
        .map_err(|e| format!("更新小说信息失败: {}", e))?;
    if affected == 0 {
        return Err("小说不存在".into());
    }
    Ok(())
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
    language: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    // Read existing language if not provided so callers that don't pass it
    // don't blow it away.
    let lang = match language {
        Some(l) => l,
        None => db
            .query_row(
                "SELECT language FROM novel WHERE id=?1",
                rusqlite::params![id],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "zh".to_string()),
    };
    let affected = db
        .execute(
            "UPDATE novel SET title=?1, author=?2, category=?3, raw_text=?4, cleaned_text=?5, is_favorite=?6, language=?7, updated_at=datetime('now','localtime') WHERE id=?8",
            rusqlite::params![title, author, category, raw_text, cleaned_text, is_favorite as i32, lang, id],
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
    // Cascade cleanup so we don't leave orphan rows when a novel is deleted.
    db.execute("DELETE FROM chapter WHERE novel_id=?1", rusqlite::params![id])
        .map_err(|e| format!("清理章节失败: {}", e))?;
    // Detach vocab words from this novel (keep the words in their books; only
    // clear the novel_id link so high-light lists & statistics stay consistent).
    db.execute(
        "UPDATE vocab_word SET novel_id=NULL WHERE novel_id=?1",
        rusqlite::params![id],
    )
    .map_err(|e| format!("更新生词归属失败: {}", e))?;
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
            "SELECT id, title, author, category, raw_text, cleaned_text, is_favorite, language, created_at, updated_at \
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
        "SELECT id, title, author, category, raw_text, cleaned_text, is_favorite, language, created_at, updated_at FROM novel WHERE id=?1",
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
        is_favorite: row.get(6)?,
        language: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}
