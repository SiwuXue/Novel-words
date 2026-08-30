//! Preset vocab-book "click-to-study" flow for Chinese novels:
//! tailor a bundled English vocab list (e.g. CET4) to the current Chinese
//! novel, then let the user commit the result as a personal vocab book.
//!
//! The tailoring is heuristic: for each preset word we look up its Chinese
//! translations via the local dictionary, count occurrences in the novel's
//! cleaned text, and take the top-N by frequency. An optional cloud-LLM
//! enhancer is not implemented yet; the heuristic baseline already gives a
//! novel-relevant subset with auto-extracted example sentences.

use rusqlite::params;
use serde::Serialize;
use tauri::State;

use crate::commands::vocab_word::row_to_vocab_word;
use crate::db::DbState;
use crate::dictionary::{dict_lookup_english, DictDbState};
use crate::models::VocabWord;

/// Summary of a bundled preset vocab book shown in the preset catalog.
#[derive(Debug, Clone, Serialize)]
pub struct PresetVocabBook {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub preset_key: String,
    pub word_count: i64,
}

/// One tailored word in the preview/commit result.
#[derive(Debug, Clone, Serialize)]
pub struct PresetCloneItem {
    pub word: String,
    pub definition: String,
    pub phonetic: String,
    pub example_sentence: String,
    pub hit_count: i64,
}

/// Result of `preview_preset_clone` (and reused by `commit_preset_clone`).
#[derive(Debug, Clone, Serialize)]
pub struct PresetClonePreview {
    pub preset_key: String,
    pub novel_id: i64,
    pub total_preset_words: i64,
    pub matched_count: i64,
    pub limit: i64,
    pub items: Vec<PresetCloneItem>,
}

/// List all bundled preset vocab books (read-only references).
#[tauri::command]
pub fn list_preset_vocab_books(state: State<DbState>) -> Result<Vec<PresetVocabBook>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT b.id, b.name, b.description, b.preset_key, COUNT(w.id)
             FROM vocab_book b LEFT JOIN vocab_word w ON w.vocab_book_id = b.id
             WHERE b.is_preset = 1
             GROUP BY b.id, b.name, b.description, b.preset_key
             ORDER BY b.preset_key, b.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PresetVocabBook {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                preset_key: row.get(3)?,
                word_count: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// Internal: compute the tailored subset of `preset_words` against the novel's
/// `cleaned_text`. Used by both `preview_preset_clone` and `commit_preset_clone`.
fn tailor(
    dict_state: &State<'_, DictDbState>,
    preset_words: &[VocabWord],
    novel_text: &str,
    limit: i64,
) -> Vec<PresetCloneItem> {
    let mut items: Vec<PresetCloneItem> = Vec::new();
    for w in preset_words {
        // Look up the Chinese translation(s) for this preset word.
        let chinese_translation = match dict_lookup_english(dict_state.clone(), w.word.clone()) {
            Ok(Some(entry)) => entry.translation,
            _ => continue,
        };
        // Split the translation into Chinese terms.
        let terms: Vec<&str> = chinese_translation
            .split(|c: char| matches!(c, ';' | '；' | ',' | '，' | '、' | '/' | '|' | ' ' | '\t' | '\n'))
            .filter(|s| !s.trim().is_empty())
            .collect();
        if terms.is_empty() {
            continue;
        }
        // Count occurrences (substring) of each term in the novel text.
        let mut hit_count: i64 = 0;
        let mut first_example = String::new();
        for term in &terms {
            let t = term.trim();
            if t.is_empty() {
                continue;
            }
            let mut from = 0usize;
            while let Some(pos) = novel_text[from..].find(t) {
                hit_count += 1;
                if first_example.is_empty() {
                    first_example = extract_example_sentence(novel_text, pos, t);
                }
                from = pos + t.len();
                if from >= novel_text.len() {
                    break;
                }
            }
        }
        if hit_count == 0 {
            continue;
        }
        items.push(PresetCloneItem {
            word: w.word.clone(),
            definition: w.definition.clone(),
            phonetic: w.phonetic.clone(),
            example_sentence: first_example,
            hit_count,
        });
    }
    // Sort by hit_count desc, then word asc for stability.
    items.sort_by(|a, b| b.hit_count.cmp(&a.hit_count).then_with(|| a.word.cmp(&b.word)));
    if limit > 0 {
        items.truncate(limit as usize);
    }
    items
}

/// Pull a single short example sentence containing `term` near `pos`.
fn extract_example_sentence(text: &str, pos: usize, term: &str) -> String {
    // Find the sentence boundaries around `pos` using common CJK / ASCII
    // sentence terminators. Keep the sentence containing the term.
    let bytes = text.as_bytes();
    let len = bytes.len();
    // scan backwards for '.', '。', '!', '！', '?', '？', '\n'
    let start = {
        let mut s = pos;
        while s > 0 {
            let c = bytes[s - 1] as char;
            if matches!(c, '.' | '。' | '!' | '！' | '?' | '？' | '\n' | '\r') {
                break;
            }
            s -= 1;
            if pos - s > 80 { break; } // don't search too far back
        }
        s
    };
    let end = {
        let mut e = pos + term.len();
        while e < len {
            let c = bytes[e] as char;
            if matches!(c, '.' | '。' | '!' | '！' | '?' | '？' | '\n' | '\r') {
                e += 1;
                break;
            }
            e += 1;
            if e - pos > 100 { break; } // don't search too far forward
        }
        e.min(len)
    };
    let s = &text[start..end];
    let trimmed = s.trim().replace(['\n', '\r'], " ");
    if trimmed.chars().count() > 80 {
        trimmed.chars().take(77).collect::<String>() + "…"
    } else {
        trimmed
    }
}

/// Compute (without writing) the tailored subset for `preset_key` against
/// `novel_id`. Returns counts + the first `limit` items as a preview.
#[tauri::command]
pub fn preview_preset_clone(
    state: State<DbState>,
    dict_state: State<'_, DictDbState>,
    preset_key: String,
    novel_id: i64,
    limit: i64,
) -> Result<PresetClonePreview, String> {
    let (preset_id, preset_words, novel_text) = load_preset_and_novel(&state, &preset_key, novel_id)?;
    let total = preset_words.len() as i64;
    let items = tailor(&dict_state, &preset_words, &novel_text, limit);
    let matched = items.len() as i64;
    Ok(PresetClonePreview {
        preset_key,
        novel_id,
        total_preset_words: total,
        matched_count: matched,
        limit,
        items,
    })
}

/// Write the tailored subset into a NEW personal vocab book and return its id.
#[tauri::command]
pub fn commit_preset_clone(
    state: State<DbState>,
    dict_state: State<'_, DictDbState>,
    preset_key: String,
    novel_id: i64,
    limit: i64,
    new_book_name: Option<String>,
) -> Result<i64, String> {
    let (preset_id, preset_words, novel_text) = load_preset_and_novel(&state, &preset_key, novel_id)?;
    let items = tailor(&dict_state, &preset_words, &novel_text, limit);
    if items.is_empty() {
        return Err("没有匹配的单词，无法导入".into());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Build a sensible default name.
    let novel_title: String = db
        .query_row(
            "SELECT title FROM novel WHERE id=?1",
            params![novel_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "未知小说".to_string());
    let name = new_book_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            // Translate preset_key into a friendly label.
            let label = match preset_key.as_str() {
                "cet4" => "CET4",
                "gaokao3500" => "高考3500",
                "ielts" => "雅思",
                "toefl" => "托福",
                other => other,
            };
            format!("{} · {}精选", label, novel_title)
        });

    db.execute(
        "INSERT INTO vocab_book (name, description, is_preset, preset_key, cloned_from_preset_key) \
         VALUES (?1, ?2, 0, '', ?3)",
        params![
            &name,
            format!("从「{}」按当前小说裁剪", preset_key),
            &preset_key,
        ],
    )
    .map_err(|e| format!("创建词表失败: {}", e))?;
    let new_book_id = db.last_insert_rowid();

    let mut stmt = db
        .prepare(
            "INSERT INTO vocab_word \
             (vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'unknown', '')",
        )
        .map_err(|e| format!("准备插入失败: {}", e))?;
    for it in &items {
        stmt.execute(params![
            new_book_id,
            &it.word,
            &it.definition,
            &it.phonetic,
            &it.example_sentence,
            novel_id,
        ])
        .map_err(|e| format!("插入单词 {} 失败: {}", it.word, e))?;
    }
    // Touch the source preset so its updated_at bumps (purely cosmetic).
    let _ = preset_id;

    Ok(new_book_id)
}

/// Load the preset vocab_book (by preset_key), its vocab_words, and the
/// novel's cleaned_text. Returns (preset_book_id, words, novel_text).
fn load_preset_and_novel(
    state: &State<'_, DbState>,
    preset_key: &str,
    novel_id: i64,
) -> Result<(i64, Vec<VocabWord>, String), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let preset_id: i64 = db
        .query_row(
            "SELECT id FROM vocab_book WHERE is_preset = 1 AND preset_key = ?1 LIMIT 1",
            params![preset_key],
            |row| row.get(0),
        )
        .map_err(|_| format!("未找到预设词表: {}", preset_key))?;
    let mut stmt = db
        .prepare("SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at FROM vocab_word WHERE vocab_book_id = ?1")
        .map_err(|e| e.to_string())?;
    let preset_words: Vec<VocabWord> = stmt
        .query_map(params![preset_id], row_to_vocab_word)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    let novel_text: String = db
        .query_row(
            "SELECT cleaned_text FROM novel WHERE id = ?1",
            params![novel_id],
            |row| row.get(0),
        )
        .map_err(|_| format!("未找到小说: id={}", novel_id))?;
    Ok((preset_id, preset_words, novel_text))
}
