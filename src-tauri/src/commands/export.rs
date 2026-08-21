use crate::db::DbState;
use std::io::Write;
use tauri::State;

fn proficiency_label(p: &str) -> &str {
    match p {
        "mastered" => "已掌握",
        "familiar" => "熟悉",
        _ => "生疏",
    }
}

/// Load (word, definition, phonetic, example, proficiency, memory_tag) rows.
fn load_word_rows(
    db: &rusqlite::Connection,
    vocab_book_id: i64,
) -> Result<Vec<(String, String, String, String, String, String)>, String> {
    let mut stmt = db
        .prepare(
            "SELECT word, definition, phonetic, example_sentence, proficiency, memory_tag \
             FROM vocab_word WHERE vocab_book_id=?1 ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![vocab_book_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

/// Export a vocab book as an Excel (.xlsx) spreadsheet.
#[tauri::command]
pub fn export_vocab_words_xlsx(
    state: State<DbState>,
    vocab_book_id: i64,
    file_path: String,
) -> Result<usize, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let rows = load_word_rows(&db, vocab_book_id)?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    let headers = ["单词", "释义", "音标", "例句", "熟练度", "标签"];
    for (c, h) in headers.iter().enumerate() {
        worksheet
            .write_string(0, c as u16, *h)
            .map_err(|e| format!("写入表头失败: {}", e))?;
    }

    for (i, (w, d, p, es, prof, mt)) in rows.iter().enumerate() {
        let r = (i + 1) as u32;
        worksheet
            .write_string(r, 0, w.as_str())
            .and_then(|ws| ws.write_string(r, 1, d.as_str()))
            .and_then(|ws| ws.write_string(r, 2, p.as_str()))
            .and_then(|ws| ws.write_string(r, 3, es.as_str()))
            .and_then(|ws| ws.write_string(r, 4, proficiency_label(prof)))
            .and_then(|ws| ws.write_string(r, 5, mt.as_str()))
            .map_err(|e| format!("写入数据失败: {}", e))?;
    }

    workbook.save(&file_path).map_err(|e| format!("保存 Excel 失败: {}", e))?;
    Ok(rows.len())
}

// ---------------------------------------------------------------------------
// Anki .apkg export
// ---------------------------------------------------------------------------

const ANKI_SCHEMA: &str = "
CREATE TABLE col (
    id integer primary key,
    crt integer not null,
    mod integer not null,
    scm integer not null,
    ver integer not null,
    dty integer not null,
    usn integer not null,
    ls integer not null,
    conf text not null,
    models text not null,
    decks text not null,
    dconf text not null,
    tags text not null
);
CREATE TABLE notes (
    id integer primary key,
    guid text not null,
    mid integer not null,
    mod integer not null,
    usn integer not null,
    tags text not null,
    flds text not null,
    sfld integer not null,
    csum integer not null,
    flags integer not null,
    data text not null
);
CREATE TABLE cards (
    id integer primary key,
    nid integer not null,
    did integer not null,
    ord integer not null,
    mod integer not null,
    usn integer not null,
    type integer not null,
    queue integer not null,
    due integer not null,
    ivl integer not null,
    factor integer not null,
    reps integer not null,
    lapses integer not null,
    left integer not null,
    odue integer not null,
    odid integer not null,
    flags integer not null,
    data text not null
);
CREATE TABLE revlog (
    id integer primary key,
    cid integer not null,
    usn integer not null,
    ease integer not null,
    ivl integer not null,
    lastIvl integer not null,
    factor integer not null,
    time integer not null,
    type integer not null
);
CREATE TABLE graves (
    usn integer not null,
    oid integer not null,
    type integer not null
);
CREATE INDEX ix_notes_usn ON notes (usn);
CREATE INDEX ix_cards_usn ON cards (usn);
CREATE INDEX ix_revlog_usn ON revlog (usn);
CREATE INDEX ix_cards_nid ON cards (nid);
CREATE INDEX ix_cards_sched ON cards (did, queue, due);
CREATE INDEX ix_revlog_cid ON revlog (cid);
CREATE INDEX ix_notes_csum ON notes (csum);
";

const MODEL_ID: i64 = 1_700_000_000_000;
const DECK_ID: i64 = 1;

fn anki_models_json() -> String {
    serde_json::json!({
        "1700000000000": {
            "id": MODEL_ID,
            "name": "词阅单词",
            "type": 0,
            "mod": 1_700_000_000,
            "usn": -1,
            "sortf": 0,
            "did": DECK_ID,
            "tmpls": [{
                "name": "卡片 1",
                "ord": 0,
                "qfmt": "{{单词}}",
                "afmt": "{{FrontSide}}<hr id=answer>{{释义}}",
                "bqfmt": "",
                "bafmt": "",
                "did": null,
                "bfont": "",
                "bsize": 0
            }],
            "flds": [
                {"name": "单词", "ord": 0, "sticky": false, "rtl": false, "font": "Arial", "size": 20},
                {"name": "释义", "ord": 1, "sticky": false, "rtl": false, "font": "Arial", "size": 20}
            ],
            "css": ".card { font-family: arial; font-size: 20px; text-align: center; color: black; background-color: white; }",
            "latexPre": "\\documentclass[12pt]{article}\n\\special{papersize=3in,5in}\n\\usepackage[utf8]{inputenc}\n\\usepackage{amssymb,amsmath}\n\\pagestyle{empty}\n\\setlength{\\parindent}{0in}\n\\begin{document}\n",
            "latexPost": "\\end{document}",
            "req": [[0, "any", [0]]]
        }
    })
    .to_string()
}

fn anki_decks_json(deck_name: &str) -> String {
    serde_json::json!({
        "1": {
            "id": DECK_ID,
            "name": deck_name,
            "mod": 1_700_000_000,
            "usn": -1,
            "lrnToday": [0, 0],
            "revToday": [0, 0],
            "newToday": [0, 0],
            "midToday": [0, 0],
            "desc": "",
            "dyn": 0,
            "collapsed": false,
            "extendNew": 10,
            "extendRev": 50,
            "conf": 1,
            "browserCollapsed": false
        }
    })
    .to_string()
}

/// Export a vocab book as an Anki `.apkg` package (word → definition basic card).
#[tauri::command]
pub fn export_vocab_words_apkg(
    state: State<DbState>,
    vocab_book_id: i64,
    deck_name: String,
    file_path: String,
) -> Result<usize, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let rows = load_word_rows(&db, vocab_book_id)?;
    drop(db);

    if rows.is_empty() {
        return Err("词汇本没有单词，无法导出".into());
    }

    // Build the collection.anki2 SQLite database in a temp file, then zip it.
    let tmp_db = std::env::temp_dir().join(format!(
        "nw_collection_{}_{}.anki2",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let _ = std::fs::remove_file(&tmp_db);

    {
        let conn = rusqlite::Connection::open(&tmp_db).map_err(|e| format!("创建 Anki 数据库失败: {}", e))?;
        conn.execute_batch(ANKI_SCHEMA).map_err(|e| format!("建表失败: {}", e))?;

        let now_sec = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO col (id, crt, mod, scm, ver, dty, usn, ls, conf, models, decks, dconf, tags) \
             VALUES (1, ?1, ?2, ?3, 11, 0, -1, 0, '{}', ?4, ?5, '{}', '{}')",
            rusqlite::params![
                now_sec,
                chrono::Utc::now().timestamp_millis(),
                1_708_300_000_i64,
                anki_models_json(),
                anki_decks_json(&deck_name),
            ],
        )
        .map_err(|e| format!("写入 Anki 集合失败: {}", e))?;

        let mut note_stmt = conn
            .prepare(
                "INSERT INTO notes (id, guid, mid, mod, usn, tags, flds, sfld, csum, flags, data) \
                 VALUES (?1, ?2, ?3, ?4, -1, '', ?5, ?6, 0, 0, '')",
            )
            .map_err(|e| e.to_string())?;
        let mut card_stmt = conn
            .prepare(
                "INSERT INTO cards (id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, reps, lapses, left, odue, odid, flags, data) \
                 VALUES (?1, ?2, ?3, 0, ?4, -1, 0, 0, ?5, 0, 0, 0, 0, 0, 0, 0, 0, '')",
            )
            .map_err(|e| e.to_string())?;

        for (i, (word, definition, _p, _es, _prof, _mt)) in rows.iter().enumerate() {
            let note_id = MODEL_ID + i as i64 + 1;
            let card_id = MODEL_ID + i as i64 + 1;
            let guid = format!("nw{:016x}", note_id);
            let flds = format!("{}\u{1f}{}", word, definition);

            note_stmt
                .execute(rusqlite::params![note_id, guid, MODEL_ID, now_sec, flds, word])
                .map_err(|e| format!("写入 Anki 笔记失败: {}", e))?;
            card_stmt
                .execute(rusqlite::params![card_id, note_id, DECK_ID, now_sec, note_id])
                .map_err(|e| format!("写入 Anki 卡片失败: {}", e))?;
        }
    }

    let db_bytes = std::fs::read(&tmp_db).map_err(|e| format!("读取 Anki 数据库失败: {}", e))?;
    let _ = std::fs::remove_file(&tmp_db);

    // Zip the collection into the .apkg file.
    let file = std::fs::File::create(&file_path).map_err(|e| format!("创建 apkg 文件失败: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("collection.anki2", options)
        .map_err(|e| format!("写入 apkg 失败: {}", e))?;
    zip.write_all(&db_bytes)
        .map_err(|e| format!("写入 apkg 失败: {}", e))?;

    zip.start_file("media", options)
        .map_err(|e| format!("写入 apkg 失败: {}", e))?;
    zip.write_all(b"{}")
        .map_err(|e| format!("写入 apkg 失败: {}", e))?;

    zip.finish().map_err(|e| format!("完成 apkg 失败: {}", e))?;

    Ok(rows.len())
}
