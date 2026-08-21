use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
}

/// Expose the app version and the on-disk data directory, so the "About" dialog
/// can tell users where their data lives (for backup / migration).
#[tauri::command]
pub fn get_app_info(app: AppHandle) -> Result<AppInfo, String> {
    let version = app.package_info().version.to_string();
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法解析数据目录: {}", e))?
        .to_string_lossy()
        .to_string();
    Ok(AppInfo { version, data_dir })
}
