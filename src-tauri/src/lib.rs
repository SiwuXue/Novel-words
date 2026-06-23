mod commands;
mod db;
mod models;
mod utils;

use db::DbState;
use tauri::Manager;

use commands::novel::{
    create_novel, delete_novel, get_all_novels, get_novel, search_novels, update_novel,
};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Resolve app data directory and initialize database
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");

            let db_state = db::init_db(&app_data_dir)
                .expect("failed to initialize database");

            app.manage(db_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            create_novel,
            get_all_novels,
            get_novel,
            update_novel,
            delete_novel,
            search_novels
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
