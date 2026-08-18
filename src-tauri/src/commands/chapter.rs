use crate::db::DbState;
use crate::models::Chapter;
use tauri::State;

/// Save chapters for a novel. Replaces all existing chapters for the novel
/// with the provided list (delete then re-insert).
#[tauri::command]
pub fn save_chapters(
    state: State<DbState>,
    novel_id: i64,
    chapters: Vec<Chapter>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "DELETE FROM chapter WHERE novel_id = ?1",
        rusqlite::params![novel_id],
    )
    .map_err(|e| format!("清理旧章节失败: {}", e))?;

    let mut stmt = db
        .prepare(
            "INSERT INTO chapter (novel_id, title, content, sort_order) VALUES (?1, ?2, ?3, ?4)",
        )
        .map_err(|e| format!("准备插入章节失败: {}", e))?;

    for (i, ch) in chapters.iter().enumerate() {
        stmt.execute(rusqlite::params![novel_id, ch.title, ch.content, i as i32])
            .map_err(|e| format!("插入章节 '{}' 失败: {}", ch.title, e))?;
    }
    Ok(())
}

/// Load chapters for a novel, ordered by sort_order.
#[tauri::command]
pub fn get_chapters(
    state: State<DbState>,
    novel_id: i64,
) -> Result<Vec<Chapter>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, novel_id, title, content, sort_order, created_at FROM chapter WHERE novel_id = ?1 ORDER BY sort_order",
        )
        .map_err(|e| e.to_string())?;

    let chapters = stmt
        .query_map(rusqlite::params![novel_id], |row| {
            Ok(Chapter {
                id: row.get(0)?,
                novel_id: row.get(1)?,
                title: row.get(2)?,
                content: row.get(3)?,
                sort_order: row.get(4)?,
                start_index: 0,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(chapters)
}

/// Update a single chapter's title.
#[tauri::command]
pub fn update_chapter_title(
    state: State<DbState>,
    id: i64,
    title: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let affected = db
        .execute(
            "UPDATE chapter SET title = ?1 WHERE id = ?2",
            rusqlite::params![title, id],
        )
        .map_err(|e| format!("更新章节标题失败: {}", e))?;
    if affected == 0 {
        return Err("章节不存在".into());
    }
    Ok(())
}

/// Delete all chapters for a novel.
#[tauri::command]
pub fn delete_chapters_by_novel(
    state: State<DbState>,
    novel_id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "DELETE FROM chapter WHERE novel_id = ?1",
        rusqlite::params![novel_id],
    )
    .map_err(|e| format!("删除章节失败: {}", e))?;
    Ok(())
}
