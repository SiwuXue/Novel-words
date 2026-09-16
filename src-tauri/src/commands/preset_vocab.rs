//! Preset vocab-book "click-to-study" flow for Chinese novels:
//! tailor a bundled English vocab list (e.g. CET4) to the current Chinese
//! novel, then let the user commit the result as a personal vocab book.
//!
//! The tailoring is heuristic: for each preset word we look up its Chinese
//! translations via the local dictionary, count occurrences in the novel's
//! cleaned text, rank matches by frequency, and optionally ask an
//! OpenAI-compatible model to validate the contextual sense.

use std::collections::HashMap;

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::ai_enhancer::{
    enhance_vocab_items, load_ai_config, AiWordDecision, AiWordInput,
};
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
    pub category: String,
    pub sources: Vec<String>,
}

/// One tailored word in the preview/commit result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetCloneItem {
    pub word: String,
    pub definition: String,
    pub phonetic: String,
    pub example_sentence: String,
    pub hit_count: i64,
    #[serde(default, skip_serializing)]
    pub matched_terms: Vec<String>,
}

/// Result of `preview_preset_clone` (and reused by `commit_preset_clone`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetClonePreview {
    pub preset_key: String,
    pub novel_id: i64,
    pub total_preset_words: i64,
    pub matched_count: i64,
    pub ai_enhanced: bool,
    pub ai_message: String,
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
    pub stage: String,
}

/// List all bundled preset vocab books (read-only references).
#[tauri::command]
pub fn list_preset_vocab_books(
    state: State<DbState>,
    catalog: State<crate::preset_catalog::PresetCatalog>,
) -> Result<Vec<PresetVocabBook>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let manifest = catalog.register(&db)?;
    manifest
        .books
        .into_iter()
        .map(|book| {
            let id = db
                .query_row(
                    "SELECT id FROM vocab_book WHERE is_preset=1 AND preset_key=?1",
                    [&book.key],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            Ok(PresetVocabBook {
                id,
                name: book.name,
                description: book.description,
                preset_key: book.key,
                word_count: book.word_count as i64,
                category: book.category,
                sources: book.sources,
            })
        })
        .collect()
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
            let mut matched_terms = Vec::new();
            for term in &terms {
                let t = term.as_str();
                let hits_before = hit_count;
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
                if hit_count > hits_before {
                    matched_terms.push(term.clone());
                }
            }

            if hit_count > 0 {
                items.push(PresetCloneItem {
                    word: w.word.clone(),
                    definition: w.definition.clone(),
                    phonetic: w.phonetic.clone(),
                    example_sentence: first_example,
                    hit_count,
                    matched_terms,
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

fn build_ai_inputs(items: &[PresetCloneItem]) -> Vec<AiWordInput> {
    items
        .iter()
        .map(|item| AiWordInput {
            word: item.word.clone(),
            definition: item
                .definition
                .split('【')
                .next()
                .unwrap_or(&item.definition)
                .chars()
                .take(120)
                .collect(),
            example_sentence: item.example_sentence.clone(),
            matched_terms: item.matched_terms.clone(),
            hit_count: item.hit_count,
        })
        .collect()
}

fn part_of_speech_prefix(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let dot = trimmed.find('.')?;
    let prefix = &trimmed[..=dot];
    if prefix.len() <= 24
        && prefix
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || matches!(ch, ' ' | '/' | '-' | '.'))
    {
        Some(prefix.trim().to_string())
    } else {
        None
    }
}

fn contextual_part_of_speech(definition: &str, matched_terms: &[String]) -> Option<String> {
    let primary = definition.split('【').next().unwrap_or(definition);
    for line in primary.lines() {
        if matched_terms.iter().any(|term| line.contains(term)) {
            if let Some(prefix) = part_of_speech_prefix(line) {
                return Some(prefix);
            }
        }
    }
    primary.lines().find_map(part_of_speech_prefix)
}

fn definition_with_part_of_speech(
    original: &str,
    enhanced: &str,
    matched_terms: &[String],
) -> String {
    if part_of_speech_prefix(enhanced).is_some() {
        return enhanced.trim().to_string();
    }
    contextual_part_of_speech(original, matched_terms)
        .map(|pos| format!("{} {}", pos, enhanced.trim()))
        .unwrap_or_else(|| enhanced.trim().to_string())
}

fn apply_ai_decisions(items: &mut Vec<PresetCloneItem>, decisions: Vec<AiWordDecision>) {
    let decisions: HashMap<String, AiWordDecision> = decisions
        .into_iter()
        .map(|decision| (decision.word.trim().to_lowercase(), decision))
        .collect();
    items.retain_mut(|item| {
        let Some(decision) = decisions.get(&item.word.to_lowercase()) else {
            return true;
        };
        if !decision.keep {
            return false;
        }
        let definition = decision.context_definition.trim();
        if !definition.is_empty()
            && definition.chars().count() <= 40
            && item
                .matched_terms
                .iter()
                .any(|term| definition.contains(term))
        {
            item.definition =
                definition_with_part_of_speech(&item.definition, definition, &item.matched_terms);
        }
        let example = decision.example_sentence.trim();
        if !example.is_empty()
            && example.chars().count() <= 160
            && item.matched_terms.iter().any(|term| example.contains(term))
        {
            item.example_sentence = example.to_string();
        }
        true
    });
}

/// Repair AI-enhanced clones created by older versions that replaced the
/// source definition with a bare Chinese meaning. Preset books are available
/// by this point, so the correct contextual POS can be recovered by word.
pub fn repair_cloned_parts_of_speech(conn: &mut rusqlite::Connection) -> Result<usize, String> {
    let rows: Vec<(i64, String, String, String)> = {
        let mut stmt = conn
            .prepare(
                "SELECT cw.id, cw.definition, sw.definition, cw.match_terms \
                 FROM vocab_word cw \
                 JOIN vocab_book cb ON cb.id = cw.vocab_book_id AND cb.is_preset = 0 \
                 JOIN vocab_book pb ON pb.is_preset = 1 \
                   AND pb.preset_key = cb.cloned_from_preset_key \
                 JOIN vocab_word sw ON sw.vocab_book_id = pb.id \
                   AND lower(sw.word) = lower(cw.word) \
                 WHERE cb.cloned_from_preset_key <> ''",
            )
            .map_err(|e| format!("准备修复 AI 词性失败: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| format!("读取待修复 AI 词性失败: {}", e))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| format!("读取待修复 AI 词性失败: {}", e))?;
        rows
    };

    let repairs: Vec<(i64, String)> = rows
        .into_iter()
        .filter_map(|(id, current, source, encoded_terms)| {
            if part_of_speech_prefix(&current).is_some() {
                return None;
            }
            let terms = serde_json::from_str::<Vec<String>>(&encoded_terms).unwrap_or_default();
            let repaired = definition_with_part_of_speech(&source, &current, &terms);
            (repaired != current).then_some((id, repaired))
        })
        .collect();
    if repairs.is_empty() {
        return Ok(0);
    }

    let tx = conn
        .transaction()
        .map_err(|e| format!("开启 AI 词性修复事务失败: {}", e))?;
    {
        let mut update = tx
            .prepare("UPDATE vocab_word SET definition = ?1 WHERE id = ?2")
            .map_err(|e| format!("准备写入 AI 词性失败: {}", e))?;
        for (id, definition) in &repairs {
            update
                .execute(params![definition, id])
                .map_err(|e| format!("写入 AI 词性失败: {}", e))?;
        }
    }
    tx.commit()
        .map_err(|e| format!("提交 AI 词性修复失败: {}", e))?;
    Ok(repairs.len())
}

#[cfg(test)]
mod tests {
    use super::{
        definition_with_part_of_speech, extract_cjk_terms, extract_example_sentence, tailor,
        PresetCloneItem, PresetClonePreview, PresetCloneProgress, PresetVocabBook,
    };
    use crate::dictionary::DictDbState;
    use crate::models::VocabWord;
    use rusqlite::Connection;
    use std::sync::Mutex;

    #[test]
    fn whole_preset_import_inherits_personal_state_and_preserves_word_sources() {
        let dir = std::env::temp_dir().join(format!(
            "nw-preset-import-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let state = crate::db::init_db(&dir).unwrap();
        let mut db = state.db.lock().unwrap();
        let catalog = crate::preset_catalog::PresetCatalog {
            path: std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/presets.zip"),
        };
        let id = catalog.ensure_loaded(&mut db, "cet6-all", None).unwrap();
        let sample = crate::user_vocab::load_words(&db, id).unwrap().remove(0);
        db.execute("INSERT INTO vocab_book(name) VALUES ('已有')", [])
            .unwrap();
        let existing = db.last_insert_rowid();
        crate::user_vocab::insert_word(
            &db,
            existing,
            &crate::user_vocab::NewWord::simple(&sample.word, "mastered"),
        )
        .unwrap();
        let result = super::import_loaded_preset(&mut db, id, Some("六级个人词汇"), None).unwrap();
        assert_eq!(result.counts.imported, 3992);
        assert_eq!(result.counts.inherited, 1);
        assert_eq!(result.counts.new_words, 3991);
        let imported = crate::user_vocab::load_words(&db, result.book_id).unwrap();
        assert_eq!(
            imported
                .iter()
                .find(|w| w.word == sample.word)
                .unwrap()
                .proficiency,
            "mastered"
        );
        let source: String = db
            .query_row(
                "SELECT source_keys FROM vocab_word WHERE vocab_book_id=?1 LIMIT 1",
                [result.book_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_ne!(source, "[]");
        drop(db);
        drop(state);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn preset_payload_uses_frontend_camel_case_fields() {
        let book = PresetVocabBook {
            id: 1,
            name: "CET4".into(),
            description: String::new(),
            preset_key: "cet4".into(),
            word_count: 1162,
            category: "university".into(),
            sources: vec!["CET4luan_1".into()],
        };
        let item = PresetCloneItem {
            word: "novel".into(),
            definition: "小说".into(),
            phonetic: String::new(),
            example_sentence: String::new(),
            hit_count: 2,
            matched_terms: vec!["小说".into()],
        };
        let preview = PresetClonePreview {
            preset_key: "cet4".into(),
            novel_id: 7,
            total_preset_words: 1162,
            matched_count: 1,
            ai_enhanced: false,
            ai_message: String::new(),
            items: vec![item.clone()],
        };
        let progress = PresetCloneProgress {
            request_id: "request-1".into(),
            processed: 10,
            total: 100,
            percent: 10,
            stage: "local".into(),
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
    fn ai_definition_keeps_contextual_part_of_speech() {
        let terms = vec!["没有".to_string()];
        assert_eq!(
            definition_with_part_of_speech("adv. 不；并不\nadj. 没有", "没有", &terms,),
            "adj. 没有"
        );
        assert_eq!(
            definition_with_part_of_speech("n. 天赋；礼物", "n. 天赋", &["天赋".into()]),
            "n. 天赋"
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
            user_vocab_id: None,
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
            match_terms: String::new(),
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
    let (ai_config, initial_ai_message) = match load_ai_config(&app.state::<DbState>()) {
        Ok(config) if config.enabled => match config.validate() {
            Ok(()) => (Some(config), String::new()),
            Err(error) => (None, format!("AI 增强未运行，已使用本地结果：{}", error)),
        },
        Ok(_) => (None, String::new()),
        Err(error) => (None, format!("AI 设置读取失败，已使用本地结果：{}", error)),
    };
    let local_progress_max = if ai_config.is_some() { 70 } else { 100 };
    let local_app = app.clone();
    let local_request_id = request_id.clone();
    let local_preset_key = preset_key.clone();

    let mut preview = tokio::task::spawn_blocking(move || -> Result<PresetClonePreview, String> {
        let state = local_app.state::<DbState>();
        let dict_state = local_app.state::<DictDbState>();
        let loaded_id = {
            let mut db = state.db.lock().map_err(|e| e.to_string())?;
            let catalog = local_app.state::<crate::preset_catalog::PresetCatalog>();
            catalog.ensure_loaded(
                &mut db,
                &local_preset_key,
                Some(&|current, total| {
                    let _ = local_app.emit(
                        "preset-clone-progress",
                        PresetCloneProgress {
                            request_id: local_request_id.clone(),
                            processed: current as usize,
                            total: total as usize,
                            percent: if total == 0 { 20 } else { current * 20 / total },
                            stage: "local".into(),
                        },
                    );
                }),
            )?
        };
        let (_preset_id, preset_words, novel_text) =
            load_preset_and_novel(&state, loaded_id, novel_id)?;
        let total = preset_words.len() as i64;
        let progress = |processed: usize, total: usize| {
            let percent = if total == 0 {
                local_progress_max
            } else {
                20 + ((processed * (local_progress_max - 20) as usize) / total) as u32
            };
            let _ = local_app.emit(
                "preset-clone-progress",
                PresetCloneProgress {
                    request_id: local_request_id.clone(),
                    processed,
                    total,
                    percent,
                    stage: "local".into(),
                },
            );
        };
        let items = tailor(&dict_state, &preset_words, &novel_text, Some(&progress))?;
        let matched = items.len() as i64;
        Ok(PresetClonePreview {
            preset_key: local_preset_key,
            novel_id,
            total_preset_words: total,
            matched_count: matched,
            ai_enhanced: false,
            ai_message: initial_ai_message,
            items,
        })
    })
    .await
    .map_err(|e| format!("后台计算任务失败: {}", e))??;

    if let Some(config) = ai_config {
        let before = preview.items.len();
        let inputs = build_ai_inputs(&preview.items);
        let progress_app = app.clone();
        let progress_request_id = request_id.clone();
        match enhance_vocab_items(&config, &inputs, move |processed, total| {
            let percent = if total == 0 {
                100
            } else {
                70 + ((processed * 30) / total) as u32
            };
            let _ = progress_app.emit(
                "preset-clone-progress",
                PresetCloneProgress {
                    request_id: progress_request_id.clone(),
                    processed,
                    total,
                    percent,
                    stage: "ai".into(),
                },
            );
        })
        .await
        {
            Ok(decisions) => {
                apply_ai_decisions(&mut preview.items, decisions);
                preview.matched_count = preview.items.len() as i64;
                preview.ai_enhanced = true;
                preview.ai_message = format!(
                    "AI 已复核 {} 个本地匹配，保留 {} 个",
                    before,
                    preview.items.len()
                );
            }
            Err(error) => {
                preview.ai_message = format!("AI 增强失败，已使用本地结果：{}", error);
            }
        }
    }

    Ok(preview)
}

/// Write the tailored subset into a NEW personal vocab book and return its id.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetImportResult {
    pub book_id: i64,
    #[serde(flatten)]
    pub counts: crate::commands::vocab_word::ImportResult,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetImportProgress {
    pub request_id: String,
    pub processed: u32,
    pub total: u32,
    pub percent: u32,
    pub stage: String,
}

pub(crate) fn import_loaded_preset(
    db: &mut rusqlite::Connection,
    preset_id: i64,
    name: Option<&str>,
    progress: Option<&dyn Fn(u32, u32)>,
) -> Result<PresetImportResult, String> {
    let (preset_name, key): (String, String) = db
        .query_row(
            "SELECT name,preset_key FROM vocab_book WHERE id=?1 AND is_preset=1",
            [preset_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let words = crate::user_vocab::load_words(db, preset_id)?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let name = name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(&preset_name);
    tx.execute(
        "INSERT INTO vocab_book(name,description,cloned_from_preset_key) VALUES (?1,?2,?3)",
        params![name.trim(), format!("从「{}」整套导入", preset_name), key],
    )
    .map_err(|e| e.to_string())?;
    let book_id = tx.last_insert_rowid();
    let mut counts = crate::commands::vocab_word::ImportResult::default();
    for (index, word) in words.iter().enumerate() {
        let sources: String = tx
            .query_row(
                "SELECT source_keys FROM vocab_word WHERE id=?1",
                [word.id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let inserted = crate::user_vocab::insert_word(
            &tx,
            book_id,
            &crate::user_vocab::NewWord {
                word: &word.word,
                definition: &word.definition,
                phonetic: &word.phonetic,
                example: &word.example_sentence,
                memory: &word.memory_tag,
                source_keys: &sources,
                ..crate::user_vocab::NewWord::simple(&word.word, "unknown")
            },
        )?;
        counts.record(&inserted);
        if index % 100 == 0 || index + 1 == words.len() {
            if let Some(progress) = progress {
                progress(index as u32 + 1, words.len() as u32);
            }
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(PresetImportResult { book_id, counts })
}
#[tauri::command]
pub async fn import_preset_vocab_book(
    app: AppHandle,
    preset_key: String,
    new_book_name: Option<String>,
    request_id: Option<String>,
) -> Result<PresetImportResult, String> {
    tokio::task::spawn_blocking(move || {
        let state = app.state::<DbState>();
        let catalog = app.state::<crate::preset_catalog::PresetCatalog>();
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        let request_id = request_id.unwrap_or_default();
        let report = |stage: &str, current: u32, total: u32| {
            let part = if total == 0 {
                100
            } else {
                current * 100 / total
            };
            let percent = if stage == "loading" {
                part * 40 / 100
            } else {
                40 + part * 60 / 100
            };
            let _ = app.emit(
                "preset-import-progress",
                PresetImportProgress {
                    request_id: request_id.clone(),
                    processed: current,
                    total,
                    percent,
                    stage: stage.into(),
                },
            );
        };
        let id = catalog.ensure_loaded(
            &mut db,
            &preset_key,
            Some(&|current, total| report("loading", current, total)),
        )?;
        report("importing", 0, 1);
        import_loaded_preset(
            &mut db,
            id,
            new_book_name.as_deref(),
            Some(&|current, total| report("importing", current, total)),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

pub(crate) fn commit_items(
    db: &mut rusqlite::Connection,
    preset_id: i64,
    novel_id: i64,
    name: Option<&str>,
    items: &[PresetCloneItem],
) -> Result<PresetImportResult, String> {
    if items.is_empty() {
        return Err("没有匹配的单词，无法导入".into());
    }
    let (preset_name, key): (String, String) = db
        .query_row(
            "SELECT name,preset_key FROM vocab_book WHERE id=?1 AND is_preset=1",
            [preset_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let (novel_title,novel_text):(String,String)=db.query_row("SELECT title,CASE WHEN cleaned_text<>'' THEN cleaned_text ELSE raw_text END FROM novel WHERE id=?1",[novel_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|e|e.to_string())?;
    let source_words: HashMap<_, _> = crate::user_vocab::load_words(db, preset_id)?
        .into_iter()
        .map(|w| (crate::user_vocab::word_key(&w.word), w))
        .collect();
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let default_name = format!("{} · {}精选", preset_name, novel_title);
    let name = name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&default_name);
    tx.execute(
        "INSERT INTO vocab_book(name,description,cloned_from_preset_key) VALUES (?1,?2,?3)",
        params![
            name.trim(),
            format!("从「{}」按当前小说裁剪", preset_name),
            key
        ],
    )
    .map_err(|e| e.to_string())?;
    let book_id = tx.last_insert_rowid();
    let mut counts = crate::commands::vocab_word::ImportResult::default();
    for item in items {
        let source = source_words
            .get(&crate::user_vocab::word_key(&item.word))
            .ok_or_else(|| format!("{} 不属于当前预设词表", item.word))?;
        let primary = source
            .definition
            .split('【')
            .next()
            .unwrap_or(&source.definition);
        let terms: Vec<_> = extract_cjk_terms(
            item.definition
                .split('【')
                .next()
                .unwrap_or(&item.definition),
        )
        .into_iter()
        .filter(|term| primary.contains(term) && novel_text.contains(term))
        .collect();
        if terms.is_empty() {
            return Err(format!("{} 的匹配依据已失效，请重新计算", item.word));
        }
        let sources: String = tx
            .query_row(
                "SELECT source_keys FROM vocab_word WHERE id=?1",
                [source.id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let terms = serde_json::to_string(&terms).map_err(|e| e.to_string())?;
        let inserted = crate::user_vocab::insert_word(
            &tx,
            book_id,
            &crate::user_vocab::NewWord {
                word: &source.word,
                definition: &item.definition,
                phonetic: &item.phonetic,
                example: &item.example_sentence,
                novel_id: Some(novel_id),
                match_terms: &terms,
                source_keys: &sources,
                ..crate::user_vocab::NewWord::simple(&source.word, "unknown")
            },
        )?;
        counts.record(&inserted);
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(PresetImportResult { book_id, counts })
}
#[tauri::command]
pub async fn commit_preset_clone_with_state(
    app: AppHandle,
    preset_key: String,
    novel_id: i64,
    new_book_name: Option<String>,
    items: Vec<PresetCloneItem>,
) -> Result<PresetImportResult, String> {
    tokio::task::spawn_blocking(move || {
        let state = app.state::<DbState>();
        let catalog = app.state::<crate::preset_catalog::PresetCatalog>();
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        let id = catalog.ensure_loaded(&mut db, &preset_key, None)?;
        commit_items(&mut db, id, novel_id, new_book_name.as_deref(), &items)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn commit_preset_clone(
    app: AppHandle,
    preset_key: String,
    novel_id: i64,
    new_book_name: Option<String>,
    items: Vec<PresetCloneItem>,
) -> Result<i64, String> {
    Ok(
        commit_preset_clone_with_state(app, preset_key, novel_id, new_book_name, items)
            .await?
            .book_id,
    )
}
fn load_preset_and_novel(
    state: &State<'_, DbState>,
    preset_id: i64,
    novel_id: i64,
) -> Result<(i64, Vec<VocabWord>, String), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let words = crate::user_vocab::load_words(&db, preset_id)?;
    let text=db.query_row("SELECT CASE WHEN cleaned_text<>'' THEN cleaned_text ELSE raw_text END FROM novel WHERE id=?1",[novel_id],|r|r.get(0)).map_err(|e|e.to_string())?;
    Ok((preset_id, words, text))
}
