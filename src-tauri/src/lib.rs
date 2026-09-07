mod commands;
mod db;
mod dictionary;
mod models;
mod pdf;
mod utils;

#[cfg(target_os = "android")]
use std::path::{Path, PathBuf};
use tauri::Manager;
#[cfg(target_os = "android")]
use tauri::AppHandle;
#[cfg(target_os = "android")]
use tauri_plugin_fs::FsExt;

use commands::file_io::{import_file, read_text_file, write_text_file};
use commands::novel::{
    create_novel, delete_novel, get_all_novels, get_novel, search_novels, update_novel,
};
use commands::vocab_book::{
    create_vocab_book, delete_vocab_book, ensure_preset_book_populated, get_all_vocab_books,
    import_cet4_core_words, update_vocab_book, BUNDLED_PRESETS,
};
use commands::vocab_word::{
    create_vocab_word, delete_vocab_word, delete_vocab_words, export_vocab_words_csv,
    get_highlight_words, get_vocab_words, get_vocab_words_page, import_vocab_words_csv,
    search_vocab_words, update_vocab_word,
};
use commands::pdf_template::{
    create_pdf_template, delete_pdf_template, get_all_pdf_templates, get_builtin_templates,
    update_pdf_template,
};
use commands::chapter::{
    delete_chapters_by_novel, get_chapters, save_chapters, update_chapter_title,
};
use commands::app_info::get_app_info;
use commands::backup::{backup_database, restore_database};
use commands::export::{
    export_vocab_book_json, export_vocab_words_apkg, export_vocab_words_xlsx, import_vocab_book_json,
};
use commands::pdf_export::export_pdf;
use commands::preset_vocab::{
    commit_preset_clone, list_preset_vocab_books, preview_preset_clone,
    repair_cloned_parts_of_speech,
};
use commands::review::{get_all_due_words, get_due_words, get_due_words_count, get_learning_stats, get_review_progress, review_vocab_word};
use commands::settings::{get_all_settings, get_setting, set_setting};
use commands::ai_enhancer::{get_ai_settings, list_ai_models, save_ai_settings, test_ai_connection};
use dictionary::{dict_lookup_chinese, dict_lookup_english, DictDbState};

/// Android packages resources as APK assets, represented by an `asset://` URI.
/// The SQLite and import code works with normal filesystem paths, so materialize
/// those bundled files into app data once before handing them to that code.
#[cfg(target_os = "android")]
fn materialize_resource(
    app: &AppHandle,
    resource_path: &Path,
    cache_dir: &Path,
    file_name: &str,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(cache_dir)
        .map_err(|e| format!("无法创建资源缓存目录: {}", e))?;
    let cached_path = cache_dir.join(file_name);
    if !cached_path.exists() {
        let bytes = app
            .fs()
            .read(resource_path)
            .map_err(|e| format!("读取内置资源失败: {}", e))?;
        std::fs::write(&cached_path, bytes)
            .map_err(|e| format!("写入资源缓存失败: {}", e))?;
    }
    Ok(cached_path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Resolve app data directory and initialize main database
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("无法解析数据目录: {}", e))?;

            let resource_dir = app
                .path()
                .resource_dir()
                .map_err(|e| format!("无法解析资源目录: {}", e))?;

            let mut db_state = db::init_db(&app_data_dir)
                .map_err(|e| format!("数据库初始化失败: {}", e))?;

            // ---- Auto-seed bundled preset vocab books on first launch ----
            // IMPORTANT: must happen before `app.manage(db_state)` which moves
            // db_state, otherwise we can't get a mutable ref again without
            // re-locking. Locking is avoided here since this is the single
            // setup thread.
            {
                let conn = db_state.db.get_mut().map_err(|e| e.to_string())?;
                for preset in BUNDLED_PRESETS {
                    let bundled_path = resource_dir.join("resources").join(preset.file_name);
                    #[cfg(target_os = "android")]
                    let path = materialize_resource(
                        &app.handle(),
                        &bundled_path,
                        &app_data_dir.join("resources"),
                        preset.file_name,
                    )?;
                    #[cfg(not(target_os = "android"))]
                    let path = bundled_path;
                    match ensure_preset_book_populated(&path, preset, conn) {
                        Ok(res) => {
                            println!(
                                "[preset:{}] 导入完成：新增 {} / 跳过 {} / 总数 {}",
                                preset.preset_key, res.imported, res.skipped, res.total_in_file
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "[preset:{}] 预装失败（不阻断启动）: {}",
                                preset.preset_key, e
                            );
                        }
                    }
                }
                match repair_cloned_parts_of_speech(conn) {
                    Ok(count) if count > 0 => {
                        println!("[preset] 已为 {} 条旧 AI 词汇补回词性", count);
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("[preset] 修复旧 AI 词性失败（不阻断启动）: {}", e),
                }
            }
            // ---- Database integrity check (run before `app.manage` moves db_state) ----
            {
                let conn = db_state.db.get_mut().map_err(|e| e.to_string())?;
                let integrity: Result<String, _> =
                    conn.query_row("PRAGMA integrity_check", [], |row| row.get(0));
                match integrity {
                    Ok(ref s) if s == "ok" => {}
                    Ok(s) => eprintln!("[integrity] 数据库完整性异常: {}", s),
                    Err(e) => eprintln!("[integrity] 完整性检查出错: {}", e),
                }
                let fk_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM (PRAGMA foreign_key_check)",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                if fk_count > 0 {
                    eprintln!(
                        "[integrity] 外键检查: {} 行违反（PRAGMA foreign_key_check）",
                        fk_count
                    );
                }
            }
            app.manage(db_state);

            // Auto-backup on startup (best effort; failures are logged only).
            {
                let state = app.state::<db::DbState>();
                commands::backup::auto_backup(&app_data_dir, &state);
            }

            // Initialize embedded dictionary (read-only). Failure here is
            // non-fatal: dict_lookup_* commands will return errors and the
            // app continues without lookup feature.
            let bundled_dict_path = resource_dir
                .join("resources")
                .join("dictionary.db");
            #[cfg(target_os = "android")]
            let dict_db_path = materialize_resource(
                &app.handle(),
                &bundled_dict_path,
                &app_data_dir.join("resources"),
                "dictionary.db",
            )?;
            #[cfg(not(target_os = "android"))]
            let dict_db_path = bundled_dict_path;
            match DictDbState::open(dict_db_path) {
                Ok(state) => {
                    app.manage(state);
                }
                Err(e) => {
                    eprintln!("[dictionary] 词典库初始化失败（不阻断启动）: {}", e);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_novel,
            get_all_novels,
            get_novel,
            update_novel,
            delete_novel,
            search_novels,
            import_file,
            write_text_file,
            read_text_file,
            create_vocab_book,
            get_all_vocab_books,
            update_vocab_book,
            delete_vocab_book,
            import_cet4_core_words,
            create_vocab_word,
            get_vocab_words,
            get_vocab_words_page,
            update_vocab_word,
            delete_vocab_word,
            delete_vocab_words,
            search_vocab_words,
            get_highlight_words,
            export_vocab_words_csv,
            export_vocab_words_xlsx,
            export_vocab_words_apkg,
            export_vocab_book_json,
            import_vocab_book_json,
            list_preset_vocab_books,
            preview_preset_clone,
            commit_preset_clone,
            get_due_words,
            get_all_due_words,
            get_due_words_count,
            get_review_progress,
            get_learning_stats,
            review_vocab_word,
            import_vocab_words_csv,
            create_pdf_template,
            get_all_pdf_templates,
            update_pdf_template,
            delete_pdf_template,
            get_builtin_templates,
            save_chapters,
            get_chapters,
            update_chapter_title,
            delete_chapters_by_novel,
            export_pdf,
            backup_database,
            restore_database,
            get_app_info,
            get_setting,
            set_setting,
            get_all_settings,
            get_ai_settings,
            list_ai_models,
            save_ai_settings,
            test_ai_connection,
            dict_lookup_english,
            dict_lookup_chinese,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
