mod commands;
mod db;
mod dict_online;
mod dictionary;
mod license;
mod models;
mod pdf;
mod preset_catalog;
mod user_vocab;
mod utils;

use license::state::{activate_license, get_license_status, verify_license, LicenseState};
#[cfg(target_os = "android")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "android")]
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
#[cfg(target_os = "android")]
use tauri_plugin_fs::FsExt;

use commands::ai_enhancer::{
    get_ai_settings, list_ai_models, save_ai_settings, test_ai_connection,
};
use commands::app_info::get_app_info;
use commands::backup::{backup_database, restore_database};
use commands::chapter::{
    delete_chapters_by_novel, get_chapter_content, get_chapter_list, get_chapters, save_chapters,
    update_chapter_content, update_chapter_title,
};
use commands::export::{
    export_vocab_book_json, export_vocab_words_apkg, export_vocab_words_xlsx,
    import_vocab_book_json,
};
use commands::file_io::{import_file, read_text_file, write_text_file};
use commands::novel::{
    create_novel, delete_novel, get_all_novels, get_novel, get_novel_content, get_novel_meta,
    search_novels, update_novel, update_novel_content, update_novel_metadata,
};
use commands::pdf_export::export_pdf;
use commands::pdf_template::{
    create_pdf_template, delete_pdf_template, get_all_pdf_templates, get_builtin_templates,
    update_pdf_template,
};
use commands::preset_vocab::{
    commit_preset_clone, commit_preset_clone_with_state, import_preset_vocab_book,
    list_preset_vocab_books, preview_preset_clone, repair_cloned_parts_of_speech,
};
use commands::review::{
    get_all_due_words, get_due_words, get_due_words_count, get_learning_stats, get_review_progress,
    review_vocab_word,
};
use commands::settings::{get_all_settings, get_setting, set_setting};
use commands::user_vocab::{
    get_user_vocab_page, lookup_user_vocab, lookup_word_tap_phrases, lookup_word_tap_states,
    mark_word_tap_proficiency, set_user_vocab_proficiency,
};
use commands::vocab_book::{
    create_vocab_book, delete_vocab_book, get_all_vocab_books, import_cet4_core_words,
    update_vocab_book,
};
use commands::vocab_word::{
    create_vocab_word, delete_vocab_word, delete_vocab_words, export_vocab_words_csv,
    get_highlight_words, get_vocab_words, get_vocab_words_page, import_vocab_words_csv,
    search_vocab_words, update_vocab_word,
};
use dictionary::{dict_lookup_chinese, dict_lookup_english, DictDbState};
use dict_online::{dict_online_cambridge, dict_online_youdao, dict_translate_sentence};

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
    std::fs::create_dir_all(cache_dir).map_err(|e| format!("无法创建资源缓存目录: {}", e))?;
    let cached_path = cache_dir.join(file_name);
    if !cached_path.exists() {
        let bytes = app
            .fs()
            .read(resource_path)
            .map_err(|e| format!("读取内置资源失败: {}", e))?;
        std::fs::write(&cached_path, bytes).map_err(|e| format!("写入资源缓存失败: {}", e))?;
    }
    Ok(cached_path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        // 本地朗读音频流：…/stream/{novelId}。
        // 音频路径保存在 app_settings(novel_audio_{id})，URL 只暴露小说 id 不暴露路径，
        // 处理器按 id 回查数据库后再读文件，天然限制了可访问范围。
        .register_uri_scheme_protocol("novelaudio", |ctx, request| {
            let not_found = || {
                tauri::http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap()
            };
            let uri = request.uri().to_string();
            let id: i64 = uri
                .rsplit('/')
                .next()
                .and_then(|s| s.split('?').next())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            if id <= 0 {
                return not_found();
            }
            let key = format!("novel_audio_{id}");
            let path: String = {
                let db = ctx.app_handle().state::<crate::db::DbState>();
                let conn = match db.db.lock() {
                    Ok(c) => c,
                    Err(_) => return not_found(),
                };
                conn.query_row(
                    "SELECT value FROM app_settings WHERE key=?1",
                    [&key],
                    |r| r.get(0),
                )
                .unwrap_or_default()
            };
            if path.trim().is_empty() {
                return not_found();
            }
            let bytes = match std::fs::read(&path) {
                Ok(b) => b,
                Err(_) => return not_found(),
            };
            let ext = path
                .rsplit('.')
                .next()
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            let content_type = match ext.as_str() {
                "mp3" => "audio/mpeg",
                "m4a" => "audio/mp4",
                "aac" => "audio/aac",
                "wav" => "audio/wav",
                "ogg" | "opus" => "audio/ogg",
                "flac" => "audio/flac",
                _ => "application/octet-stream",
            };
            tauri::http::Response::builder()
                .status(200)
                .header("Content-Type", content_type)
                .header("Access-Control-Allow-Origin", "*")
                .body(bytes)
                .unwrap()
        })
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

            let mut db_state =
                db::init_db(&app_data_dir).map_err(|e| format!("数据库初始化失败: {}", e))?;

            let preset_path = resource_dir.join("resources").join("presets.zip");
            #[cfg(target_os = "android")]
            let preset_path = materialize_resource(
                &app.handle(),
                &preset_path,
                &app_data_dir.join("resources"),
                "presets.zip",
            )?;
            let catalog = preset_catalog::PresetCatalog { path: preset_path };
            {
                let conn = db_state.db.get_mut().map_err(|e| e.to_string())?;
                if let Err(error) = catalog.register(conn) {
                    eprintln!("[preset] 目录注册失败，页面可重试: {}", error);
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
            app.manage(catalog);
            let license_state = (|| -> Result<LicenseState, license::LicenseError> {
                let config = license::ClientConfig::compiled()?;
                let device = license::storage::stable_device_id(&app_data_dir, &config.product_id)?;
                LicenseState::new(config, &app_data_dir, &device)
            })();
            match license_state {
                Ok(state) => {
                    app.manage(state);
                    let license_app = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        let state = license_app.state::<LicenseState>();
                        if let Ok(status) = state.verify().await {
                            let _ = license_app.emit("license-state-changed", status);
                        } else if let Ok(status) = state.status() {
                            let _ = license_app.emit("license-state-changed", status);
                        }
                    });
                }
                Err(_) => eprintln!("[license] Initialization failed: LICENSING_UNAVAILABLE"),
            }

            // Auto-backup on startup (best effort; failures are logged only).
            {
                let state = app.state::<db::DbState>();
                commands::backup::auto_backup(&app_data_dir, &state);
            }

            // Initialize embedded dictionary (read-only). Failure here is
            // non-fatal: dict_lookup_* commands will return errors and the
            // app continues without lookup feature.
            let bundled_dict_path = resource_dir.join("resources").join("dictionary.db");
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
        .invoke_handler(|invoke| {
            let command = invoke.message.command();
            if let Some(state) = invoke.message.state_ref().try_get::<LicenseState>() {
                let payload = match invoke.message.payload() {
                    tauri::ipc::InvokeBody::Json(value) => Some(value),
                    tauri::ipc::InvokeBody::Raw(_) => None,
                };
                if let Err(error) = state.guard_with_payload(command, payload) {
                    if let Ok(status) = state.status() {
                        let _ = invoke
                            .message
                            .webview_ref()
                            .app_handle()
                            .emit("license-state-changed", status);
                    }
                    invoke.resolver.reject(error);
                    return true;
                }
            } else if matches!(
                command,
                "get_license_status" | "verify_license" | "activate_license"
            ) || !license::state::public_command(command)
            {
                invoke
                    .resolver
                    .reject(license::LicenseError::new("LICENSING_UNAVAILABLE"));
                return true;
            }
            let handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
                get_license_status,
                verify_license,
                activate_license,
                create_novel,
                get_all_novels,
                get_novel,
                get_novel_meta,
                get_novel_content,
                update_novel,
                update_novel_content,
                update_novel_metadata,
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
                get_user_vocab_page,
                lookup_user_vocab,
                set_user_vocab_proficiency,
                mark_word_tap_proficiency,
                lookup_word_tap_states,
                lookup_word_tap_phrases,
                import_preset_vocab_book,
                commit_preset_clone_with_state,
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
                get_chapter_list,
                get_chapter_content,
                update_chapter_content,
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
                dict_online_youdao,
                dict_online_cambridge,
                dict_translate_sentence,
            ];
            handler(invoke)
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
