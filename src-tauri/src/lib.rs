mod commands;
mod db;
mod models;
mod pdf;
mod utils;

use tauri::Manager;

use commands::file_io::import_text_file;
use commands::novel::{
    create_novel, delete_novel, get_all_novels, get_novel, search_novels, update_novel,
};
use commands::vocab_book::{
    create_vocab_book, delete_vocab_book, get_all_vocab_books, update_vocab_book,
};
use commands::vocab_word::{
    create_vocab_word, delete_vocab_word, export_vocab_words_csv, get_highlight_words,
    get_vocab_words, import_vocab_words_csv, search_vocab_words, update_vocab_word,
};
use commands::pdf_template::{
    create_pdf_template, delete_pdf_template, get_all_pdf_templates, get_builtin_templates,
    update_pdf_template,
};
use commands::pdf_export::export_pdf;
use commands::settings::{get_all_settings, get_setting, set_setting};

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
            create_vocab_word,
            get_vocab_words,
            update_vocab_word,
            delete_vocab_word,
            search_vocab_words,
            get_highlight_words,
            export_vocab_words_csv,
            import_vocab_words_csv,
            create_pdf_template,
            get_all_pdf_templates,
            update_pdf_template,
            delete_pdf_template,
            get_builtin_templates,
            export_pdf,
            get_setting,
            set_setting,
            get_all_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
