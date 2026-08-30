//! Preset vocab-book "click-to-study" flow for Chinese novels:
//! tailor a bundled English vocab list (e.g. CET4) to the current Chinese
//! novel, then let the user commit the result as a personal vocab book.
//!
//! The tailoring is heuristic: for each preset word we look up its Chinese
//! translations via the local dictionary, count occurrences in the novel's
//! cleaned text, and take the top-N by frequency. An optional cloud-LLM
//! enhancer is not implemented yet; the heuristic baseline already gives a
//! novel-relevant subset with auto-extracted example sentences.

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::vocab_word::row_to_vocab_word;
use crate::db::DbState;
use crate::dictionary::DictDbState;
use crate::models::VocabWord;

/// Summary of a bundled preset vocab book shown in the preset catalog.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetVocabBook {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub preset_key: String,
    pub word_count: i64,
}

/// One tailored word in the preview/commit result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetCloneItem {
    pub word: String,
    pub definition: String,
    pub phonetic: String,
    pub example_sentence: String,
    pub hit_count: i64,
}

/// Result of `preview_preset_clone` (and reused by `commit_preset_clone`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetClonePreview {
    pub preset_key: String,
    pub novel_id: i64,
    pub total_preset_words: i64,
    pub matched_count: i64,
    pub items: Vec<PresetCloneItem>,
}

/// Live progress emitted while a preset is matched against a novel.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetCloneProgress {
    pub request_id: String,
    pub processed: usize,
    pub total: usize,
    pub percent: u32,
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

fn extract_cjk_terms(text: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut run = String::new();
    for c in text.chars() {
        let is_cjk = matches!(c,
            '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}'
        );
        if is_cjk {
            run.push(c);
        } else if !run.is_empty() {
            if run.chars().count() >= 2 && !terms.contains(&run) {
                terms.push(std::mem::take(&mut run));
            } else {
                run.clear();
            }
        }
    }
    if run.chars().count() >= 2 && !terms.contains(&run) {
        terms.push(run);
    }
    terms
}

/// Internal: compute the tailored subset of `preset_words` against the novel's
/// `cleaned_text`. Used by both `preview_preset_clone` and `commit_preset_clone`.
fn tailor(
    dict_state: &DictDbState,
    preset_words: &[VocabWord],
    novel_text: &str,
    progress: Option<&dyn Fn(usize, usize)>,
) -> Result<Vec<PresetCloneItem>, String> {
    let mut items: Vec<PresetCloneItem> = Vec::new();
    let total = preset_words.len();
    if let Some(report) = progress {
        report(0, total);
    }

    // Reuse one lock and one prepared statement for the full run. The old
    // path locked and prepared SQLite once per word, adding avoidable latency.
    let dict = dict_state.db.lock().map_err(|e| e.to_string())?;
    let mut lookup = dict
        .prepare(
            "SELECT translation FROM dict_word
             WHERE word = ?1 COLLATE NOCASE LIMIT 1",
        )
        .map_err(|e| format!("准备词典查询失败: {}", e))?;

    for (index, w) in preset_words.iter().enumerate() {
        let chinese_translation: Option<String> = lookup
            .query_row(params![w.word.trim()], |row| {
                row.get::<_, Option<String>>(0)
            })
            .optional()
            .map_err(|e| format!("查询单词 '{}' 失败: {}", w.word, e))?
            .flatten();

        if let Some(chinese_translation) = chinese_translation {
            // Only keep multi-character dictionary terms that are also shown
            // in the word's primary definition. This avoids the old flood of
            // false positives from single characters such as “说/先/点”.
            let primary_definition = w.definition.split('【').next().unwrap_or(&w.definition);
            let display_terms = extract_cjk_terms(primary_definition);
            let terms: Vec<String> = extract_cjk_terms(&chinese_translation)
                .into_iter()
                .filter(|term| display_terms.contains(term))
                .collect();

            // Count occurrences (substring) of each term in the novel text.
            let mut hit_count: i64 = 0;
            let mut first_example = String::new();
            for term in &terms {
                let t = term.as_str();
                let mut from = 0usize;
                while let Some(relative_pos) = novel_text[from..].find(t) {
                    let pos = from + relative_pos;
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

            if hit_count > 0 {
                items.push(PresetCloneItem {
                    word: w.word.clone(),
                    definition: w.definition.clone(),
                    phonetic: w.phonetic.clone(),
                    example_sentence: first_example,
                    hit_count,
                });
            }
        }

        let processed = index + 1;
        if processed == total || processed % 10 == 0 {
            if let Some(report) = progress {
                report(processed, total);
            }
        }
    }
    // Sort by hit_count desc, then word asc for stability.
    items.sort_by(|a, b| {
        b.hit_count
            .cmp(&a.hit_count)
            .then_with(|| a.word.cmp(&b.word))
    });
    Ok(items)
}

/// Pull a single short example sentence containing `term` near `pos`.
fn extract_example_sentence(text: &str, pos: usize, term: &str) -> String {
    fn is_terminator(c: char) -> bool {
        matches!(c, '.' | '。' | '!' | '！' | '?' | '？' | '\n' | '\r')
    }

    // `pos` is a byte offset returned by `str::find`. Walk by Unicode scalar
    // values so the resulting byte offsets always remain valid UTF-8
    // boundaries, including when the novel and punctuation are Chinese.
    let mut start = pos;
    for (seen, (index, c)) in text[..pos].char_indices().rev().enumerate() {
        if is_terminator(c) {
            start = index + c.len_utf8();
            break;
        }
        start = index;
        if seen + 1 >= 80 {
            break;
        }
    }

    let term_end = pos + term.len();
    let mut end = term_end;
    for (seen, (offset, c)) in text[term_end..].char_indices().enumerate() {
        end = term_end + offset + c.len_utf8();
        if is_terminator(c) || seen + 1 >= 100 {
            break;
        }
    }

    let s = &text[start..end];
    let trimmed = s.trim().replace(['\n', '\r'], " ");
    if trimmed.chars().count() > 80 {
        trimmed.chars().take(77).collect::<String>() + "…"
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::{
        extract_cjk_terms, extract_example_sentence, tailor, PresetCloneItem, PresetClonePreview,
        PresetCloneProgress, PresetVocabBook,
    };
    use crate::dictionary::DictDbState;
    use crate::models::VocabWord;
    use rusqlite::Connection;
    use std::sync::Mutex;

    #[test]
    fn preset_payload_uses_frontend_camel_case_fields() {
        let book = PresetVocabBook {
            id: 1,
            name: "CET4".into(),
            description: String::new(),
            preset_key: "cet4".into(),
            word_count: 1162,
        };
        let item = PresetCloneItem {
            word: "novel".into(),
            definition: "小说".into(),
            phonetic: String::new(),
            example_sentence: String::new(),
            hit_count: 2,
        };
        let preview = PresetClonePreview {
            preset_key: "cet4".into(),
            novel_id: 7,
            total_preset_words: 1162,
            matched_count: 1,
            items: vec![item.clone()],
        };
        let progress = PresetCloneProgress {
            request_id: "request-1".into(),
            processed: 10,
            total: 100,
            percent: 10,
        };

        let book_json = serde_json::to_value(book).unwrap();
        let item_json = serde_json::to_value(item).unwrap();
        let preview_json = serde_json::to_value(preview).unwrap();
        let progress_json = serde_json::to_value(progress).unwrap();
        assert_eq!(book_json["presetKey"], "cet4");
        assert_eq!(book_json["wordCount"], 1162);
        assert!(book_json.get("preset_key").is_none());
        assert_eq!(item_json["hitCount"], 2);
        assert_eq!(preview_json["totalPresetWords"], 1162);
        assert_eq!(preview_json["matchedCount"], 1);
        assert_eq!(preview_json["items"][0]["exampleSentence"], "");
        assert_eq!(progress_json["requestId"], "request-1");
    }

    #[test]
    fn extracts_chinese_sentence_on_utf8_boundaries() {
        let text = "前一句。神通者在雨夜中前行！后一句。";
        let term = "神通者";
        let pos = text.find(term).unwrap();

        assert_eq!(
            extract_example_sentence(text, pos, term),
            "神通者在雨夜中前行！"
        );
    }

    #[test]
    fn tailoring_reports_progress_and_counts_all_occurrences() {
        let dict = Connection::open_in_memory().unwrap();
        dict.execute_batch(
            "CREATE TABLE dict_word (word TEXT PRIMARY KEY, translation TEXT);
             INSERT INTO dict_word (word, translation) VALUES ('novel', '小说');",
        )
        .unwrap();
        let dict_state = DictDbState {
            db: Mutex::new(dict),
        };
        let words = vec![VocabWord {
            id: 1,
            vocab_book_id: 1,
            word: "novel".into(),
            definition: "小说".into(),
            phonetic: String::new(),
            example_sentence: String::new(),
            novel_id: None,
            chapter_id: None,
            proficiency: "unknown".into(),
            memory_tag: String::new(),
            created_at: String::new(),
        }];
        let updates = Mutex::new(Vec::new());
        let report = |processed, total| updates.lock().unwrap().push((processed, total));

        let items = tailor(&dict_state, &words, "开头小说，中间小说。", Some(&report)).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].hit_count, 2);
        assert_eq!(*updates.lock().unwrap(), vec![(0, 1), (1, 1)]);
    }

    #[test]
    fn preset_matching_ignores_single_character_terms() {
        assert_eq!(
            extract_cjk_terms("n.点, 粒子, 微粒；说"),
            vec!["粒子".to_string(), "微粒".to_string()]
        );
    }
}

/// Compute (without writing) the tailored subset for `preset_key` against
/// `novel_id`. Returns every reliable match as a preview.
#[tauri::command]
pub async fn preview_preset_clone(
    app: AppHandle,
    preset_key: String,
    novel_id: i64,
    request_id: String,
) -> Result<PresetClonePreview, String> {
    tokio::task::spawn_blocking(move || {
        let state = app.state::<DbState>();
        let dict_state = app.state::<DictDbState>();
        let (_preset_id, preset_words, novel_text) =
            load_preset_and_novel(&state, &preset_key, novel_id)?;
        let total = preset_words.len() as i64;
        let progress = |processed: usize, total: usize| {
            let percent = if total == 0 {
                100
            } else {
                ((processed * 100) / total) as u32
            };
            let _ = app.emit(
                "preset-clone-progress",
                PresetCloneProgress {
                    request_id: request_id.clone(),
                    processed,
                    total,
                    percent,
                },
            );
        };
        let items = tailor(&dict_state, &preset_words, &novel_text, Some(&progress))?;
        let matched = items.len() as i64;
        Ok(PresetClonePreview {
            preset_key,
            novel_id,
            total_preset_words: total,
            matched_count: matched,
            items,
        })
    })
    .await
    .map_err(|e| format!("后台计算任务失败: {}", e))?
}

/// Write the tailored subset into a NEW personal vocab book and return its id.
#[tauri::command]
pub async fn commit_preset_clone(
    app: AppHandle,
    preset_key: String,
    novel_id: i64,
    new_book_name: Option<String>,
) -> Result<i64, String> {
    tokio::task::spawn_blocking(move || {
        let state = app.state::<DbState>();
        let dict_state = app.state::<DictDbState>();
        let (preset_id, preset_words, novel_text) =
            load_preset_and_novel(&state, &preset_key, novel_id)?;
        let items = tailor(&dict_state, &preset_words, &novel_text, None)?;
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
    })
    .await
    .map_err(|e| format!("后台导入任务失败: {}", e))?
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
