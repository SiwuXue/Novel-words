use crate::db::DbState;
use rusqlite::{Connection, DatabaseName};
use tauri::State;

/// Back up the entire SQLite database into a single self-contained file.
/// Uses SQLite's online backup API so the snapshot is consistent even while
/// WAL mode is enabled, and produces one portable `.db` file (no -wal/-shm).
#[tauri::command]
pub fn backup_database(state: State<DbState>, dest_path: String) -> Result<String, String> {
    // Ensure the destination directory exists (defensive; the save dialog
    // normally guarantees this already).
    if let Some(parent) = std::path::Path::new(&dest_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("无法创建目标目录: {}", e))?;
        }
    }
    // Remove any existing file so the backup starts from a clean destination.
    if std::path::Path::new(&dest_path).exists() {
        std::fs::remove_file(&dest_path).map_err(|e| format!("无法覆盖已有文件: {}", e))?;
    }

    let guard = state.db.lock().map_err(|e| e.to_string())?;
    guard
        .backup(
            DatabaseName::Main,
            &dest_path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .map_err(|e| format!("备份数据库失败: {}", e))?;
    Ok(dest_path)
}

/// Restore the database from a previously created backup file.
/// The live connection is overwritten in place; the frontend should reload the
/// window afterwards so every store re-reads the restored data.
#[tauri::command]
pub fn restore_database(state: State<DbState>, src_path: String) -> Result<(), String> {
    // Validate the source before touching the live database.
    {
        let src = Connection::open(&src_path).map_err(|e| format!("无法打开备份文件: {}", e))?;
        let is_novel_words: bool = src
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='novel'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);
        if !is_novel_words {
            return Err("所选文件不是有效的词阅数据库备份（缺少 novel 表）".into());
        }
    }

    let mut guard = state.db.lock().map_err(|e| e.to_string())?;
    guard
        .restore(
            DatabaseName::Main,
            &src_path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .map_err(|e| format!("恢复数据库失败: {}", e))?;
    Ok(())
}
