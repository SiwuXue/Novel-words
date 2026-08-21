mod commands;
mod db;
mod dictionary;
mod models;
mod pdf;
mod utils;

use tauri::Manager;

use commands::file_io::import_text_file;
use commands::novel::{
    create_novel, delete_novel, get_all_novels, get_novel, search_novels, update_novel,
};
use commands::vocab_book::{
    create_vocab_book, delete_vocab_book, ensure_cet4_book_populated, get_all_vocab_books,
    import_cet4_core_words, update_vocab_book,
};
use commands::vocab_word::{
    create_vocab_word, delete_vocab_word, delete_vocab_words, export_vocab_words_csv,
    get_highlight_words, get_vocab_words, import_vocab_words_csv, search_vocab_words,
    update_vocab_word,
};
use commands::pdf_template::{
    create_pdf_template, delete_pdf_template, get_all_pdf_templates, get_builtin_templates,
    update_pdf_template,
};
use commands::chapter::{
    delete_chapters_by_novel, get_chapters, save_chapters, update_chapter_title,
};
use commands::backup::{backup_database, restore_database};
use commands::pdf_export::export_pdf;
use commands::settings::{get_all_settings, get_setting, set_setting};
use dictionary::{dict_lookup_chinese, dict_lookup_english, DictDbState};

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

            // ---- Auto-seed "四级真题核心词" vocab book on first launch ----
            // IMPORTANT: must happen before `app.manage(db_state)` which moves
            // db_state, otherwise we can't get a mutable ref again without
            // re-locking. Locking is avoided here since this is the single
            // setup thread.
            let cet4_path = resource_dir.join("resources").join("CET4luan_1.json");
            {
                let conn = db_state.db.get_mut().map_err(|e| e.to_string())?;
                match ensure_cet4_book_populated(&cet4_path, conn) {
                    Ok(res) => {
                        if res.imported > 0 {
                            println!(
                                "[CET4] 预装四级词汇本完成：新增 {} / 跳过 {} / 总数 {}",
                                res.imported, res.skipped, res.total_in_file
                            );
                        } else {
                            println!(
                                "[CET4] 四级词汇本已存在（跳过 {} 词）",
                                res.skipped
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("[CET4] 预装四级词汇本失败（不阻断启动）: {}", e);
                    }
                }
            }
            app.manage(db_state);

            // Initialize embedded dictionary (read-only). Failure here is
            // non-fatal: dict_lookup_* commands will return errors and the
            // app continues without lookup feature.
            let dict_db_path = resource_dir
                .join("resources")
                .join("dictionary.db");
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
            import_text_file,
            create_vocab_book,
            get_all_vocab_books,
            update_vocab_book,
            delete_vocab_book,
            import_cet4_core_words,
            create_vocab_word,
            get_vocab_words,
            update_vocab_word,
            delete_vocab_word,
            delete_vocab_words,
            search_vocab_words,
            get_highlight_words,
            export_vocab_words_csv,
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
            get_setting,
            set_setting,
            get_all_settings,
            dict_lookup_english,
            dict_lookup_chinese,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
