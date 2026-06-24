use crate::db::DbState;
use crate::models::AppSetting;
use tauri::State;

#[tauri::command]
pub fn get_setting(state: State<DbState>, key: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let result: Result<String, _> = db.query_row(
        "SELECT value FROM app_settings WHERE key=?1",
        rusqlite::params![key],
        |row| row.get(0),
    );
    match result {
        Ok(value) => Ok(value),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(String::new()),
        Err(e) => Err(format!("读取设置失败: {}", e)),
    }
}

#[tauri::command]
pub fn set_setting(state: State<DbState>, key: String, value: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )
    .map_err(|e| format!("保存设置失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_all_settings(state: State<DbState>) -> Result<Vec<AppSetting>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT key, value FROM app_settings")
        .map_err(|e| e.to_string())?;

    let settings = stmt
        .query_map([], |row| {
            Ok(AppSetting {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(settings)
}
