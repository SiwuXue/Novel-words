//! Personal learning state, independent of book membership and source metadata.
use crate::utils::srs::{apply_rating, parse_memory_tag, serialize_memory_tag_reviewed, SrsState};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeMap;

pub fn word_key(word: &str) -> String {
    word.replace(['‘', '’'], "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub fn now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

fn rank(proficiency: &str) -> u8 {
    match proficiency {
        "mastered" => 2,
        "familiar" => 1,
        _ => 0,
    }
}

pub fn valid_proficiency(proficiency: &str) -> Result<(), String> {
    if matches!(proficiency, "unknown" | "familiar" | "mastered" | "ignore") {
        Ok(())
    } else {
        Err("无效的熟练度".into())
    }
}

fn initial_srs(proficiency: &str, mut srs: SrsState) -> SrsState {
    if proficiency == "ignore" {
        // 忽略档永不进入复习队列：把到期日推到极远
        srs.due = "9999-12-31".into();
        return srs;
    }
    if srs.due.is_empty() && proficiency != "unknown" {
        apply_rating(
            &mut srs,
            if proficiency == "mastered" {
                "easy"
            } else {
                "good"
            },
        );
    }
    srs
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS user_vocab (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 word_key TEXT NOT NULL UNIQUE,
 word TEXT NOT NULL,
 definition TEXT NOT NULL DEFAULT '',
 phonetic TEXT NOT NULL DEFAULT '',
 example_sentence TEXT NOT NULL DEFAULT '',
 proficiency TEXT NOT NULL DEFAULT 'unknown' CHECK(proficiency IN ('unknown','familiar','mastered','ignore')),
 srs_state TEXT NOT NULL DEFAULT '{}',
 last_reviewed_at INTEGER NOT NULL DEFAULT 0,
 state_updated_at INTEGER NOT NULL DEFAULT 0,
 created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
 updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
CREATE INDEX IF NOT EXISTS idx_user_vocab_proficiency ON user_vocab(proficiency);
";

#[derive(Clone)]
struct LegacyWord {
    id: i64,
    book: i64,
    word: String,
    definition: String,
    phonetic: String,
    example: String,
    proficiency: String,
    memory: String,
    preset: bool,
    reviewed: i64,
    log_id: i64,
    log_proficiency: String,
    due: String,
}

pub fn migration_needed(conn: &Connection) -> bool {
    schema_version(conn) < SCHEMA_VERSION
}

/// 当前 user_vocab 迁移版本（0 = 未迁移，1 = 个人总词汇库合并，2 = ignore 档）。
fn schema_version(conn: &Connection) -> i32 {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key='user_vocab_schema'",
        [],
        |r| r.get::<_, String>(0),
    )
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(0)
}

/// 逐词阅读的 ignore 档：user_vocab.proficiency 的 CHECK 需要扩展。
/// SQLite 无法修改 CHECK，只能整表重建；保留 id 使 vocab_word/review_log 的
/// 外键引用继续有效（PRAGMA foreign_keys 不能在事务内切换，因此放在独立事务外）。
fn migrate_v2(conn: &mut Connection) -> Result<(), String> {
    let table_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='user_vocab')",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let already_ok = if table_exists {
        let sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='user_vocab'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        sql.contains("'ignore'")
    } else {
        true
    };
    if already_ok {
        conn.execute_batch(
            "INSERT OR REPLACE INTO app_settings(key,value) VALUES ('user_vocab_schema','2');",
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    conn.execute_batch("PRAGMA foreign_keys=OFF;")
        .map_err(|e| e.to_string())?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启迁移事务失败: {}", e))?;
    tx.execute_batch(
        "CREATE TABLE user_vocab_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            word_key TEXT NOT NULL UNIQUE,
            word TEXT NOT NULL,
            definition TEXT NOT NULL DEFAULT '',
            phonetic TEXT NOT NULL DEFAULT '',
            example_sentence TEXT NOT NULL DEFAULT '',
            proficiency TEXT NOT NULL DEFAULT 'unknown' CHECK(proficiency IN ('unknown','familiar','mastered','ignore')),
            srs_state TEXT NOT NULL DEFAULT '{}',
            last_reviewed_at INTEGER NOT NULL DEFAULT 0,
            state_updated_at INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
        );
        INSERT INTO user_vocab_new (id,word_key,word,definition,phonetic,example_sentence,proficiency,srs_state,last_reviewed_at,state_updated_at,created_at,updated_at)
        SELECT id,word_key,word,definition,phonetic,example_sentence,proficiency,srs_state,last_reviewed_at,state_updated_at,created_at,updated_at FROM user_vocab;
        DROP TABLE user_vocab;
        ALTER TABLE user_vocab_new RENAME TO user_vocab;
        CREATE INDEX IF NOT EXISTS idx_user_vocab_proficiency ON user_vocab(proficiency);",
    )
    .map_err(|e| format!("user_vocab 迁移失败: {}", e))?;
    tx.commit().map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA foreign_keys=ON;
         INSERT OR REPLACE INTO app_settings(key,value) VALUES ('user_vocab_schema','2');",
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn migrate(conn: &mut Connection) -> Result<(), String> {
    if schema_version(conn) < 1 {
        migrate_v1(conn)?;
    }
    migrate_v2(conn)
}

/// v1：合并个人总词汇库（旧库从零建表，新库幂等）。
fn migrate_v1(conn: &mut Connection) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
    for (name, ddl) in [
        (
            "word_key",
            "ALTER TABLE vocab_word ADD COLUMN word_key TEXT NOT NULL DEFAULT '';",
        ),
        (
            "user_vocab_id",
            "ALTER TABLE vocab_word ADD COLUMN user_vocab_id INTEGER REFERENCES user_vocab(id);",
        ),
        (
            "source_keys",
            "ALTER TABLE vocab_word ADD COLUMN source_keys TEXT NOT NULL DEFAULT '[]';",
        ),
    ] {
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('vocab_word') WHERE name=?1)",
                [name],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !exists {
            tx.execute_batch(ddl).map_err(|e| e.to_string())?;
        }
    }
    // Rebuild before deduplication so deleting a membership never erases events.
    tx.execute_batch("CREATE TABLE review_log_new (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        vocab_word_id INTEGER REFERENCES vocab_word(id) ON DELETE SET NULL,
        vocab_book_id INTEGER REFERENCES vocab_book(id) ON DELETE SET NULL,
        user_vocab_id INTEGER REFERENCES user_vocab(id),
        word_snapshot TEXT NOT NULL DEFAULT '', book_name_snapshot TEXT NOT NULL DEFAULT '',
        rating TEXT NOT NULL DEFAULT 'legacy', reviewed_at INTEGER NOT NULL,
        due_before TEXT NOT NULL DEFAULT '', due_after TEXT NOT NULL DEFAULT '', proficiency TEXT NOT NULL DEFAULT ''
    );
    INSERT INTO review_log_new (id,vocab_word_id,vocab_book_id,word_snapshot,book_name_snapshot,rating,reviewed_at,due_before,due_after,proficiency)
    SELECT r.id,r.vocab_word_id,r.vocab_book_id,COALESCE(w.word,''),COALESCE(b.name,''),r.rating,r.reviewed_at,r.due_before,r.due_after,r.proficiency
    FROM review_log r LEFT JOIN vocab_word w ON w.id=r.vocab_word_id LEFT JOIN vocab_book b ON b.id=r.vocab_book_id;
    DROP TABLE review_log; ALTER TABLE review_log_new RENAME TO review_log;
    CREATE INDEX idx_review_log_reviewed_at ON review_log(reviewed_at);
    CREATE INDEX idx_review_log_word ON review_log(vocab_word_id);
    CREATE INDEX idx_review_log_user ON review_log(user_vocab_id);").map_err(|e|e.to_string())?;
    let rows: Vec<LegacyWord> = {
        let mut stmt=tx.prepare("SELECT w.id,w.vocab_book_id,w.word,w.definition,w.phonetic,w.example_sentence,w.proficiency,w.memory_tag,b.is_preset,
          COALESCE(r.reviewed_at,0),COALESCE(r.proficiency,''),COALESCE(r.due_after,''),COALESCE(r.id,0)
          FROM vocab_word w JOIN vocab_book b ON b.id=w.vocab_book_id
          LEFT JOIN review_log r ON r.id=(SELECT id FROM review_log WHERE vocab_word_id=w.id ORDER BY reviewed_at DESC,id DESC LIMIT 1)
          ORDER BY w.id").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(LegacyWord {
                    id: r.get(0)?,
                    book: r.get(1)?,
                    word: r.get(2)?,
                    definition: r.get(3)?,
                    phonetic: r.get(4)?,
                    example: r.get(5)?,
                    proficiency: r.get(6)?,
                    memory: r.get(7)?,
                    preset: r.get::<_, i64>(8)? != 0,
                    reviewed: r.get(9)?,
                    log_proficiency: r.get(10)?,
                    due: r.get(11)?,
                    log_id: r.get(12)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
    };
    let mut groups: BTreeMap<String, Vec<&LegacyWord>> = BTreeMap::new();
    for row in &rows {
        let key = word_key(&row.word);
        tx.execute(
            "UPDATE vocab_word SET word_key=?1 WHERE id=?2",
            params![key, row.id],
        )
        .map_err(|e| e.to_string())?;
        if !row.preset && !key.is_empty() {
            groups.entry(key).or_default().push(row);
        }
    }
    for (key, group) in &groups {
        let best = group
            .iter()
            .max_by_key(|w| {
                let tag_time = serde_json::from_str::<serde_json::Value>(&w.memory)
                    .ok()
                    .and_then(|v| v["last_reviewed_at"].as_i64())
                    .unwrap_or(0);
                let (_, srs) = parse_memory_tag(&w.memory);
                (
                    w.reviewed.max(tag_time),
                    if w.reviewed > 0 && w.reviewed >= tag_time {
                        w.log_id
                    } else {
                        0
                    },
                    rank(&w.proficiency),
                    !srs.due.is_empty(),
                    std::cmp::Reverse(w.id),
                )
            })
            .unwrap();
        let (tag, srs) = parse_memory_tag(&best.memory);
        let tag_time = serde_json::from_str::<serde_json::Value>(&best.memory)
            .ok()
            .and_then(|v| v["last_reviewed_at"].as_i64())
            .unwrap_or(0);
        let reviewed = best.reviewed.max(tag_time);
        let mut proficiency = best.proficiency.clone();
        let mut srs = srs;
        if best.reviewed > 0 && best.reviewed >= tag_time {
            if valid_proficiency(&best.log_proficiency).is_ok() {
                proficiency = best.log_proficiency.clone();
            }
            if !best.due.is_empty() {
                srs.due = best.due.clone();
            }
        }
        srs = initial_srs(&proficiency, srs);
        let metadata = |get: fn(&LegacyWord) -> &str| {
            group
                .iter()
                .find_map(|w| {
                    let value = get(w);
                    (!value.is_empty()).then_some(value.to_owned())
                })
                .unwrap_or_default()
        };
        tx.execute("INSERT INTO user_vocab (word_key,word,definition,phonetic,example_sentence,proficiency,srs_state,last_reviewed_at,state_updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8)",params![key,best.word.trim(),metadata(|w|&w.definition),metadata(|w|&w.phonetic),metadata(|w|&w.example),proficiency,serde_json::to_string(&srs).map_err(|e|e.to_string())?,reviewed]).map_err(|e|e.to_string())?;
        let uid = tx.last_insert_rowid();
        for w in group {
            let local_tag = parse_memory_tag(&w.memory).0;
            tx.execute(
                "UPDATE vocab_word SET user_vocab_id=?1,memory_tag=?2 WHERE id=?3",
                params![uid, local_tag, w.id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "UPDATE review_log SET user_vocab_id=?1 WHERE vocab_word_id=?2",
                params![uid, w.id],
            )
            .map_err(|e| e.to_string())?;
        }
        let _ = tag;
    }
    // Preserve the earliest membership, filling missing metadata from duplicates.
    let mut seen = BTreeMap::new();
    for w in &rows {
        let key = (w.book, word_key(&w.word));
        if let Some(keep) = seen.get(&key) {
            tx.execute("UPDATE vocab_word SET definition=CASE WHEN definition='' THEN ?1 ELSE definition END, phonetic=CASE WHEN phonetic='' THEN ?2 ELSE phonetic END, example_sentence=CASE WHEN example_sentence='' THEN ?3 ELSE example_sentence END, memory_tag=CASE WHEN memory_tag='' THEN (SELECT memory_tag FROM vocab_word WHERE id=?4) ELSE memory_tag END, novel_id=COALESCE(novel_id,(SELECT novel_id FROM vocab_word WHERE id=?4)), chapter_id=COALESCE(chapter_id,(SELECT chapter_id FROM vocab_word WHERE id=?4)), source_keys=CASE WHEN source_keys IN ('','[]') THEN (SELECT source_keys FROM vocab_word WHERE id=?4) ELSE source_keys END, match_terms=CASE WHEN match_terms IN ('','[]') THEN (SELECT match_terms FROM vocab_word WHERE id=?4) ELSE match_terms END WHERE id=?5",params![w.definition,w.phonetic,w.example,w.id,keep]).map_err(|e|e.to_string())?;
            tx.execute(
                "UPDATE review_log SET vocab_word_id=?1 WHERE vocab_word_id=?2",
                params![keep, w.id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM vocab_word WHERE id=?1", [w.id])
                .map_err(|e| e.to_string())?;
        } else {
            seen.insert(key, w.id);
        }
    }
    tx.execute_batch(
        "DROP INDEX IF EXISTS idx_vocab_word_unique;
       CREATE UNIQUE INDEX idx_vocab_word_unique ON vocab_word(vocab_book_id,word_key);
       CREATE INDEX idx_vocab_word_user ON vocab_word(user_vocab_id);
       INSERT OR REPLACE INTO app_settings(key,value) VALUES ('user_vocab_schema','1');",
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    migrate_v2(conn)
}

/// 逐词阅读 ignore 档的当前 schema 版本。
pub const SCHEMA_VERSION: i32 = 2;

/// Returns (personal id, inherited). Caller owns the transaction.
pub fn ensure_personal(
    conn: &Connection,
    word: &str,
    definition: &str,
    phonetic: &str,
    example: &str,
    proficiency: &str,
    memory: &str,
) -> Result<(i64, bool), String> {
    let key = word_key(word);
    if key.is_empty() {
        return Err("单词不能为空".into());
    }
    valid_proficiency(proficiency)?;
    if let Some(id) = conn
        .query_row("SELECT id FROM user_vocab WHERE word_key=?1", [&key], |r| {
            r.get::<_, i64>(0)
        })
        .optional()
        .map_err(|e| e.to_string())?
    {
        conn.execute("UPDATE user_vocab SET definition=CASE WHEN ?1<>'' THEN ?1 ELSE definition END,phonetic=CASE WHEN ?2<>'' THEN ?2 ELSE phonetic END,example_sentence=CASE WHEN ?3<>'' THEN ?3 ELSE example_sentence END,updated_at=datetime('now','localtime') WHERE id=?4",params![definition,phonetic,example,id]).map_err(|e|e.to_string())?;
        return Ok((id, true));
    }
    let (_, srs) = parse_memory_tag(memory);
    let srs = initial_srs(proficiency, srs);
    let reviewed = serde_json::from_str::<serde_json::Value>(memory)
        .ok()
        .and_then(|v| v["last_reviewed_at"].as_i64())
        .unwrap_or(0);
    conn.execute("INSERT INTO user_vocab(word_key,word,definition,phonetic,example_sentence,proficiency,srs_state,last_reviewed_at,state_updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![key,word.trim(),definition,phonetic,example,proficiency,serde_json::to_string(&srs).map_err(|e|e.to_string())?,reviewed,now_secs()]).map_err(|e|e.to_string())?;
    Ok((conn.last_insert_rowid(), false))
}

pub fn set_proficiency(conn: &Connection, id: i64, proficiency: &str) -> Result<(), String> {
    valid_proficiency(proficiency)?;
    let encoded: String = conn
        .query_row("SELECT srs_state FROM user_vocab WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())?;
    let mut srs: SrsState = serde_json::from_str(&encoded).map_err(|e| e.to_string())?;
    if proficiency == "unknown" {
        srs.due = chrono::Local::now().date_naive().to_string();
    } else {
        srs = initial_srs(proficiency, srs);
    }
    conn.execute("UPDATE user_vocab SET proficiency=?1,srs_state=?2,state_updated_at=?3,updated_at=datetime('now','localtime') WHERE id=?4",params![proficiency,serde_json::to_string(&srs).map_err(|e|e.to_string())?,now_secs(),id]).map_err(|e|e.to_string())?;
    Ok(())
}

/// One shared read path for lists, highlights, review and every export format.
pub const SELECT_WORDS: &str = "SELECT w.id,w.vocab_book_id,w.word,w.definition,w.phonetic,w.example_sentence,w.novel_id,w.chapter_id,
 COALESCE(u.proficiency,w.proficiency),w.memory_tag,w.created_at,w.match_terms,w.user_vocab_id,u.srs_state,u.last_reviewed_at
 FROM vocab_word w LEFT JOIN user_vocab u ON u.id=w.user_vocab_id";

pub fn row_to_word(row: &rusqlite::Row) -> rusqlite::Result<crate::models::VocabWord> {
    let memory: String = row.get(9)?;
    let srs: Option<String> = row.get(13)?;
    let memory = if let Some(srs) = srs {
        let srs: SrsState = serde_json::from_str(&srs).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(13, rusqlite::types::Type::Text, Box::new(e))
        })?;
        serialize_memory_tag_reviewed(
            &parse_memory_tag(&memory).0,
            &srs,
            row.get::<_, Option<i64>>(14)?.unwrap_or(0) as u64,
        )
    } else {
        memory
    };
    Ok(crate::models::VocabWord {
        id: row.get(0)?,
        vocab_book_id: row.get(1)?,
        word: row.get(2)?,
        definition: row.get(3)?,
        phonetic: row.get(4)?,
        example_sentence: row.get(5)?,
        novel_id: row.get(6)?,
        chapter_id: row.get(7)?,
        proficiency: row.get(8)?,
        memory_tag: memory,
        created_at: row.get(10)?,
        match_terms: row.get(11)?,
        user_vocab_id: row.get(12)?,
    })
}

pub fn load_words(conn: &Connection, book: i64) -> Result<Vec<crate::models::VocabWord>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "{} WHERE w.vocab_book_id=?1 ORDER BY w.created_at DESC,w.id DESC",
            SELECT_WORDS
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([book], row_to_word)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}

pub struct NewWord<'a> {
    pub word: &'a str,
    pub definition: &'a str,
    pub phonetic: &'a str,
    pub example: &'a str,
    pub proficiency: &'a str,
    pub memory: &'a str,
    pub novel_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub match_terms: &'a str,
    pub source_keys: &'a str,
}
impl<'a> NewWord<'a> {
    pub fn simple(word: &'a str, proficiency: &'a str) -> Self {
        Self {
            word,
            proficiency,
            definition: "",
            phonetic: "",
            example: "",
            memory: "",
            novel_id: None,
            chapter_id: None,
            match_terms: "",
            source_keys: "[]",
        }
    }
}
pub struct InsertedWord {
    pub id: Option<i64>,
    pub inherited: bool,
    pub skipped: bool,
}

pub fn require_personal_book(conn: &Connection, book: i64) -> Result<(), String> {
    let preset: bool = conn
        .query_row(
            "SELECT is_preset FROM vocab_book WHERE id=?1",
            [book],
            |r| r.get(0),
        )
        .map_err(|_| "词汇本不存在".to_string())?;
    if preset {
        Err("预设词表只读，请先导入个人词汇本".into())
    } else {
        Ok(())
    }
}

/// Book metadata and personal state are inserted under the caller's transaction.
pub fn insert_word(conn: &Connection, book: i64, input: &NewWord) -> Result<InsertedWord, String> {
    require_personal_book(conn, book)?;
    let key = word_key(input.word);
    if key.is_empty() {
        return Err("单词不能为空".into());
    }
    if let Some(id) = conn
        .query_row(
            "SELECT id FROM vocab_word WHERE vocab_book_id=?1 AND word_key=?2",
            params![book, key],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
    {
        return Ok(InsertedWord {
            id: Some(id),
            inherited: false,
            skipped: true,
        });
    }
    let (uid, inherited) = ensure_personal(
        conn,
        input.word,
        input.definition,
        input.phonetic,
        input.example,
        input.proficiency,
        input.memory,
    )?;
    conn.execute("INSERT INTO vocab_word(vocab_book_id,word,word_key,user_vocab_id,definition,phonetic,example_sentence,novel_id,chapter_id,memory_tag,match_terms,source_keys) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![book,input.word.trim(),key,uid,input.definition,input.phonetic,input.example,input.novel_id,input.chapter_id,parse_memory_tag(input.memory).0,input.match_terms,input.source_keys]).map_err(|e|e.to_string())?;
    Ok(InsertedWord {
        id: Some(conn.last_insert_rowid()),
        inherited,
        skipped: false,
    })
}

pub fn edit_word(
    conn: &Connection,
    id: i64,
    word: &str,
    definition: &str,
    phonetic: &str,
    example: &str,
    memory: &str,
    proficiency: Option<&str>,
) -> Result<(), String> {
    let (book, old_key, old_uid): (i64, String, i64) = conn
        .query_row(
            "SELECT vocab_book_id,word_key,user_vocab_id FROM vocab_word WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "单词不存在或为只读预设".to_string())?;
    require_personal_book(conn, book)?;
    let key = word_key(word);
    let duplicate:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM vocab_word WHERE vocab_book_id=?1 AND word_key=?2 AND id<>?3)",params![book,key,id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if duplicate {
        return Err("该单词已在当前词汇本中".into());
    }
    let initial = if key == old_key {
        "unknown"
    } else {
        proficiency.unwrap_or("unknown")
    };
    let (uid, _) = ensure_personal(conn, word, definition, phonetic, example, initial, "")?;
    if key == old_key {
        if let Some(proficiency) = proficiency {
            set_proficiency(conn, old_uid, proficiency)?;
        }
    }
    conn.execute("UPDATE vocab_word SET word=?1,word_key=?2,user_vocab_id=?3,definition=?4,phonetic=?5,example_sentence=?6,memory_tag=?7,match_terms=CASE WHEN word_key<>?2 THEN '' ELSE match_terms END WHERE id=?8",params![word.trim(),key,uid,definition,phonetic,example,parse_memory_tag(memory).0,id]).map_err(|e|e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn database() -> (std::path::PathBuf, crate::db::DbState) {
        let dir = std::env::temp_dir().join(format!(
            "nw-shared-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let state = crate::db::init_db(&dir).unwrap();
        state
            .db
            .lock()
            .unwrap()
            .execute_batch("INSERT INTO vocab_book(name) VALUES ('A'),('B');")
            .unwrap();
        (dir, state)
    }
    #[test]
    fn shared_import_inherits_without_overwriting_and_skips_normalized_duplicate() {
        let (dir, state) = database();
        let db = state.db.lock().unwrap();
        let first = insert_word(&db, 1, &NewWord::simple("garden", "mastered")).unwrap();
        let uid = load_words(&db, 1).unwrap()[0].user_vocab_id.unwrap();
        let before: String = db
            .query_row("SELECT srs_state FROM user_vocab WHERE id=?1", [uid], |r| {
                r.get(0)
            })
            .unwrap();
        let second = insert_word(&db, 2, &NewWord::simple(" Garden ", "unknown")).unwrap();
        assert!(!first.inherited);
        assert!(second.inherited);
        assert!(!second.skipped);
        assert!(
            insert_word(&db, 2, &NewWord::simple("GARDEN", "unknown"))
                .unwrap()
                .skipped
        );
        assert_eq!(load_words(&db, 2).unwrap()[0].proficiency, "mastered");
        assert_eq!(
            db.query_row("SELECT srs_state FROM user_vocab WHERE id=?1", [uid], |r| r
                .get::<_, String>(0))
                .unwrap(),
            before
        );
        set_proficiency(&db, uid, "unknown").unwrap();
        assert_eq!(load_words(&db, 1).unwrap()[0].proficiency, "unknown");
        assert_eq!(load_words(&db, 2).unwrap()[0].proficiency, "unknown");
        drop(db);
        drop(state);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn metadata_edit_ignores_stale_srs_and_rename_does_not_transfer_learning() {
        let (dir, state) = database();
        let db = state.db.lock().unwrap();
        let inserted = insert_word(&db, 1, &NewWord::simple("garden", "mastered")).unwrap();
        let id = inserted.id.unwrap();
        let old = load_words(&db, 1).unwrap().remove(0);
        edit_word(&db, id, "GARDEN", "花园", "", "", "custom", None).unwrap();
        let updated = load_words(&db, 1).unwrap().remove(0);
        assert_eq!(
            parse_memory_tag(&old.memory_tag).1.due,
            parse_memory_tag(&updated.memory_tag).1.due
        );
        assert_eq!(parse_memory_tag(&updated.memory_tag).0, "custom");
        edit_word(&db, id, "running", "跑步", "", "", "", None).unwrap();
        let renamed = load_words(&db, 1).unwrap().remove(0);
        assert_eq!(renamed.proficiency, "unknown");
        assert_ne!(old.user_vocab_id, renamed.user_vocab_id);
        assert_eq!(word_key("  one’s   own "), word_key("ONE'S OWN"));
        assert_ne!(word_key("run"), word_key("running"));
        drop(db);
        drop(state);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn renamed_new_word_accepts_explicit_initial_state_but_existing_target_inherits() {
        let (dir, state) = database();
        let db = state.db.lock().unwrap();
        let id = insert_word(&db, 1, &NewWord::simple("garden", "unknown"))
            .unwrap()
            .id
            .unwrap();
        edit_word(&db, id, "orchard", "果园", "", "", "", Some("mastered")).unwrap();
        assert_eq!(load_words(&db, 1).unwrap()[0].proficiency, "mastered");
        insert_word(&db, 2, &NewWord::simple("field", "familiar")).unwrap();
        edit_word(&db, id, "field", "田地", "", "", "", Some("mastered")).unwrap();
        assert_eq!(load_words(&db, 1).unwrap()[0].proficiency, "familiar");
        drop(db);
        drop(state);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
