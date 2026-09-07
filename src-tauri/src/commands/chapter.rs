use crate::db::DbState;
use crate::models::Chapter;
use tauri::State;

/// Save chapters for a novel while reusing IDs by position. Reusing IDs keeps
/// vocab words linked to their source chapter when only the chapter content
/// changes.
#[tauri::command]
pub fn save_chapters(
    state: State<DbState>,
    novel_id: i64,
    chapters: Vec<Chapter>,
) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db
        .transaction()
        .map_err(|e| format!("开启章节保存事务失败: {}", e))?;
    let old_ids: Vec<i64> = {
        let mut stmt = tx
            .prepare("SELECT id FROM chapter WHERE novel_id=?1 ORDER BY sort_order")
            .map_err(|e| format!("读取旧章节失败: {}", e))?;
        let rows = stmt.query_map(rusqlite::params![novel_id], |row| row.get(0))
            .map_err(|e| format!("读取旧章节失败: {}", e))?
            .filter_map(|r| r.ok())
            .collect::<Vec<i64>>();
        rows
    };
    for (i, ch) in chapters.iter().enumerate() {
        if let Some(id) = old_ids.get(i) {
            tx.execute(
                "UPDATE chapter SET title=?1, content=?2, sort_order=?3 WHERE id=?4 AND novel_id=?5",
                rusqlite::params![ch.title, ch.content, i as i32, id, novel_id],
            )
            .map_err(|e| format!("更新章节 '{}' 失败: {}", ch.title, e))?;
        } else {
            tx.execute(
                "INSERT INTO chapter (novel_id, title, content, sort_order) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![novel_id, ch.title, ch.content, i as i32],
            )
            .map_err(|e| format!("插入章节 '{}' 失败: {}", ch.title, e))?;
        }
    }
    for id in old_ids.iter().skip(chapters.len()) {
        tx.execute(
            "UPDATE vocab_word SET chapter_id=NULL WHERE chapter_id=?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("清理章节词汇关联失败: {}", e))?;
        tx.execute("DELETE FROM chapter WHERE id=?1", rusqlite::params![id])
            .map_err(|e| format!("删除旧章节失败: {}", e))?;
    }
    tx.commit().map_err(|e| format!("提交章节保存失败: {}", e))?;
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
