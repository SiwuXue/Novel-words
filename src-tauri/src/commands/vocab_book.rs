use crate::db::DbState;
use crate::models::VocabBook;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use tauri::{Manager, State};

pub const CET4_BOOK_NAME: &str = "四级真题核心词";
pub const CET4_BOOK_DESC: &str = "1162 条四级考试高频核心词，含真题例句、记忆法和常见搭配。数据来源：四级词汇乱序版。";

#[derive(Clone, Copy)]
pub struct BundledPreset {
    pub file_name: &'static str,
    pub preset_key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub word_count: u32,
}

pub const BUNDLED_PRESETS: &[BundledPreset] = &[
    BundledPreset {
        file_name: "CET4luan_1.json",
        preset_key: "cet4",
        name: CET4_BOOK_NAME,
        description: CET4_BOOK_DESC,
        word_count: 1162,
    },
    BundledPreset {
        file_name: "CET6luan_1.json",
        preset_key: "CET6luan_1",
        name: "六级真题核心词（图片记忆）",
        description: "1228 条六级真题核心词，包含释义、例句、记忆法和常见搭配。",
        word_count: 1228,
    },
    BundledPreset {
        file_name: "KaoYanluan_1.json",
        preset_key: "KaoYanluan_1",
        name: "考研必考词汇",
        description: "1341 条考研高频必考词汇，包含释义、例句、记忆法和常见搭配。",
        word_count: 1341,
    },
    BundledPreset {
        file_name: "Level4luan_1.json",
        preset_key: "Level4luan_1",
        name: "专四真题高频词",
        description: "595 条英语专业四级真题高频词，适合重点记忆和阅读匹配。",
        word_count: 595,
    },
    BundledPreset {
        file_name: "Level8_1.json",
        preset_key: "Level8_1",
        name: "专八真题高频词",
        description: "684 条英语专业八级真题高频词，适合重点记忆和阅读匹配。",
        word_count: 684,
    },
    BundledPreset {
        file_name: "CET4luan_2.json",
        preset_key: "CET4luan_2",
        name: "四级英语词汇",
        description: "3739 条大学英语四级词汇，适合系统学习和阅读匹配。",
        word_count: 3739,
    },
    BundledPreset {
        file_name: "CET6_2.json",
        preset_key: "CET6_2",
        name: "六级英语词汇",
        description: "2078 条大学英语六级词汇，适合系统学习和阅读匹配。",
        word_count: 2078,
    },
    BundledPreset {
        file_name: "KaoYan_2.json",
        preset_key: "KaoYan_2",
        name: "考研英语词汇",
        description: "4533 条考研英语词汇，覆盖核心与扩展学习范围。",
        word_count: 4533,
    },
    BundledPreset {
        file_name: "Level4luan_2.json",
        preset_key: "Level4luan_2",
        name: "专四核心词汇",
        description: "4025 条英语专业四级核心词汇，覆盖系统学习范围。",
        word_count: 4025,
    },
    BundledPreset {
        file_name: "Level8luan_2.json",
        preset_key: "Level8luan_2",
        name: "专八核心词汇",
        description: "12197 条英语专业八级核心词汇，覆盖系统与扩展学习范围。",
        word_count: 12197,
    },
    BundledPreset {
        file_name: "ChuZhongluan_2.json",
        preset_key: "ChuZhongluan_2",
        name: "中考必备词汇",
        description: "1420 条初中及中考必备英语词汇，适合基础学习和复习。",
        word_count: 1420,
    },
    BundledPreset {
        file_name: "GaoZhongluan_2.json",
        preset_key: "GaoZhongluan_2",
        name: "高考必备词汇（图片记忆）",
        description: "3668 条高中及高考必备英语词汇，包含图片记忆相关内容。",
        word_count: 3668,
    },
];

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
            "SELECT id, name, description, is_preset, preset_key, cloned_from_preset_key, created_at, updated_at FROM vocab_book ORDER BY updated_at DESC",
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
        "SELECT id, name, description, is_preset, preset_key, cloned_from_preset_key, created_at, updated_at FROM vocab_book WHERE id=?1",
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
        is_preset: row.get::<_, i64>(3)? != 0,
        preset_key: row.get(4)?,
        cloned_from_preset_key: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
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

/// 从资源目录解析逐行 JSON 并写入内置预设词表。
///
/// 从 lib.rs 的 setup 阶段调用：
/// - json_path: `resource_dir/resources/{preset.file_name}` 已解析好的路径
/// - conn: 主库（novel_words.db）的可变引用，调用方负责持有 MutexGuard
/// - 幂等：完整预设直接跳过解析；缺失的单词通过唯一索引自动补齐
pub fn ensure_preset_book_populated(
    json_path: &std::path::Path,
    preset: &BundledPreset,
    conn: &mut rusqlite::Connection,
) -> Result<Cet4ImportResult, String> {
    if !json_path.exists() {
        return Err(format!("找不到预设词汇文件：{}", json_path.display()));
    }

    // Fast path for normal subsequent launches: avoid reparsing all bundled
    // resources once this preset already contains its expected number of words.
    let complete_book = conn
        .query_row(
            "SELECT b.id, COUNT(w.id)
             FROM vocab_book b LEFT JOIN vocab_word w ON w.vocab_book_id = b.id
             WHERE b.is_preset = 1 AND b.preset_key = ?1
             GROUP BY b.id
             LIMIT 1",
            rusqlite::params![preset.preset_key],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, u32>(1)?)),
        )
        .optional()
        .map_err(|e| format!("查询预设词表完整性失败: {}", e))?;
    if let Some((book_id, count)) = complete_book {
        if count >= preset.word_count {
            conn.execute(
                "UPDATE vocab_book SET name = ?1, description = ?2 WHERE id = ?3",
                rusqlite::params![preset.name, preset.description, book_id],
            )
            .map_err(|e| format!("更新预设词表元数据失败: {}", e))?;
            return Ok(Cet4ImportResult {
                book_id,
                imported: 0,
                skipped: preset.word_count,
                total_in_file: preset.word_count,
            });
        }
    }

    // 读取并解析 NDJSON
    let file = std::fs::File::open(json_path)
        .map_err(|e| format!("打开预设词汇文件失败: {}", e))?;
    let reader = BufReader::new(file);
    let mut entries: Vec<Cet4Entry> = Vec::with_capacity(5000);
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

    // 优先按稳定的预设 ID 复用；仅 CET4 兼容旧版本按名称创建的记录。
    let existing_id = tx
        .query_row(
            "SELECT id FROM vocab_book WHERE is_preset = 1 AND preset_key = ?1 LIMIT 1",
            rusqlite::params![preset.preset_key],
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| format!("查询预设词表失败: {}", e))?
        .or_else(|| {
            if preset.preset_key != "cet4" {
                return None;
            }
            tx.query_row(
                "SELECT id FROM vocab_book WHERE name = ?1 LIMIT 1",
                rusqlite::params![preset.name],
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .ok()
            .flatten()
        });
    let book_id = if let Some(id) = existing_id {
        id
    } else {
        tx.execute(
            "INSERT INTO vocab_book (name, description, is_preset, preset_key) VALUES (?1, ?2, 1, ?3)",
            rusqlite::params![preset.name, preset.description, preset.preset_key],
        )
        .map_err(|e| format!("创建预设词表失败: {}", e))?;
        tx.last_insert_rowid()
    };

    // 每次启动同步内置元数据，并自愈旧版本的预设标记。
    tx.execute(
        "UPDATE vocab_book SET name = ?1, description = ?2, is_preset = 1, preset_key = ?3 WHERE id = ?4",
        rusqlite::params![preset.name, preset.description, preset.preset_key, book_id],
    )
    .map_err(|e| format!("更新预设词表元数据失败: {}", e))?;

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

pub fn ensure_cet4_book_populated(
    json_path: &std::path::Path,
    conn: &mut rusqlite::Connection,
) -> Result<Cet4ImportResult, String> {
    ensure_preset_book_populated(json_path, &BUNDLED_PRESETS[0], conn)
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

#[cfg(test)]
mod tests {
    use super::{ensure_preset_book_populated, BUNDLED_PRESETS};
    use rusqlite::Connection;

    #[test]
    fn imports_every_bundled_preset_idempotently() {
        let expected = [
            (1162_u32, 1162_u32),
            (1228, 1228),
            (1341, 1341),
            (595, 595),
            (684, 684),
            (3739, 3739),
            (2078, 2078),
            (4533, 4533),
            (4025, 4025),
            (12197, 12197),
            (1420, 1420),
            (3668, 3668),
        ];
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE vocab_book (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               name TEXT NOT NULL,
               description TEXT NOT NULL DEFAULT '',
               is_preset INTEGER NOT NULL DEFAULT 0,
               preset_key TEXT NOT NULL DEFAULT ''
             );
             CREATE TABLE vocab_word (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               vocab_book_id INTEGER NOT NULL,
               word TEXT NOT NULL,
               definition TEXT NOT NULL DEFAULT '',
               phonetic TEXT NOT NULL DEFAULT '',
               example_sentence TEXT NOT NULL DEFAULT '',
               proficiency TEXT NOT NULL DEFAULT 'unknown',
               memory_tag TEXT NOT NULL DEFAULT ''
             );
             CREATE UNIQUE INDEX idx_vocab_word_unique
               ON vocab_word (vocab_book_id, word);",
        )
        .unwrap();
        let resources = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources");

        for (preset, (source_count, unique_count)) in BUNDLED_PRESETS.iter().zip(expected) {
            let path = resources.join(preset.file_name);
            let first = ensure_preset_book_populated(&path, preset, &mut conn).unwrap();
            assert_eq!(first.total_in_file, source_count, "{}", preset.preset_key);
            assert_eq!(first.imported, unique_count, "{}", preset.preset_key);

            let second = ensure_preset_book_populated(&path, preset, &mut conn).unwrap();
            assert_eq!(second.imported, 0, "{}", preset.preset_key);
            assert_eq!(second.skipped, source_count, "{}", preset.preset_key);
        }

        let preset_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM vocab_book WHERE is_preset = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(preset_count, BUNDLED_PRESETS.len() as i64);
    }
}
