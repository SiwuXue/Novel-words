use crate::db::DbState;
use rusqlite::{Connection, DatabaseName};
use std::path::Path;
use tauri::{AppHandle, State};
use tauri_plugin_fs::FsExt;

/// Back up the entire SQLite database into a single self-contained file.
/// Uses SQLite's online backup API so the snapshot is consistent even while
/// WAL mode is enabled, and produces one portable `.db` file (no -wal/-shm).
#[tauri::command]
pub fn backup_database(
    app: AppHandle,
    state: State<DbState>,
    dest_path: String,
) -> Result<String, String> {
    // SQLite's backup API takes a filesystem path, while Android's save dialog
    // returns a content:// URI. Create the snapshot in the app temp directory,
    // then copy it through the fs plugin to support both URI kinds.
    let tmp_path = std::env::temp_dir().join(format!(
        "nw_backup_{}_{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let _ = std::fs::remove_file(&tmp_path);

    let guard = state.db.lock().map_err(|e| e.to_string())?;
    guard
        .backup(
            DatabaseName::Main,
            &tmp_path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .map_err(|e| format!("备份数据库失败: {}", e))?;
    drop(guard);

    let bytes = std::fs::read(&tmp_path).map_err(|e| format!("读取备份文件失败: {}", e))?;
    let _ = std::fs::remove_file(&tmp_path);
    let mut options = tauri_plugin_fs::OpenOptions::new();
    options.read(false).write(true).create(true).truncate(true);
    let mut dest = app
        .fs()
        .open(dest_path.parse::<tauri_plugin_fs::FilePath>().unwrap(), options)
        .map_err(|e| format!("无法创建备份文件: {}", e))?;
    std::io::Write::write_all(&mut dest, &bytes)
        .map_err(|e| format!("写入备份文件失败: {}", e))?;
    Ok(dest_path)
}

/// Restore the database from a previously created backup file.
/// The live connection is overwritten in place; the frontend should reload the
/// window afterwards so every store re-reads the restored data.
#[tauri::command]
pub fn restore_database(
    app: AppHandle,
    state: State<DbState>,
    src_path: String,
) -> Result<(), String> {
    let tmp_path = std::env::temp_dir().join(format!(
        "nw_restore_{}_{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let bytes = app
        .fs()
        .read(src_path.parse::<tauri_plugin_fs::FilePath>().unwrap())
        .map_err(|e| format!("无法读取备份文件: {}", e))?;
    std::fs::write(&tmp_path, bytes).map_err(|e| format!("无法准备恢复文件: {}", e))?;

    // Validate the source before touching the live database.
    {
        let src = Connection::open(&tmp_path).map_err(|e| format!("无法打开备份文件: {}", e))?;
        let is_novel_words: bool = src
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='novel'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);
        if !is_novel_words {
            let _ = std::fs::remove_file(&tmp_path);
            return Err("所选文件不是有效的词阅数据库备份（缺少 novel 表）".into());
        }
    }

    let mut guard = state.db.lock().map_err(|e| e.to_string())?;
    guard
        .restore(
            DatabaseName::Main,
            &tmp_path,
            None::<fn(rusqlite::backup::Progress)>,
        )
        .map_err(|e| format!("恢复数据库失败: {}", e))?;
    let _ = std::fs::remove_file(&tmp_path);
    Ok(())
}

/// Auto-backup on startup, based on the `auto_backup` setting
/// ("off" | "daily" | "weekly" | "monthly", default weekly). Backs up into
/// `<app_data_dir>/backups/auto-YYYYMMDD-HHMMSS.db`, keeps the newest 10, and
/// records `last_auto_backup` so we don't back up on every launch.
pub fn auto_backup(app_data_dir: &Path, state: &State<DbState>) {
    let interval_secs: u64 = {
        let db = match state.db.lock() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("[auto-backup] 锁获取失败: {}", e);
                return;
            }
        };
        let val: Result<String, _> = db.query_row(
            "SELECT value FROM app_settings WHERE key='auto_backup'",
            [],
            |row| row.get(0),
        );
        match val.ok().as_deref() {
            Some("off") => return,
            Some("daily") => 24 * 3600,
            Some("monthly") => 30 * 24 * 3600,
            _ => 7 * 24 * 3600, // weekly (also the default when unset)
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Last successful auto-backup timestamp.
    let last: u64 = {
        let db = match state.db.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let val: Result<String, _> = db.query_row(
            "SELECT value FROM app_settings WHERE key='last_auto_backup'",
            [],
            |row| row.get(0),
        );
        val.ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    };
    if last > 0 && now.saturating_sub(last) < interval_secs {
        return;
    }

    let dir = app_data_dir.join("backups");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("[auto-backup] 创建备份目录失败: {}", e);
        return;
    }
    let fname = format!("auto-{}.db", crate::utils::date::timestamp_compact());
    let dest = dir.join(&fname);

    {
        let db = match state.db.lock() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("[auto-backup] 锁获取失败: {}", e);
                return;
            }
        };
        if let Err(e) = db.backup(DatabaseName::Main, &dest, None::<fn(rusqlite::backup::Progress)>)
        {
            eprintln!("[auto-backup] 备份失败: {}", e);
            return;
        }
    }

    // Prune old auto-* files, keeping only the newest 10.
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut files: Vec<std::path::PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("auto-") && n.ends_with(".db"))
                    .unwrap_or(false)
            })
            .collect();
        files.sort();
        while files.len() > 10 {
            if let Some(old) = files.first() {
                let _ = std::fs::remove_file(old);
            }
            files.remove(0);
        }
    }

    {
        let db = match state.db.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let _ = db.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('last_auto_backup', ?1)",
            rusqlite::params![now.to_string()],
        );
    }
    println!("[auto-backup] 已生成自动备份: {}", dest.display());
}
