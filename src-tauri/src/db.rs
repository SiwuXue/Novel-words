use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct DbState {
    pub db: Mutex<Connection>,
}

/// Initialize the SQLite database at the given app data directory.
/// Creates the directory and database file if they don't exist,
/// runs DDL to create all tables, and enables WAL + foreign keys.
pub fn init_db(app_data_dir: &PathBuf) -> Result<DbState, String> {
    // Ensure the directory exists
    fs::create_dir_all(app_data_dir).map_err(|e| format!("无法创建数据目录: {}", e))?;

    let db_path = app_data_dir.join("novel_words.db");
    let conn = Connection::open(&db_path).map_err(|e| format!("无法打开数据库: {}", e))?;

    // Enable WAL mode for concurrent reads during writes
    conn.execute_batch("PRAGMA journal_mode=WAL;").map_err(|e| e.to_string())?;
    // Enable foreign key constraints
    conn.execute_batch("PRAGMA foreign_keys=ON;").map_err(|e| e.to_string())?;

    // Run DDL
    conn.execute_batch(CREATE_TABLES_SQL).map_err(|e| format!("建表失败: {}", e))?;

    Ok(DbState {
        db: Mutex::new(conn),
    })
}

const CREATE_TABLES_SQL: &str = "
CREATE TABLE IF NOT EXISTS novel (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    title        TEXT    NOT NULL DEFAULT '',
    author       TEXT    NOT NULL DEFAULT '',
    category     TEXT    NOT NULL DEFAULT '',
    raw_text     TEXT    NOT NULL DEFAULT '',
    cleaned_text TEXT    NOT NULL DEFAULT '',
    is_favorite  INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS vocab_book (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL DEFAULT '',
    created_at  TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS vocab_word (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    vocab_book_id    INTEGER NOT NULL,
    word             TEXT    NOT NULL,
    definition       TEXT    NOT NULL DEFAULT '',
    phonetic         TEXT    NOT NULL DEFAULT '',
    example_sentence TEXT    NOT NULL DEFAULT '',
    novel_id         INTEGER,
    proficiency      TEXT    NOT NULL DEFAULT 'unknown'
                             CHECK(proficiency IN ('unknown', 'familiar', 'mastered')),
    memory_tag       TEXT    NOT NULL DEFAULT '',
    created_at       TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    FOREIGN KEY (vocab_book_id) REFERENCES vocab_book(id) ON DELETE CASCADE,
    FOREIGN KEY (novel_id) REFERENCES novel(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_vocab_word_book ON vocab_word(vocab_book_id);
CREATE INDEX IF NOT EXISTS idx_vocab_word_novel ON vocab_word(novel_id);

CREATE TABLE IF NOT EXISTS pdf_template (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    paper_size      TEXT    NOT NULL DEFAULT 'A4',
    font_family     TEXT    NOT NULL DEFAULT 'SimSun',
    font_size       INTEGER NOT NULL DEFAULT 14,
    line_spacing    REAL    NOT NULL DEFAULT 1.5,
    margins         TEXT    NOT NULL DEFAULT '{\"top\":25,\"bottom\":25,\"left\":20,\"right\":20}',
    annotation_mode TEXT    NOT NULL DEFAULT 'appendix',
    created_at      TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Seed default settings
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('theme', 'light');
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('default_export_folder', '');
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('default_vocab_book_id', '');
";
