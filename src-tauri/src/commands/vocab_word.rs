use crate::db::DbState;
use crate::models::{HighlightWord, VocabWord, VocabWordPage};
pub(crate) use crate::user_vocab::row_to_word as row_to_vocab_word;
use crate::user_vocab::{self, NewWord, SELECT_WORDS};
use rusqlite::params;
use tauri::{AppHandle, State};
use tauri_plugin_fs::FsExt;

#[tauri::command]
pub fn create_vocab_word(
    state: State<DbState>,
    vocab_book_id: i64,
    word: String,
    definition: String,
    phonetic: String,
    example_sentence: String,
    novel_id: Option<i64>,
    chapter_id: Option<i64>,
    proficiency: String,
    memory_tag: String,
) -> Result<VocabWord, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let result = user_vocab::insert_word(
        &tx,
        vocab_book_id,
        &NewWord {
            word: &word,
            definition: &definition,
            phonetic: &phonetic,
            example: &example_sentence,
            proficiency: &proficiency,
            memory: &memory_tag,
            novel_id,
            chapter_id,
            match_terms: "",
            source_keys: "[]",
        },
    )?;
    if result.skipped {
        return Err(format!("单词「{}」已存在", word.trim()));
    }
    let word = get_vocab_word_by_id(&tx, result.id.unwrap())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(word)
}
#[tauri::command]
pub fn get_vocab_words(
    state: State<DbState>,
    vocab_book_id: i64,
) -> Result<Vec<VocabWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    user_vocab::load_words(&db, vocab_book_id)
}
#[tauri::command]
pub fn get_vocab_words_page(
    state: State<DbState>,
    vocab_book_id: i64,
    query: Option<String>,
    proficiencies: Option<Vec<String>>,
    offset: i64,
    limit: i64,
) -> Result<VocabWordPage, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut wheres = vec!["w.vocab_book_id=?".to_string()];
    let mut args: Vec<rusqlite::types::Value> = vec![vocab_book_id.into()];
    let q = query.unwrap_or_default().trim().to_owned();
    if !q.is_empty() {
        wheres.push("(w.word LIKE ? OR w.definition LIKE ? OR w.phonetic LIKE ?)".into());
        for _ in 0..3 {
            args.push(format!("%{}%", q).into());
        }
    }
    let profs: Vec<_> = proficiencies
        .unwrap_or_default()
        .into_iter()
        .filter(|p| user_vocab::valid_proficiency(p).is_ok())
        .collect();
    if !profs.is_empty() {
        wheres.push(format!(
            "COALESCE(u.proficiency,w.proficiency) IN ({})",
            vec!["?"; profs.len()].join(",")
        ));
        args.extend(profs.into_iter().map(Into::into));
    }
    let filter = wheres.join(" AND ");
    let total=db.query_row(&format!("SELECT COUNT(*) FROM vocab_word w LEFT JOIN user_vocab u ON u.id=w.user_vocab_id WHERE {}",filter),rusqlite::params_from_iter(&args),|r|r.get(0)).map_err(|e|e.to_string())?;
    args.push(limit.clamp(1, 500).into());
    args.push(offset.max(0).into());
    let mut stmt = db
        .prepare(&format!(
            "{} WHERE {} ORDER BY w.created_at DESC,w.id DESC LIMIT ? OFFSET ?",
            SELECT_WORDS, filter
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(&args), row_to_vocab_word)
        .map_err(|e| e.to_string())?;
    Ok(VocabWordPage {
        total,
        words: rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?,
    })
}
#[tauri::command]
pub fn update_vocab_word(
    state: State<DbState>,
    id: i64,
    word: String,
    definition: String,
    phonetic: String,
    example_sentence: String,
    proficiency: Option<String>,
    memory_tag: String,
) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    user_vocab::edit_word(
        &tx,
        id,
        &word,
        &definition,
        &phonetic,
        &example_sentence,
        &memory_tag,
        proficiency.as_deref(),
    )?;
    tx.commit().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn delete_vocab_word(state: State<DbState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let book: i64 = db
        .query_row(
            "SELECT vocab_book_id FROM vocab_word WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    user_vocab::require_personal_book(&db, book)?;
    db.execute("DELETE FROM vocab_word WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub fn delete_vocab_words(state: State<DbState>, ids: Vec<i64>) -> Result<u32, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let mut count = 0;
    for id in ids {
        let book: i64 = tx
            .query_row(
                "SELECT vocab_book_id FROM vocab_word WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        user_vocab::require_personal_book(&tx, book)?;
        count += tx
            .execute("DELETE FROM vocab_word WHERE id=?1", [id])
            .map_err(|e| e.to_string())? as u32;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}
#[tauri::command]
pub fn search_vocab_words(
    state: State<DbState>,
    vocab_book_id: i64,
    query: String,
) -> Result<Vec<VocabWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(&format!(
            "{} WHERE w.vocab_book_id=?1 AND w.word LIKE ?2 ORDER BY w.created_at DESC,w.id DESC",
            SELECT_WORDS
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            params![vocab_book_id, format!("%{}%", query)],
            row_to_vocab_word,
        )
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}
pub(crate) fn get_vocab_word_by_id(
    db: &rusqlite::Connection,
    id: i64,
) -> Result<VocabWord, String> {
    db.query_row(
        &format!("{} WHERE w.id=?1", SELECT_WORDS),
        [id],
        row_to_vocab_word,
    )
    .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn get_highlight_words(
    state: State<DbState>,
    vocab_book_id: i64,
) -> Result<Vec<HighlightWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    Ok(user_vocab::load_words(&db, vocab_book_id)?
        .into_iter()
        .map(|w| HighlightWord {
            word: w.word,
            definition: w.definition,
            phonetic: w.phonetic,
            example_sentence: w.example_sentence,
            novel_id: w.novel_id,
            proficiency: w.proficiency,
            match_terms: w.match_terms,
        })
        .collect())
}
#[tauri::command]
pub fn export_vocab_words_csv(
    app: AppHandle,
    state: State<DbState>,
    vocab_book_id: i64,
    file_path: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let words = user_vocab::load_words(&db, vocab_book_id)?;
    drop(db);
    let mut options = tauri_plugin_fs::OpenOptions::new();
    options.read(false).write(true).create(true).truncate(true);
    let file = app
        .fs()
        .open(
            file_path
                .parse::<tauri_plugin_fs::FilePath>()
                .map_err(|e| e.to_string())?,
            options,
        )
        .map_err(|e| e.to_string())?;
    let mut writer = csv::Writer::from_writer(file);
    writer
        .write_record([
            "word",
            "definition",
            "phonetic",
            "example_sentence",
            "proficiency",
            "memory_tag",
        ])
        .map_err(|e| e.to_string())?;
    for w in words {
        writer
            .write_record([
                w.word,
                w.definition,
                w.phonetic,
                w.example_sentence,
                w.proficiency,
                w.memory_tag,
            ])
            .map_err(|e| e.to_string())?;
    }
    writer.flush().map_err(|e| e.to_string())
}
#[derive(Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: u32,
    pub inherited: u32,
    pub skipped: u32,
    pub new_words: u32,
}
impl ImportResult {
    pub fn record(&mut self, result: &user_vocab::InsertedWord) {
        if result.skipped {
            self.skipped += 1;
        } else {
            self.imported += 1;
            if result.inherited {
                self.inherited += 1;
            } else {
                self.new_words += 1;
            }
        }
    }
}
pub fn import_csv(
    db: &mut rusqlite::Connection,
    book: i64,
    bytes: &[u8],
) -> Result<ImportResult, String> {
    let mut reader = csv::Reader::from_reader(bytes);
    let tx = db.transaction().map_err(|e| e.to_string())?;
    user_vocab::require_personal_book(&tx, book)?;
    let mut result = ImportResult::default();
    for record in reader.records() {
        let record = record.map_err(|e| format!("CSV 解析失败: {}", e))?;
        let word = record.get(0).unwrap_or("").trim();
        if word.is_empty() {
            result.skipped += 1;
            continue;
        }
        let raw = record.get(4).unwrap_or("");
        let proficiency = if user_vocab::valid_proficiency(raw).is_ok() {
            raw
        } else {
            "unknown"
        };
        let inserted = user_vocab::insert_word(
            &tx,
            book,
            &NewWord {
                word,
                definition: record.get(1).unwrap_or(""),
                phonetic: record.get(2).unwrap_or(""),
                example: record.get(3).unwrap_or(""),
                memory: record.get(5).unwrap_or(""),
                ..NewWord::simple(word, proficiency)
            },
        )?;
        result.record(&inserted);
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(result)
}
#[tauri::command]
pub fn import_vocab_words_csv(
    app: AppHandle,
    state: State<DbState>,
    vocab_book_id: i64,
    file_path: String,
) -> Result<ImportResult, String> {
    let bytes = app
        .fs()
        .read(
            file_path
                .parse::<tauri_plugin_fs::FilePath>()
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    import_csv(&mut db, vocab_book_id, &bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn csv_inherits_state_normalizes_variants_and_rolls_back_malformed_file() {
        let dir = std::env::temp_dir().join(format!(
            "nw-csv-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let state = crate::db::init_db(&dir).unwrap();
        let mut db = state.db.lock().unwrap();
        db.execute_batch("INSERT INTO vocab_book(name) VALUES ('A'),('B');")
            .unwrap();
        crate::user_vocab::insert_word(
            &db,
            1,
            &crate::user_vocab::NewWord::simple("don't stop", "mastered"),
        )
        .unwrap();
        let bytes="word,definition,phonetic,example_sentence,proficiency,memory_tag\n  Don’t   Stop  ,继续,,,unknown,\nDON'T STOP,重复,,,familiar,\nrun,跑,,,familiar,\nrunning,正在跑,,,invalid,\n".as_bytes();
        let result = super::import_csv(&mut db, 2, bytes).unwrap();
        assert_eq!(
            (result.new_words, result.inherited, result.skipped),
            (2, 1, 1)
        );
        let words = crate::user_vocab::load_words(&db, 2).unwrap();
        assert_eq!(
            words
                .iter()
                .find(|w| crate::user_vocab::word_key(&w.word) == "don't stop")
                .unwrap()
                .proficiency,
            "mastered"
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM user_vocab", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            3
        );
        let malformed = b"word,definition\nnewword,new definition\nbroken\n";
        assert!(super::import_csv(&mut db, 2, malformed).is_err());
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM user_vocab", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            3
        );
        drop(db);
        drop(state);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
