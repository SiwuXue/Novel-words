//! 嵌入式词典模块 — 独立只读 SQLite 连接，与主库 novel_words.db 隔离。
//!
//! 数据来源：scripts/build_dictionary_db.py 生成的 dictionary.db
//! 表结构：dict_word(word PK, phonetic_uk, phonetic_us, frequency, difficulty, translation)
//!
//! 两种查询：
//! - 英→中：精确匹配 word 字段（COLLATE NOCASE 大小写不敏感）
//! - 中→英：LIKE '%keyword%' 模糊匹配 translation 字段，返回最多 20 条按词频降序

use crate::models::dict_word::DictWord;
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

pub struct DictDbState {
    pub db: Mutex<Connection>,
}

impl DictDbState {
    pub fn open(db_path: PathBuf) -> Result<Self, String> {
        let conn = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("无法打开词典库: {}", e))?;
        Ok(Self {
            db: Mutex::new(conn),
        })
    }
}

fn row_to_dict_word(row: &rusqlite::Row) -> rusqlite::Result<DictWord> {
    Ok(DictWord {
        word: row.get(0)?,
        phonetic_uk: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        phonetic_us: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        translation: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
        frequency: row.get(4)?,
        difficulty: row.get(5)?,
    })
}

/// 英→中：精确查英文单词（大小写不敏感），返回单条或 None
#[tauri::command]
pub fn dict_lookup_english(
    state: State<DictDbState>,
    word: String,
) -> Result<Option<DictWord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let result = db
        .prepare(
            "SELECT word, phonetic_uk, phonetic_us, translation,
                    COALESCE(frequency, 0.0), COALESCE(difficulty, 0)
             FROM dict_word WHERE word = ?1 COLLATE NOCASE LIMIT 1",
        )
        .map_err(|e| format!("查询失败: {}", e))?
        .query_row(rusqlite::params![word.trim()], row_to_dict_word)
        .ok();
    Ok(result)
}

/// 中→英：模糊查中文释义包含关键词的英文单词，最多 20 条按词频降序
#[tauri::command]
pub fn dict_lookup_chinese(
    state: State<DictDbState>,
    keyword: String,
) -> Result<Vec<DictWord>, String> {
    let kw = keyword.trim();
    if kw.is_empty() {
        return Ok(Vec::new());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let pattern = format!("%{}%", kw);
    let mut stmt = db
        .prepare(
            "SELECT word, phonetic_uk, phonetic_us, translation,
                    COALESCE(frequency, 0.0), COALESCE(difficulty, 0)
             FROM dict_word
             WHERE translation LIKE ?1 AND translation != ''
             ORDER BY frequency DESC
             LIMIT 20",
        )
        .map_err(|e| format!("查询失败: {}", e))?;
    let rows = stmt
        .query_map(rusqlite::params![pattern], row_to_dict_word)
        .map_err(|e| format!("查询失败: {}", e))?;
    let results: Vec<DictWord> = rows.filter_map(|r| r.ok()).collect();
    Ok(results)
}
