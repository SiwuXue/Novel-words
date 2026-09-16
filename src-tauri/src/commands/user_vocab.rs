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
}
