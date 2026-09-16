use crate::db::DbState;
use rusqlite::{Connection, DatabaseName};
use std::path::Path;
use tauri::{AppHandle, Manager, State};
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
        .open(
            dest_path.parse::<tauri_plugin_fs::FilePath>().unwrap(),
            options,
        )
        .map_err(|e| format!("无法创建备份文件: {}", e))?;
    std::io::Write::write_all(&mut dest, &bytes).map_err(|e| format!("写入备份文件失败: {}", e))?;
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
    let bytes = app
        .fs()
        .read(
            src_path
                .parse::<tauri_plugin_fs::FilePath>()
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("无法读取备份文件: {}", e))?;
    let mut guard = state.db.lock().map_err(|e| e.to_string())?;
    let catalog = app.state::<crate::preset_catalog::PresetCatalog>();
    restore_bytes(&mut guard, &bytes, Some(&catalog))
}

struct RestoreFile(std::path::PathBuf);
impl Drop for RestoreFile {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", self.0.display(), suffix));
        }
    }
}
pub(crate) fn restore_bytes(
    live: &mut Connection,
    bytes: &[u8],
    catalog: Option<&crate::preset_catalog::PresetCatalog>,
) -> Result<(), String> {
    let path = std::env::temp_dir().join(format!(
        "nw_restore_{}_{}.db",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    let file = RestoreFile(path);
    std::fs::write(&file.0, bytes).map_err(|e| e.to_string())?;
    {
        let mut source = Connection::open(&file.0).map_err(|e| format!("无法打开备份: {}", e))?;
        for table in ["novel", "vocab_book", "vocab_word"] {
            let exists: bool = source
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                    [table],
                    |r| r.get(0),
                )
                .map_err(|e| format!("无效备份: {}", e))?;
            if !exists {
                return Err(format!("所选文件不是词阅数据库备份（缺少 {} 表）", table));
            }
        }
        let integrity: String = source
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if integrity != "ok" {
            return Err(format!("备份完整性检查失败: {}", integrity));
        }
        crate::db::migrate_connection(&mut source)?;
        if let Some(catalog) = catalog {
            catalog.register(&source)?;
        }
        let violations: i64 = source
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        if violations != 0 {
            return Err("备份存在无效的关联数据，未恢复".into());
        }
        source
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| e.to_string())?;
    }
    live.restore(
        DatabaseName::Main,
        &file.0,
        None::<fn(rusqlite::backup::Progress)>,
    )
    .map_err(|e| format!("恢复失败: {}", e))?;
    live.execute_batch("PRAGMA foreign_keys=ON;")
        .map_err(|e| e.to_string())?;
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
        val.ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
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
        if let Err(e) = db.backup(
            DatabaseName::Main,
            &dest,
            None::<fn(rusqlite::backup::Progress)>,
        ) {
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

#[cfg(test)]
mod tests {
    #[test]
    fn restore_migrates_legacy_copy_and_invalid_restore_preserves_live_state() {
        let dir = std::env::temp_dir().join(format!(
            "nw-restore-test-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("legacy.db");
        let conn = rusqlite::Connection::open(&source).unwrap();
        conn.execute_batch(crate::db::CREATE_TABLES_SQL).unwrap();
        conn.execute_batch("INSERT INTO vocab_book(name) VALUES ('旧词汇本');INSERT INTO vocab_word(vocab_book_id,word,proficiency) VALUES (1,'garden','familiar');").unwrap();
        drop(conn);
        let bytes = std::fs::read(&source).unwrap();
        let state = crate::db::init_db(&dir.join("live")).unwrap();
        let mut live = state.db.lock().unwrap();
        super::restore_bytes(&mut live, &bytes, None).unwrap();
        assert_eq!(
            live.query_row("SELECT proficiency FROM user_vocab", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "familiar"
        );
        assert_eq!(
            std::fs::read(&source).unwrap(),
            bytes,
            "The selected backup must remain unchanged"
        );
        assert!(super::restore_bytes(&mut live, b"not a database", None).is_err());
        assert_eq!(
            live.query_row("SELECT COUNT(*) FROM user_vocab", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(live);
        drop(state);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
