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
    let mut conn = Connection::open(&db_path).map_err(|e| format!("无法打开数据库: {}", e))?;

    // Enable WAL mode for concurrent reads during writes
    conn.execute_batch("PRAGMA journal_mode=WAL;").map_err(|e| e.to_string())?;
    // Enable foreign key constraints
    conn.execute_batch("PRAGMA foreign_keys=ON;").map_err(|e| e.to_string())?;

    // Run DDL
    conn.execute_batch(CREATE_TABLES_SQL).map_err(|e| format!("建表失败: {}", e))?;

    // Migrations: only run when column doesn't exist yet
    {
        let has_col: bool = conn
            .prepare("SELECT COUNT(*) > 0 FROM pragma_table_info('pdf_template') WHERE name = 'updated_at'")
            .and_then(|mut s| s.query_row([], |r| r.get(0)))
            .unwrap_or(false);
        if !has_col {
            // SQLite ALTER TABLE ADD COLUMN only allows constant defaults,
            // so we add with '' then populate existing rows via UPDATE.
            conn.execute_batch(
                "ALTER TABLE pdf_template ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
                 UPDATE pdf_template SET updated_at = datetime('now','localtime') WHERE updated_at = '';",
            )
            .map_err(|e| format!("迁移 pdf_template.updated_at 失败: {}", e))?;
        }
    }

    // Migration: add chapter_id to vocab_word
    {
        let has_col: bool = conn
            .prepare("SELECT COUNT(*) > 0 FROM pragma_table_info('vocab_word') WHERE name = 'chapter_id'")
            .and_then(|mut s| s.query_row([], |r| r.get(0)))
            .unwrap_or(false);
        if !has_col {
            conn.execute_batch(
                "ALTER TABLE vocab_word ADD COLUMN chapter_id INTEGER;",
            )
            .map_err(|e| format!("迁移 vocab_word.chapter_id 失败: {}", e))?;
        }
    }

    // Persist the exact Chinese terms confirmed during preset tailoring.  Older
    // tailored books only kept an example sentence, so reconstruct their
    // reliable terms once from the primary definition + captured excerpt.
    {
        let has_col: bool = conn
            .prepare("SELECT COUNT(*) > 0 FROM pragma_table_info('vocab_word') WHERE name = 'match_terms'")
            .and_then(|mut s| s.query_row([], |r| r.get(0)))
            .unwrap_or(false);
        if !has_col {
            conn.execute_batch(
                "ALTER TABLE vocab_word ADD COLUMN match_terms TEXT NOT NULL DEFAULT '';",
            )
            .map_err(|e| format!("迁移 vocab_word.match_terms 失败: {}", e))?;
        }

        let tx = conn
            .transaction()
            .map_err(|e| format!("开启旧裁剪词汇迁移失败: {}", e))?;
        let legacy_rows: Vec<(i64, String, String)> = {
            let mut stmt = tx
                .prepare(
                    "SELECT id, definition, example_sentence FROM vocab_word \
                     WHERE match_terms = '' AND novel_id IS NOT NULL AND trim(example_sentence) <> ''",
                )
                .map_err(|e| format!("读取旧裁剪词汇失败: {}", e))?;
            let rows = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .map_err(|e| format!("读取旧裁剪词汇失败: {}", e))?
                .filter_map(Result::ok)
                .collect();
            rows
        };
        {
            let mut update = tx
                .prepare("UPDATE vocab_word SET match_terms = ?1 WHERE id = ?2")
                .map_err(|e| format!("准备回填旧裁剪词汇失败: {}", e))?;
            for (id, definition, example) in legacy_rows {
                let primary = definition.split('【').next().unwrap_or(&definition);
                let mut terms = Vec::new();
                let mut current = String::new();
                let flush = |current: &mut String, terms: &mut Vec<String>| {
                    if current.chars().count() >= 2
                        && example.contains(current.as_str())
                        && !terms.contains(current)
                    {
                        terms.push(current.clone());
                    }
                    current.clear();
                };
                for ch in primary.chars() {
                    if ('\u{3400}'..='\u{4dbf}').contains(&ch)
                        || ('\u{4e00}'..='\u{9fff}').contains(&ch)
                        || ('\u{f900}'..='\u{faff}').contains(&ch)
                    {
                        current.push(ch);
                    } else {
                        flush(&mut current, &mut terms);
                    }
                }
                flush(&mut current, &mut terms);
                if !terms.is_empty() {
                    let encoded = serde_json::to_string(&terms).unwrap_or_default();
                    update
                        .execute(rusqlite::params![encoded, id])
                        .map_err(|e| format!("回填旧裁剪词汇匹配词失败: {}", e))?;
                }
            }
        }
        tx.commit()
            .map_err(|e| format!("提交旧裁剪词汇迁移失败: {}", e))?;
    }

    // Migration: add template_type + is_builtin to pdf_template
    {
        let has_col: bool = conn
            .prepare("SELECT COUNT(*) > 0 FROM pragma_table_info('pdf_template') WHERE name = 'template_type'")
            .and_then(|mut s| s.query_row([], |r| r.get(0)))
            .unwrap_or(false);
        if !has_col {
            conn.execute_batch(
                "ALTER TABLE pdf_template ADD COLUMN template_type TEXT NOT NULL DEFAULT 'appendix';
                 ALTER TABLE pdf_template ADD COLUMN is_builtin INTEGER NOT NULL DEFAULT 0;",
            )
            .map_err(|e| format!("迁移 pdf_template.template_type 失败: {}", e))?;
            // Migrate existing annotation_mode values to template_type
            conn.execute_batch(
                "UPDATE pdf_template SET template_type = 'intensive' WHERE annotation_mode = 'inline';
                 UPDATE pdf_template SET template_type = 'intensive' WHERE template_type != 'intensive';",
            )
            .map_err(|e| format!("迁移 template_type 值失败: {}", e))?;
        }
    }

    // Migration: enforce unique (vocab_book_id, word) — dedup existing rows first,
    // keeping the earliest id, then create a unique index.
    {
        let has_index: bool = conn
            .prepare("SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name='idx_vocab_word_unique'")
            .and_then(|mut s| s.query_row([], |r| r.get(0)))
            .unwrap_or(false);
        if !has_index {
            conn.execute_batch(
                "DELETE FROM vocab_word
                 WHERE id NOT IN (
                     SELECT MIN(id) FROM vocab_word GROUP BY vocab_book_id, word
                 );
                 CREATE UNIQUE INDEX idx_vocab_word_unique ON vocab_word (vocab_book_id, word);",
            )
            .map_err(|e| format!("迁移 vocab_word 唯一索引失败: {}", e))?;
        }
    }

    // Migration: add language to novel so each book can be tagged as 'zh' or 'en'
    // for matching mode (per-novel granularity).
    {
        let has_col: bool = conn
            .prepare("SELECT COUNT(*) > 0 FROM pragma_table_info('novel') WHERE name = 'language'")
            .and_then(|mut s| s.query_row([], |r| r.get(0)))
            .unwrap_or(false);
        if !has_col {
            conn.execute_batch(
                "ALTER TABLE novel ADD COLUMN language TEXT NOT NULL DEFAULT 'zh';
                 UPDATE novel SET language = 'zh' WHERE language = '';",
            )
            .map_err(|e| format!("迁移 novel.language 失败: {}", e))?;
        }
    }

    // Migration: preset concept on vocab_book (read-only bundled lists like CET4)
    // plus a cloned_from_preset_key back-reference on the user clone.
    {
        let has_preset: bool = conn
            .prepare("SELECT COUNT(*) > 0 FROM pragma_table_info('vocab_book') WHERE name = 'is_preset'")
            .and_then(|mut s| s.query_row([], |r| r.get(0)))
            .unwrap_or(false);
        if !has_preset {
            conn.execute_batch(
                "ALTER TABLE vocab_book ADD COLUMN is_preset INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE vocab_book ADD COLUMN preset_key TEXT NOT NULL DEFAULT '';
                 ALTER TABLE vocab_book ADD COLUMN cloned_from_preset_key TEXT NOT NULL DEFAULT '';",
            )
            .map_err(|e| format!("迁移 vocab_book preset 列失败: {}", e))?;
            // Mark the existing CET4 seeded book as a preset so it becomes
            // read-only and gets the "study / clone" flow.
            conn.execute_batch(
                "UPDATE vocab_book SET is_preset = 1, preset_key = 'cet4'
                 WHERE name = '四级真题核心词' AND is_preset = 0;",
            )
            .map_err(|e| format!("迁移 CET4 为 preset 失败: {}", e))?;
        }
    }

    // Seed default settings (inserted here so future-added keys land too)
    conn.execute_batch(
        "INSERT OR IGNORE INTO app_settings (key, value) VALUES ('speech_accent', 'us');",
    )
    .map_err(|e| format!("seed speech_accent 失败: {}", e))?;

    // Migration: preserve every review as an append-only event.  Older
    // versions only kept the last review timestamp inside vocab_word.memory_tag;
    // seed one best-effort legacy event for already-reviewed cards so existing
    // users do not start with an entirely empty history.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS review_log (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            vocab_word_id   INTEGER NOT NULL,
            vocab_book_id   INTEGER NOT NULL,
            rating          TEXT NOT NULL DEFAULT 'legacy',
            reviewed_at     INTEGER NOT NULL,
            due_before      TEXT NOT NULL DEFAULT '',
            due_after       TEXT NOT NULL DEFAULT '',
            proficiency     TEXT NOT NULL DEFAULT '',
            FOREIGN KEY (vocab_word_id) REFERENCES vocab_word(id) ON DELETE CASCADE,
            FOREIGN KEY (vocab_book_id) REFERENCES vocab_book(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_review_log_reviewed_at ON review_log(reviewed_at);
        CREATE INDEX IF NOT EXISTS idx_review_log_word ON review_log(vocab_word_id);
        INSERT INTO review_log (
            vocab_word_id, vocab_book_id, rating, reviewed_at, due_after, proficiency
        )
        SELECT w.id, w.vocab_book_id, 'legacy',
               CAST(json_extract(w.memory_tag, '$.last_reviewed_at') AS INTEGER),
               COALESCE(json_extract(w.memory_tag, '$.srs.due'), ''),
               w.proficiency
        FROM vocab_word w
        WHERE json_valid(w.memory_tag)
          AND json_extract(w.memory_tag, '$.last_reviewed_at') IS NOT NULL
          AND NOT EXISTS (
              SELECT 1 FROM review_log r WHERE r.vocab_word_id = w.id
          );",
    )
    .map_err(|e| format!("迁移复习历史失败: {}", e))?;

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
    language     TEXT    NOT NULL DEFAULT 'zh',
    created_at   TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS vocab_book (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL DEFAULT '',
    is_preset          INTEGER NOT NULL DEFAULT 0,
    preset_key         TEXT    NOT NULL DEFAULT '',
    cloned_from_preset_key TEXT NOT NULL DEFAULT '',
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
    match_terms      TEXT    NOT NULL DEFAULT '',
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
    created_at      TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);

CREATE TABLE IF NOT EXISTS chapter (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    novel_id   INTEGER NOT NULL,
    title      TEXT    NOT NULL DEFAULT '',
    content    TEXT    NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    FOREIGN KEY (novel_id) REFERENCES novel(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_chapter_novel ON chapter(novel_id);

CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS review_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    vocab_word_id   INTEGER NOT NULL,
    vocab_book_id   INTEGER NOT NULL,
    rating          TEXT NOT NULL DEFAULT 'legacy',
    reviewed_at     INTEGER NOT NULL,
    due_before      TEXT NOT NULL DEFAULT '',
    due_after       TEXT NOT NULL DEFAULT '',
    proficiency     TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (vocab_word_id) REFERENCES vocab_word(id) ON DELETE CASCADE,
    FOREIGN KEY (vocab_book_id) REFERENCES vocab_book(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_review_log_reviewed_at ON review_log(reviewed_at);
CREATE INDEX IF NOT EXISTS idx_review_log_word ON review_log(vocab_word_id);

-- Seed default settings
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('theme', 'light');
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('default_export_folder', '');
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('default_vocab_book_id', '');
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('pdf_intensive_steps', '[1,2,3]');
";

#[cfg(test)]
mod tests {
    use super::init_db;

    #[test]
    fn backfills_reliable_terms_for_existing_tailored_words() {
        let unique = format!(
            "novel-words-db-migration-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);

        {
            let state = init_db(&dir).unwrap();
            let db = state.db.lock().unwrap();
            db.execute("INSERT INTO novel (title) VALUES ('测试小说')", [])
                .unwrap();
            db.execute("INSERT INTO vocab_book (name) VALUES ('测试精选')", [])
                .unwrap();
            db.execute(
                "INSERT INTO vocab_word \
                 (vocab_book_id, word, definition, example_sentence, novel_id, match_terms) \
                 VALUES (1, 'gift', 'n. 天赋；礼物', '他的修炼天赋十分出众。', 1, '')",
                [],
            )
            .unwrap();
        }

        let state = init_db(&dir).unwrap();
        let encoded: String = state
            .db
            .lock()
            .unwrap()
            .query_row("SELECT match_terms FROM vocab_word WHERE word='gift'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(serde_json::from_str::<Vec<String>>(&encoded).unwrap(), vec!["天赋"]);

        drop(state);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn creates_review_log_and_seeds_legacy_review_once() {
        let unique = format!(
            "novel-words-review-migration-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);
        let reviewed_at = 1_700_000_000_u64;

        {
            let state = init_db(&dir).unwrap();
            let db = state.db.lock().unwrap();
            db.execute("INSERT INTO vocab_book (name) VALUES ('测试词汇本')", [])
                .unwrap();
            let memory_tag = format!(
                "{{\"tag\":\"\",\"srs\":{{\"due\":\"2026-01-01\"}},\"last_reviewed_at\":{}}}",
                reviewed_at
            );
            db.execute(
                "INSERT INTO vocab_word (vocab_book_id, word, memory_tag) VALUES (1, 'test', ?1)",
                rusqlite::params![memory_tag],
            )
            .unwrap();
        }

        let state = init_db(&dir).unwrap();
        let db = state.db.lock().unwrap();
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM review_log", [], |row| row.get(0))
            .unwrap();
        let timestamp: i64 = db
            .query_row("SELECT reviewed_at FROM review_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(timestamp, reviewed_at as i64);
        drop(db);
        drop(state);

        let state = init_db(&dir).unwrap();
        let count: i64 = state
            .db
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM review_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        drop(state);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
