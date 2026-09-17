use crate::db::DbState;
use crate::utils::srs::{serialize_memory_tag_reviewed, SrsState};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBook {
    pub id: i64,
    pub name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserVocabEntry {
    pub id: i64,
    pub word: String,
    pub definition: String,
    pub phonetic: String,
    pub example_sentence: String,
    pub proficiency: String,
    pub memory_tag: String,
    pub last_reviewed_at: i64,
    pub active: bool,
    pub source_books: Vec<SourceBook>,
}
#[derive(Serialize)]
pub struct UserVocabPage {
    pub total: i64,
    pub words: Vec<UserVocabEntry>,
}

/// 逐词阅读：单词当前状态快照（key 为归一化词形，供前端着色匹配）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordTapState {
    pub key: String,
    pub word: String,
    pub proficiency: String,
}

/// 逐词阅读：批量标记的内部实现（调用方持有事务/连接）。
/// 已收录词的标记同时写入 review_log（阅读联动复习）：
/// 不认识=again / 模糊=good / 认识=easy；ignore 不记复习。
pub(crate) fn mark_word_tap(
    db: &mut Connection,
    words: &[String],
    proficiency: &str,
) -> Result<u32, String> {
    crate::user_vocab::valid_proficiency(proficiency)?;
    let rating = match proficiency {
        "unknown" => Some("again"),
        "familiar" => Some("good"),
        "mastered" => Some("easy"),
        _ => None,
    };
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let mut count: u32 = 0;
    for raw in words {
        let word = raw.trim();
        if word.is_empty() {
            continue;
        }
        let key = crate::user_vocab::word_key(word);
        if key.is_empty() {
            continue;
        }
        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM user_vocab WHERE word_key=?1",
                [&key],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match existing {
            Some(id) => crate::user_vocab::set_proficiency_with_log(&tx, id, proficiency, rating)?,
            None => {
                crate::user_vocab::ensure_personal(&tx, word, "", "", "", proficiency, "")?;
            }
        }
        count += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

/// 逐词阅读：批量查询的内部实现。
pub(crate) fn lookup_word_tap(
    db: &Connection,
    words: &[String],
) -> Result<Vec<WordTapState>, String> {
    let mut out: Vec<WordTapState> = Vec::new();
    for chunk in words.chunks(300) {
        let keys: Vec<String> = chunk
            .iter()
            .map(|w| crate::user_vocab::word_key(w))
            .filter(|k| !k.is_empty())
            .collect();
        if keys.is_empty() {
            continue;
        }
        let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT word_key, word, proficiency FROM user_vocab WHERE word_key IN ({})",
            placeholders
        );
        let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(keys.iter()), |r| {
                Ok(WordTapState {
                    key: r.get(0)?,
                    word: r.get(1)?,
                    proficiency: r.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
    }
    Ok(out)
}

/// 逐词阅读：返回全部已保存短语（word_key 含空格的 user_vocab 条目）。
/// 短语量远小于单词量，一次全量返回供渲染端做相邻词合并。
#[tauri::command]
pub fn lookup_word_tap_phrases(state: State<DbState>) -> Result<Vec<WordTapState>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT word_key, word, proficiency FROM user_vocab
             WHERE word_key LIKE '% %' ORDER BY word_key",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(WordTapState {
                key: r.get(0)?,
                word: r.get(1)?,
                proficiency: r.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// 逐词阅读：批量标记单词熟练度（写入个人总词汇库，未收录的直接建条目，
/// 不产生词汇本归属）。返回成功标记的个数。
#[tauri::command]
pub fn mark_word_tap_proficiency(
    state: State<DbState>,
    words: Vec<String>,
    proficiency: String,
) -> Result<u32, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    mark_word_tap(&mut db, &words, &proficiency)
}

/// 逐词阅读：批量查询单词状态（按归一化词形匹配，未收录的词不返回）。
#[tauri::command]
pub fn lookup_word_tap_states(
    state: State<DbState>,
    words: Vec<String>,
) -> Result<Vec<WordTapState>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    lookup_word_tap(&db, &words)
}

fn entry(db: &Connection, id: i64) -> Result<UserVocabEntry, String> {
    let mut result=db.query_row("SELECT id,word,definition,phonetic,example_sentence,proficiency,srs_state,last_reviewed_at FROM user_vocab WHERE id=?1",[id],|r| {
        let srs:String=r.get(6)?;let srs:SrsState=serde_json::from_str(&srs).map_err(|e|rusqlite::Error::FromSqlConversionFailure(6,rusqlite::types::Type::Text,Box::new(e)))?;
        let reviewed:i64=r.get(7)?;
        Ok(UserVocabEntry{id:r.get(0)?,word:r.get(1)?,definition:r.get(2)?,phonetic:r.get(3)?,example_sentence:r.get(4)?,proficiency:r.get(5)?,memory_tag:serialize_memory_tag_reviewed("",&srs,reviewed as u64),last_reviewed_at:reviewed,active:false,source_books:vec![]})
    }).map_err(|e|e.to_string())?;
    let mut stmt=db.prepare("SELECT DISTINCT b.id,b.name FROM vocab_word w JOIN vocab_book b ON b.id=w.vocab_book_id WHERE w.user_vocab_id=?1 AND b.is_preset=0 ORDER BY b.name,b.id").map_err(|e|e.to_string())?;
    result.source_books = stmt
        .query_map([id], |r| {
            Ok(SourceBook {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    result.active = !result.source_books.is_empty();
    Ok(result)
}
pub(crate) fn registry_page(
    db: &Connection,
    query: Option<&str>,
    proficiencies: &[String],
    offset: i64,
    limit: i64,
) -> Result<UserVocabPage, String> {
    let mut filters = vec!["1=1".to_string()];
    let mut args: Vec<rusqlite::types::Value> = vec![];
    let query = query.unwrap_or("").trim();
    if !query.is_empty() {
        filters.push("(word_key LIKE ? OR definition LIKE ? OR phonetic LIKE ?)".into());
        args.push(format!("%{}%", crate::user_vocab::word_key(query)).into());
        args.push(format!("%{}%", query).into());
        args.push(format!("%{}%", query).into());
    }
    let profs: Vec<_> = proficiencies
        .iter()
        .filter(|p| crate::user_vocab::valid_proficiency(p).is_ok())
        .collect();
    if !profs.is_empty() {
        filters.push(format!(
            "proficiency IN ({})",
            vec!["?"; profs.len()].join(",")
        ));
        args.extend(profs.into_iter().map(|p| p.clone().into()));
    }
    let filter = filters.join(" AND ");
    let total = db
        .query_row(
            &format!("SELECT COUNT(*) FROM user_vocab WHERE {}", filter),
            rusqlite::params_from_iter(&args),
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    args.push(limit.clamp(1, 200).into());
    args.push(offset.max(0).into());
    let mut stmt = db
        .prepare(&format!(
            "SELECT id FROM user_vocab WHERE {} ORDER BY updated_at DESC,id DESC LIMIT ? OFFSET ?",
            filter
        ))
        .map_err(|e| e.to_string())?;
    let ids = stmt
        .query_map(rusqlite::params_from_iter(&args), |r| r.get::<_, i64>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(UserVocabPage {
        total,
        words: ids
            .into_iter()
            .map(|id| entry(db, id))
            .collect::<Result<_, _>>()?,
    })
}
#[tauri::command]
pub fn get_user_vocab_page(
    state: State<DbState>,
    query: Option<String>,
    proficiencies: Option<Vec<String>>,
    offset: i64,
    limit: i64,
) -> Result<UserVocabPage, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    registry_page(
        &db,
        query.as_deref(),
        &proficiencies.unwrap_or_default(),
        offset,
        limit,
    )
}
#[tauri::command]
pub fn lookup_user_vocab(
    state: State<DbState>,
    word: String,
) -> Result<Option<UserVocabEntry>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = db
        .query_row(
            "SELECT id FROM user_vocab WHERE word_key=?1",
            [crate::user_vocab::word_key(&word)],
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    id.map(|id| entry(&db, id)).transpose()
}
#[tauri::command]
pub fn set_user_vocab_proficiency(
    state: State<DbState>,
    ids: Vec<i64>,
    proficiency: String,
) -> Result<u32, String> {
    crate::user_vocab::valid_proficiency(&proficiency)?;
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let ids: std::collections::BTreeSet<_> = ids.into_iter().collect();
    for id in &ids {
        crate::user_vocab::set_proficiency(&tx, *id, &proficiency)?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(ids.len() as u32)
}

#[cfg(test)]
mod tests {
    #[test]
    fn registry_page_filters_shared_state_and_keeps_paused_entries() {
        let dir = std::env::temp_dir().join(format!(
            "nw-registry-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let state = crate::db::init_db(&dir).unwrap();
        let db = state.db.lock().unwrap();
        db.execute("INSERT INTO vocab_book(name) VALUES ('A')", [])
            .unwrap();
        crate::user_vocab::insert_word(
            &db,
            1,
            &crate::user_vocab::NewWord::simple("garden", "familiar"),
        )
        .unwrap();
        crate::user_vocab::insert_word(
            &db,
            1,
            &crate::user_vocab::NewWord::simple("run", "unknown"),
        )
        .unwrap();
        let page =
            super::registry_page(&db, Some("GARDEN"), &["familiar".to_string()], 0, 50).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.words[0].source_books.len(), 1);
        assert!(page.words[0].active);
        db.execute("DELETE FROM vocab_book", []).unwrap();
        let page = super::registry_page(&db, None, &[], 0, 1).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.words.len(), 1);
        assert!(!page.words[0].active);
        drop(db);
        drop(state);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn word_tap_mark_creates_updates_and_ignores_review() {
        let dir = std::env::temp_dir().join(format!(
            "nw-wordtap-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let state = crate::db::init_db(&dir).unwrap();
        let mut db = state.db.lock().unwrap();

        // 建词汇本并收两个词，保证复习队列有词
        db.execute("INSERT INTO vocab_book(name) VALUES ('A')", []).unwrap();
        crate::user_vocab::insert_word(
            &db,
            1,
            &crate::user_vocab::NewWord::simple("garden", "unknown"),
        )
        .unwrap();
        crate::user_vocab::insert_word(
            &db,
            1,
            &crate::user_vocab::NewWord::simple("run", "unknown"),
        )
        .unwrap();
        assert_eq!(crate::commands::review::due_words(&db, None).unwrap().len(), 2);

        // 未收录词直接建个人条目；已收录词（大小写变体）按归一化词形更新
        let n = super::mark_word_tap(
            &mut db,
            &["serendipity".into(), "  Run ".into()],
            "unknown",
        )
        .unwrap();
        assert_eq!(n, 2);
        // 再次以不同词形标记同一词 → 归一化命中，不新建
        let n = super::mark_word_tap(&mut db, &["runs".into()], "mastered").unwrap();
        assert_eq!(n, 1);
        let states = super::lookup_word_tap(
            &db,
            &[
                "Serendipity".to_string(),
                "runs".to_string(),
                "ghost".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(states.len(), 2);
        let runs = states.iter().find(|s| s.key == "runs").unwrap();
        assert_eq!(runs.proficiency, "mastered");

        // ignore 档：写入成功且退出复习队列
        super::mark_word_tap(&mut db, &["Garden".into()], "ignore").unwrap();
        let due = crate::commands::review::due_words(&db, None).unwrap();
        assert!(due.iter().all(|w| crate::user_vocab::word_key(&w.word) != "garden"));
        assert!(due.iter().any(|w| crate::user_vocab::word_key(&w.word) == "run"));

        // 阅读联动复习：已收录词标"认识" → review_log 写入 easy
        super::mark_word_tap(&mut db, &["RUN".into()], "mastered").unwrap();
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM review_log WHERE rating='easy'", [], |r| r.get(0))
            .unwrap();
        assert!(count >= 1, "mastered 标记应写入 easy 复习日志");
        let again_count: i64 = db
            .query_row("SELECT COUNT(*) FROM review_log WHERE rating='again'", [], |r| r.get(0))
            .unwrap();
        assert!(again_count >= 1, "unknown 标记应写入 again 复习日志");
        // ignore 不写复习日志
        let ignore_count: i64 = db
            .query_row("SELECT COUNT(*) FROM review_log WHERE proficiency='ignore'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ignore_count, 0);
        drop(db);
        drop(state);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn migration_v2_rebuilds_user_vocab_check() {
        let dir = std::env::temp_dir().join(format!(
            "nw-v2mig-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("novel_words.db");
        let mut conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").unwrap();
        conn.execute_batch(
            "CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL DEFAULT '');
             CREATE TABLE vocab_book (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, is_preset INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE vocab_word (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vocab_book_id INTEGER NOT NULL,
                word TEXT NOT NULL,
                definition TEXT NOT NULL DEFAULT '',
                phonetic TEXT NOT NULL DEFAULT '',
                example_sentence TEXT NOT NULL DEFAULT '',
                novel_id INTEGER,
                chapter_id INTEGER,
                proficiency TEXT NOT NULL DEFAULT 'unknown' CHECK(proficiency IN ('unknown','familiar','mastered')),
                memory_tag TEXT NOT NULL DEFAULT '',
                match_terms TEXT NOT NULL DEFAULT '',
                word_key TEXT NOT NULL DEFAULT '',
                user_vocab_id INTEGER,
                source_keys TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
             );
             CREATE TABLE user_vocab (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word_key TEXT NOT NULL UNIQUE,
                word TEXT NOT NULL,
                definition TEXT NOT NULL DEFAULT '',
                phonetic TEXT NOT NULL DEFAULT '',
                example_sentence TEXT NOT NULL DEFAULT '',
                proficiency TEXT NOT NULL DEFAULT 'unknown' CHECK(proficiency IN ('unknown','familiar','mastered')),
                srs_state TEXT NOT NULL DEFAULT '{}',
                last_reviewed_at INTEGER NOT NULL DEFAULT 0,
                state_updated_at INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
             );
             CREATE INDEX idx_user_vocab_proficiency ON user_vocab(proficiency);
             CREATE TABLE review_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vocab_word_id INTEGER,
                vocab_book_id INTEGER,
                rating TEXT NOT NULL DEFAULT 'legacy',
                reviewed_at INTEGER NOT NULL,
                due_before TEXT NOT NULL DEFAULT '',
                due_after TEXT NOT NULL DEFAULT '',
                proficiency TEXT NOT NULL DEFAULT ''
             );
             INSERT INTO vocab_book(name) VALUES ('书A');
             INSERT INTO vocab_word(vocab_book_id,word,word_key,proficiency) VALUES (1,'garden','garden','familiar');
             INSERT INTO user_vocab(word_key,word,proficiency) VALUES ('garden','garden','familiar');
             INSERT INTO app_settings(key,value) VALUES ('user_vocab_schema','1');",
        )
        .unwrap();

        assert!(super::super::super::user_vocab::migration_needed(&conn));
        super::super::super::user_vocab::migrate(&mut conn).unwrap();
        assert!(!super::super::super::user_vocab::migration_needed(&conn));

        // 旧数据保留 + 新 CHECK 接受 ignore
        let kept: String = conn
            .query_row("SELECT proficiency FROM user_vocab WHERE word_key='garden'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(kept, "familiar");
        conn.execute(
            "INSERT INTO user_vocab(word_key,word,proficiency) VALUES ('apple','apple','ignore')",
            [],
        )
        .unwrap();
        let book_words = crate::user_vocab::load_words(&conn, 1).unwrap();
        assert_eq!(book_words.len(), 1);
        assert_eq!(book_words[0].proficiency, "familiar");
        drop(conn);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
