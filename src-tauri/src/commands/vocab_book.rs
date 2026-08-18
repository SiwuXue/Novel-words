use crate::db::DbState;
use crate::models::VocabBook;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use tauri::{Manager, State};

pub const CET4_BOOK_NAME: &str = "四级真题核心词";
pub const CET4_BOOK_DESC: &str = "1162 条四级考试高频核心词，含真题例句、记忆法和常见搭配。数据来源：四级词汇乱序版。";

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct Cet4ImportResult {
    pub book_id: i64,
    pub imported: u32,
    pub skipped: u32,
    pub total_in_file: u32,
}

// ===== Partial JSON structures for CET4luan_1.ndjson =====
// All structs use rename_all = "camelCase" because the file keys are
// e.g. "headWord", "wordHead", "wordId", "sContent", "sCn", "tranCn", etc.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cet4Entry {
    head_word: String,
    content: Cet4OuterContent,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cet4OuterContent {
    word: Cet4WordWrapper,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cet4WordWrapper {
    word_head: Option<String>,
    word_id: Option<String>,
    content: Cet4InnerContent,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cet4InnerContent {
    usphone: Option<String>,
    ukphone: Option<String>,
    phone: Option<String>,
    trans: Option<Vec<Cet4Trans>>,
    sentence: Option<Cet4SentenceBlock>,
    rem_method: Option<Cet4RemMethod>,
    phrase: Option<Cet4PhraseBlock>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Cet4Trans {
    tran_cn: String,
    pos: Option<String>,
    tran_other: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cet4SentenceBlock {
    sentences: Option<Vec<Cet4Sentence>>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Cet4Sentence {
    s_content: Option<String>,
    s_cn: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Cet4RemMethod {
    val: Option<String>,
    desc: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cet4PhraseBlock {
    phrases: Option<Vec<Cet4Phrase>>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Cet4Phrase {
    p_content: Option<String>,
    p_cn: Option<String>,
}

#[tauri::command]
pub fn create_vocab_book(
    state: State<DbState>,
    name: String,
    description: String,
) -> Result<VocabBook, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO vocab_book (name, description) VALUES (?1, ?2)",
        rusqlite::params![name, description],
    )
    .map_err(|e| format!("创建词汇本失败: {}", e))?;

    let id = db.last_insert_rowid();
    get_vocab_book_by_id(&db, id)
}

#[tauri::command]
pub fn get_all_vocab_books(state: State<DbState>) -> Result<Vec<VocabBook>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, name, description, created_at, updated_at FROM vocab_book ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let books = stmt
        .query_map([], row_to_vocab_book)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(books)
}

#[tauri::command]
pub fn update_vocab_book(
    state: State<DbState>,
    id: i64,
    name: String,
    description: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let affected = db
        .execute(
            "UPDATE vocab_book SET name=?1, description=?2, updated_at=datetime('now','localtime') WHERE id=?3",
            rusqlite::params![name, description, id],
        )
        .map_err(|e| format!("更新词汇本失败: {}", e))?;

    if affected == 0 {
        return Err("词汇本不存在".into());
    }
    Ok(())
}

#[tauri::command]
pub fn delete_vocab_book(state: State<DbState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM vocab_book WHERE id=?1", rusqlite::params![id])
        .map_err(|e| format!("删除词汇本失败: {}", e))?;
    Ok(())
}

fn get_vocab_book_by_id(db: &rusqlite::Connection, id: i64) -> Result<VocabBook, String> {
    db.query_row(
        "SELECT id, name, description, created_at, updated_at FROM vocab_book WHERE id=?1",
        rusqlite::params![id],
        row_to_vocab_book,
    )
    .map_err(|e| format!("未找到该词汇本: {}", e))
}

fn row_to_vocab_book(row: &rusqlite::Row) -> rusqlite::Result<VocabBook> {
    Ok(VocabBook {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

// ===== 四级真题核心词一键导入 =====

fn build_definition(entry: &Cet4Entry) -> String {
    let c = &entry.content.word.content;
    let mut parts: Vec<String> = Vec::new();

    // 1. 词性 + 释义（按 trans 字段）
    if let Some(trans) = &c.trans {
        for t in trans {
            let mut line = String::new();
            if let Some(pos) = &t.pos {
                if !pos.is_empty() {
                    line.push_str(pos);
                    line.push_str(". ");
                }
            }
            line.push_str(&t.tran_cn);
            parts.push(line);
        }
    } else {
        // trans 为空时给个兜底
        parts.push("（暂无释义）".into());
    }

    // 2. 记忆方法
    if let Some(rem) = &c.rem_method {
        if let Some(val) = &rem.val {
            if !val.is_empty() {
                parts.push(format!("【记忆】{}", val));
            }
        }
    }

    // 3. 常见搭配（取前 5 条）
    if let Some(phrase) = &c.phrase {
        if let Some(phrases) = &phrase.phrases {
            if !phrases.is_empty() {
                parts.push("【搭配】".into());
                for p in phrases.iter().take(5) {
                    match (&p.p_content, &p.p_cn) {
                        (Some(en), Some(cn)) => {
                            parts.push(format!("  · {} {}", en, cn));
                        }
                        (Some(en), None) => {
                            parts.push(format!("  · {}", en));
                        }
                        (None, Some(cn)) => {
                            parts.push(format!("  · {}", cn));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    parts.join("\n")
}

fn build_example(entry: &Cet4Entry) -> String {
    let c = &entry.content.word.content;
    if let Some(sent_block) = &c.sentence {
        if let Some(sentences) = &sent_block.sentences {
            for s in sentences.iter().take(1) {
                match (&s.s_content, &s.s_cn) {
                    (Some(en), Some(cn)) => return format!("{}\n{}", en, cn),
                    (Some(en), None) => return en.clone(),
                    (None, Some(cn)) => return cn.clone(),
                    _ => {}
                }
            }
        }
    }
    String::new()
}

fn build_phonetic(entry: &Cet4Entry) -> String {
    let c = &entry.content.word.content;
    // 优先美音（用户默认 settings 的 speechAccent 通常是 us）
    if let Some(us) = &c.usphone {
        if !us.is_empty() {
            return us.clone();
        }
    }
    if let Some(uk) = &c.ukphone {
        if !uk.is_empty() {
            return uk.clone();
        }
    }
    if let Some(p) = &c.phone {
        if !p.is_empty() {
            return p.clone();
        }
    }
    String::new()
}

fn build_memory_tag(entry: &Cet4Entry) -> String {
    let c = &entry.content.word.content;
    if let Some(rem) = &c.rem_method {
        if let Some(desc) = &rem.desc {
            if !desc.is_empty() {
                return desc.clone();
            }
        }
        if let Some(val) = &rem.val {
            if !val.is_empty() {
                return val.clone();
            }
        }
    }
    String::new()
}

/// 从资源目录解析 JSON 并写入库。
///
/// 从 lib.rs 的 setup 阶段调用：
/// - json_path: `resource_dir/resources/CET4luan_1.json` 已解析好的路径
/// - conn: 主库（novel_words.db）的可变引用，调用方负责持有 MutexGuard
/// - 幂等：已存在相同 book.name / 相同 word 的全部跳过
pub fn ensure_cet4_book_populated(
    json_path: &std::path::Path,
    conn: &mut rusqlite::Connection,
) -> Result<Cet4ImportResult, String> {
    if !json_path.exists() {
        return Err(format!("找不到四级词汇文件：{}", json_path.display()));
    }

    // 读取并解析 NDJSON
    let file = std::fs::File::open(json_path)
        .map_err(|e| format!("打开四级词汇文件失败: {}", e))?;
    let reader = BufReader::new(file);
    let mut entries: Vec<Cet4Entry> = Vec::with_capacity(1200);
    for (idx, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("第 {} 行读取失败: {}", idx + 1, e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry: Cet4Entry = serde_json::from_str(trimmed)
            .map_err(|e| format!("第 {} 行 JSON 解析失败: {}", idx + 1, e))?;
        entries.push(entry);
    }
    let total_in_file = entries.len() as u32;

    let tx = conn.transaction().map_err(|e| format!("开启事务失败: {}", e))?;

    // 复用或创建词汇本
    let book_id: i64 = tx
        .query_row(
            "SELECT id FROM vocab_book WHERE name = ?1 LIMIT 1",
            rusqlite::params![CET4_BOOK_NAME],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or_else(|_| {
            tx.execute(
                "INSERT INTO vocab_book (name, description) VALUES (?1, ?2)",
                rusqlite::params![CET4_BOOK_NAME, CET4_BOOK_DESC],
            )
            .expect("创建四级词汇本失败");
            tx.last_insert_rowid()
        });

    // 逐词写入
    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;
    {
        let mut stmt = tx
            .prepare(
                "INSERT OR IGNORE INTO vocab_word
                 (vocab_book_id, word, definition, phonetic, example_sentence, proficiency, memory_tag)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|e| format!("准备插入语句失败: {}", e))?;
        for entry in &entries {
            let word = entry.head_word.trim().to_string();
            if word.is_empty() {
                skipped += 1;
                continue;
            }
            let n = stmt
                .execute(rusqlite::params![
                    book_id,
                    word,
                    build_definition(entry),
                    build_phonetic(entry),
                    build_example(entry),
                    "unknown",
                    build_memory_tag(entry),
                ])
                .map_err(|e| format!("写入单词 '{}' 失败: {}", word, e))?;
            if n > 0 {
                imported += 1;
            } else {
                skipped += 1;
            }
        }
    }

    tx.commit().map_err(|e| format!("提交事务失败: {}", e))?;
    Ok(Cet4ImportResult {
        book_id,
        imported,
        skipped,
        total_in_file,
    })
}

/// 保留 Tauri 命令，方便后续前端从设置里重新预装。
#[tauri::command]
pub fn import_cet4_core_words(
    app: tauri::AppHandle,
    state: State<DbState>,
) -> Result<Cet4ImportResult, String> {
    let json_path: PathBuf = app
        .path()
        .resource_dir()
        .map_err(|e| format!("无法解析资源目录: {}", e))?
        .join("resources")
        .join("CET4luan_1.json");
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    ensure_cet4_book_populated(&json_path, &mut *db)
}
