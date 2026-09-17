use crate::db::DbState;
use crate::models::VocabWord;
use crate::user_vocab::{self, row_to_word, SELECT_WORDS};
use crate::utils::srs::{apply_rating, is_due, parse_memory_tag, serialize_memory_tag_reviewed};
use rusqlite::{params, Connection};
use serde::Serialize;
use tauri::State;

fn local_today_start_secs() -> i64 {
    use chrono::{Local, TimeZone};
    Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|v| Local.from_local_datetime(&v).earliest())
        .map(|v| v.timestamp())
        .unwrap_or_else(user_vocab::now_secs)
}
pub(crate) fn due_words(db: &Connection, book: Option<i64>) -> Result<Vec<VocabWord>, String> {
    let sql=format!("{} WHERE w.user_vocab_id IS NOT NULL AND w.id=(SELECT x.id FROM vocab_word x WHERE x.user_vocab_id=u.id AND (?1 IS NULL OR x.vocab_book_id=?1) ORDER BY x.created_at DESC,x.id DESC LIMIT 1)",SELECT_WORDS);
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let mut words = stmt
        .query_map([book], row_to_word)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    // ignore 档（逐词阅读的"忽略"标记）永不进入复习队列
    words.retain(|w| {
        w.proficiency != "ignore" && is_due(&parse_memory_tag(&w.memory_tag).1)
    });
    words.sort_by(|a, b| {
        let a_srs = parse_memory_tag(&a.memory_tag).1;
        let b_srs = parse_memory_tag(&b.memory_tag).1;
        a_srs
            .due
            .is_empty()
            .cmp(&b_srs.due.is_empty())
            .reverse()
            .then_with(|| a_srs.due.cmp(&b_srs.due))
            .then_with(|| b.id.cmp(&a.id))
    });
    Ok(words)
}
#[tauri::command]
pub fn get_due_words(state: State<DbState>, vocab_book_id: i64) -> Result<Vec<VocabWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    due_words(&db, Some(vocab_book_id))
}
#[tauri::command]
pub fn get_all_due_words(state: State<DbState>) -> Result<Vec<VocabWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    due_words(&db, None)
}
#[tauri::command]
pub fn get_due_words_count(state: State<DbState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    Ok(due_words(&db, None)?.len() as i64)
}

pub(crate) fn record_review(
    db: &mut Connection,
    id: i64,
    rating: &str,
) -> Result<VocabWord, String> {
    if !matches!(rating, "again" | "good" | "easy") {
        return Err("无效的复习评分".into());
    }
    let word = crate::commands::vocab_word::get_vocab_word_by_id(db, id)?;
    let uid = word.user_vocab_id.ok_or("请先将预设词表导入个人词汇本")?;
    let (tag, mut srs) = parse_memory_tag(&word.memory_tag);
    let due_before = srs.due.clone();
    let proficiency = apply_rating(&mut srs, rating);
    let reviewed = user_vocab::now_secs();
    let tx = db.transaction().map_err(|e| e.to_string())?;
    tx.execute("UPDATE user_vocab SET proficiency=?1,srs_state=?2,last_reviewed_at=?3,state_updated_at=?3,updated_at=datetime('now','localtime') WHERE id=?4",params![proficiency,serde_json::to_string(&srs).map_err(|e|e.to_string())?,reviewed,uid]).map_err(|e|e.to_string())?;
    tx.execute("INSERT INTO review_log(user_vocab_id,vocab_word_id,vocab_book_id,word_snapshot,book_name_snapshot,rating,reviewed_at,due_before,due_after,proficiency) VALUES (?1,?2,?3,?4,(SELECT name FROM vocab_book WHERE id=?3),?5,?6,?7,?8,?9)",params![uid,id,word.vocab_book_id,word.word,rating,reviewed,due_before,srs.due,proficiency]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(VocabWord {
        proficiency,
        memory_tag: serialize_memory_tag_reviewed(&tag, &srs, reviewed as u64),
        ..word
    })
}
#[tauri::command]
pub fn review_vocab_word(
    state: State<DbState>,
    id: i64,
    rating: String,
) -> Result<VocabWord, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    record_review(&mut db, id, &rating)
}
#[derive(Debug, Clone, Serialize)]
pub struct ReviewProgress {
    pub vocab_book_id: Option<i64>,
    pub due_total: i64,
    pub reviewed_today: i64,
    pub goal: i64,
}
#[tauri::command]
pub fn get_review_progress(
    state: State<DbState>,
    vocab_book_id: Option<i64>,
) -> Result<ReviewProgress, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let reviewed_today=db.query_row("SELECT COUNT(*) FROM review_log r WHERE r.user_vocab_id IS NOT NULL AND r.reviewed_at>=?1 AND (?2 IS NULL OR EXISTS(SELECT 1 FROM vocab_word w WHERE w.user_vocab_id=r.user_vocab_id AND w.vocab_book_id=?2))",params![local_today_start_secs(),vocab_book_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    let goal = db
        .query_row(
            "SELECT value FROM app_settings WHERE key='daily_review_goal'",
            [],
            |r| r.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(20)
        .max(1);
    Ok(ReviewProgress {
        vocab_book_id,
        due_total: due_words(&db, vocab_book_id)?.len() as i64,
        reviewed_today,
        goal,
    })
}
#[derive(Debug, Clone, Serialize)]
pub struct DailyReviewCount {
    pub date: String,
    pub count: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct LearningStats {
    pub total_words: i64,
    pub total_books: i64,
    pub total_novels: i64,
    pub by_proficiency: std::collections::BTreeMap<String, i64>,
    pub reviewed_today: i64,
    pub total_reviews: i64,
    pub reviews_last_7_days: Vec<DailyReviewCount>,
}
pub(crate) fn learning_stats(db: &Connection) -> Result<LearningStats, String> {
    let count = |sql: &str| {
        db.query_row(sql, [], |r| r.get::<_, i64>(0))
            .map_err(|e| e.to_string())
    };
    let mut by_proficiency = std::collections::BTreeMap::new();
    for proficiency in ["unknown", "familiar", "mastered"] {
        let n = db
            .query_row(
                "SELECT COUNT(*) FROM user_vocab WHERE proficiency=?1",
                [proficiency],
                |r| r.get::<_, i64>(0),
            )
            .map_err(|e| e.to_string())?;
        by_proficiency.insert(proficiency.to_owned(), n);
    }
    let today = chrono::Local::now().date_naive();
    let mut buckets = std::collections::BTreeMap::new();
    let mut stmt=db.prepare("SELECT reviewed_at FROM review_log WHERE user_vocab_id IS NOT NULL AND reviewed_at>=?1").map_err(|e|e.to_string())?;
    let times = stmt
        .query_map([local_today_start_secs() - 7 * 86400], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(|e| e.to_string())?;
    use chrono::TimeZone;
    for timestamp in times {
        if let Some(date) = chrono::Local
            .timestamp_opt(timestamp.map_err(|e| e.to_string())?, 0)
            .single()
        {
            *buckets.entry(date.date_naive().to_string()).or_insert(0) += 1;
        }
    }
    let reviews_last_7_days = (0..7)
        .rev()
        .map(|i| {
            let date = (today - chrono::Duration::days(i)).to_string();
            DailyReviewCount {
                count: *buckets.get(&date).unwrap_or(&0),
                date,
            }
        })
        .collect();
    let reviewed_today = db
        .query_row(
            "SELECT COUNT(*) FROM review_log WHERE user_vocab_id IS NOT NULL AND reviewed_at>=?1",
            [local_today_start_secs()],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(LearningStats {
        total_words: count("SELECT COUNT(*) FROM user_vocab")?,
        total_books: count("SELECT COUNT(*) FROM vocab_book WHERE is_preset=0")?,
        total_novels: count("SELECT COUNT(*) FROM novel")?,
        by_proficiency,
        reviewed_today,
        total_reviews: count("SELECT COUNT(*) FROM review_log WHERE user_vocab_id IS NOT NULL")?,
        reviews_last_7_days,
    })
}
#[tauri::command]
pub fn get_learning_stats(state: State<DbState>) -> Result<LearningStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    learning_stats(&db)
}

#[cfg(test)]
mod tests {
    #[test]
    fn shared_review_deduplicates_and_pauses_orphaned_words() {
        let dir = std::env::temp_dir().join(format!(
            "nw-review-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let state = crate::db::init_db(&dir).unwrap();
        let mut db = state.db.lock().unwrap();
        db.execute_batch("INSERT INTO vocab_book(name) VALUES ('A'),('B');")
            .unwrap();
        let a = crate::user_vocab::insert_word(
            &db,
            1,
            &crate::user_vocab::NewWord::simple("garden", "unknown"),
        )
        .unwrap()
        .id
        .unwrap();
        crate::user_vocab::insert_word(
            &db,
            2,
            &crate::user_vocab::NewWord::simple("Garden", "unknown"),
        )
        .unwrap();
        assert_eq!(super::due_words(&db, None).unwrap().len(), 1);
        super::record_review(&mut db, a, "easy").unwrap();
        assert!(super::due_words(&db, Some(2)).unwrap().is_empty());
        let stats = super::learning_stats(&db).unwrap();
        assert_eq!(stats.total_words, 1);
        assert_eq!(stats.total_reviews, 1);
        db.execute("DELETE FROM vocab_book", []).unwrap();
        assert!(super::due_words(&db, None).unwrap().is_empty());
        assert_eq!(super::learning_stats(&db).unwrap().total_reviews, 1);
        assert_eq!(super::learning_stats(&db).unwrap().total_words, 1);
        db.execute("INSERT INTO vocab_book(name) VALUES ('C')", [])
            .unwrap();
        let book = db.last_insert_rowid();
        let reimported = crate::user_vocab::insert_word(
            &db,
            book,
            &crate::user_vocab::NewWord::simple("garden", "unknown"),
        )
        .unwrap();
        assert!(reimported.inherited);
        assert_eq!(
            crate::user_vocab::load_words(&db, book).unwrap()[0].proficiency,
            "mastered"
        );
        drop(db);
        drop(state);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
