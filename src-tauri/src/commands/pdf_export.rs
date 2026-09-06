use crate::db::DbState;
use crate::models::novel::{Chapter, Novel};
use crate::models::pdf_export_response::PdfExportResponse;
use crate::models::pdf_template::PdfTemplate;
use crate::models::vocab_word::VocabWord;
use crate::pdf;
use crate::pdf::matcher::{words_found_in_text, words_found_in_text_en};
use crate::pdf::{parse_steps_from_db, IntensiveSteps};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_fs::FsExt;

fn steps_label(steps: IntensiveSteps) -> String {
    let mut parts = Vec::new();
    if steps.step1 { parts.push("Step 1") }
    if steps.step2 { parts.push("Step 2") }
    if steps.step3 { parts.push("Step 3") }
    parts.join(" + ")
}

#[tauri::command]
pub async fn export_pdf(
    app: AppHandle,
    state: State<'_, DbState>,
    novel_id: i64,
    _template_id: Option<i64>,
    template_type: Option<String>,
    vocab_book_id: Option<i64>,
    steps: Option<Vec<i64>>,
    cover: Option<bool>,
    page_numbers: Option<bool>,
    output_path: String,
) -> Result<PdfExportResponse, String> {
    let template_type = template_type.unwrap_or_else(|| "intensive".to_string());
    let cover = cover.unwrap_or(false);
    let page_numbers = page_numbers.unwrap_or(false);
    // ---- Phase 1: read all data from SQLite (fast, hold the lock only briefly) ----
    let (novel, template, vocabs, chapters, steps, background) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        // Load novel
        let novel = db
            .query_row(
                "SELECT id, title, author, category, raw_text, cleaned_text, is_favorite, language, created_at, updated_at
                 FROM novel WHERE id = ?1",
                rusqlite::params![novel_id],
                |row| {
                    Ok(Novel {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        author: row.get(2)?,
                        category: row.get(3)?,
                        raw_text: row.get(4)?,
                        cleaned_text: row.get(5)?,
                        is_favorite: row.get(6)?,
                        language: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                },
            )
            .map_err(|e| format!("查询小说失败: {}", e))?;

        // Use the built-in template matching the requested type (intensive reading
        // by default). Templates are currently defined in code; persistence (the
        // `pdf_template` table) is not yet wired into the export path.
        let template = template_for(&template_type);

        // Load vocab words if a book is selected
        let vocabs: Vec<VocabWord> = if let Some(book_id) = vocab_book_id {
            let mut stmt = db
                .prepare(
                    "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at, match_terms
                     FROM vocab_word WHERE vocab_book_id = ?1",
                )
                .map_err(|e| format!("查询生词失败: {}", e))?;
            let rows = stmt
                .query_map(rusqlite::params![book_id], |row| {
                    Ok(VocabWord {
                        id: row.get(0)?,
                        vocab_book_id: row.get(1)?,
                        word: row.get(2)?,
                        definition: row.get(3)?,
                        phonetic: row.get(4)?,
                        example_sentence: row.get(5)?,
                        novel_id: row.get(6)?,
                        chapter_id: None,
                        proficiency: row.get(7)?,
                        memory_tag: row.get(8)?,
                        created_at: row.get(9)?,
                        match_terms: row.get(10)?,
                    })
                })
                .map_err(|e| format!("查询生词失败: {}", e))?;
            rows.filter_map(|r| r.ok()).collect()
        } else {
            Vec::new()
        };

        // Load chapters (fallback to full-text if none in DB)
        let chapters: Vec<Chapter> = {
            let mut stmt = db
                .prepare(
                    "SELECT id, novel_id, title, content, sort_order, created_at FROM chapter WHERE novel_id = ?1 ORDER BY sort_order",
                )
                .map_err(|e| format!("查询章节失败: {}", e))?;
            let rows: Vec<Chapter> = stmt
                .query_map(rusqlite::params![novel_id], |row| {
                    Ok(Chapter {
                        id: row.get(0)?,
                        novel_id: row.get(1)?,
                        title: row.get(2)?,
                        content: row.get(3)?,
                        sort_order: row.get(4)?,
                        start_index: 0,
                        created_at: row.get(5)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            if rows.is_empty() {
                // Fallback: one chapter with full novel text
                let text = if !novel.cleaned_text.is_empty() {
                    &novel.cleaned_text
                } else {
                    &novel.raw_text
                };
                vec![Chapter {
                    id: 0,
                    novel_id,
                    title: "全文".into(),
                    content: text.clone(),
                    sort_order: 0,
                    start_index: 0,
                    created_at: String::new(),
                }]
            } else {
                rows
            }
        };

        // ===== Resolve steps (frontend override → DB default → all enabled) =====
        let steps = match steps {
            Some(arr) => IntensiveSteps {
                step1: arr.contains(&1),
                step2: arr.contains(&2),
                step3: arr.contains(&3),
            }
            .normalize(),
            None => {
                let db_val: Result<String, _> = db.query_row(
                    "SELECT value FROM app_settings WHERE key='pdf_intensive_steps'",
                    [],
                    |row| row.get(0),
                );
                parse_steps_from_db(db_val.ok().as_deref())
            }
        };

        // ===== PDF page background style (grid / dots / none) =====
        let bg_val: Result<String, _> = db.query_row(
            "SELECT value FROM app_settings WHERE key='pdf_background'",
            [],
            |row| row.get(0),
        );
        let background = match bg_val.ok().as_deref() {
            Some("dots") => "dots".to_string(),
            Some("none") => "none".to_string(),
            _ => "grid".to_string(),
        };

        (novel, template, vocabs, chapters, steps, background)
    };

    // The word-card template highlights English words found verbatim in the body,
    // so it only works for English novels. Reject Chinese novels with a clear hint.
    if template.template_type == "card" && novel.language != "en" {
        return Err("单词卡片版仅支持英文小说，请先切换到「英文模式」再导出。".to_string());
    }

    // ---- Phase 2: heavy work off the main thread, so progress events are
    // delivered live to the webview while the PDF is being generated. ----
    let inner = tokio::task::spawn_blocking(move || -> Result<PdfExportResponse, String> {
        let _ = app.emit(
            "pdf-export-progress",
            crate::pdf::PdfProgress {
                percent: 0,
                message: "正在统计词汇匹配…".to_string(),
            },
        );

        // ===== Compute coverage stats before generating =====
        let total_vocab = vocabs.len();
        let chapter_count = chapters.len();
        let matched_words: usize = {
            let mut all_found = std::collections::HashSet::new();
            let is_en = novel.language == "en";
            for ch in &chapters {
                let found: Vec<&VocabWord> = if is_en {
                    words_found_in_text_en(&ch.content, &vocabs)
                } else {
                    words_found_in_text(&ch.content, &vocabs, &novel.language)
                };
                for w in found {
                    all_found.insert(w.id);
                }
            }
            all_found.len()
        };

        let steps_str = steps_label(steps);
        // Emit a pre-generation progress event so the UI can show the matched
        // word count (helpful for long novels).
        let _ = app.emit(
            "pdf-export-progress",
            crate::pdf::PdfProgress {
                percent: 5,
                message: format!(
                    "已匹配 {} 词（{} 本章节，待排版……）",
                    matched_words, chapters.len()
                ),
            },
        );
        let progress = |p: crate::pdf::PdfProgress| {
            let _ = app.emit("pdf-export-progress", p);
        };
        let pdf_bytes = pdf::generate_pdf(
            &novel,
            &template,
            &vocabs,
            &chapters,
            steps,
            &background,
            cover,
            page_numbers,
            Some(&progress),
        )?;
        let mut options = tauri_plugin_fs::OpenOptions::new();
        options.read(false).write(true).create(true).truncate(true);
        let mut file = app
            .fs()
            .open(output_path.parse::<tauri_plugin_fs::FilePath>().unwrap(), options)
            .map_err(|e| format!("创建 PDF 文件失败: {}", e))?;
        std::io::Write::write_all(&mut file, &pdf_bytes)
            .map_err(|e| format!("写入 PDF 失败: {}", e))?;

        Ok(PdfExportResponse {
            path: output_path,
            total_vocab,
            matched_words,
            chapter_count,
            steps_used: steps_str,
        })
    })
    .await
    .map_err(|e| format!("导出任务执行失败: {}", e))?;

    Ok(inner?)
}

fn default_template() -> PdfTemplate {
    PdfTemplate {
        id: 0,
        name: "默认".to_string(),
        paper_size: "A4".to_string(),
        font_family: "SimSun".to_string(),
        font_size: 14,
        line_spacing: 1.5,
        margins: r#"{"top":25,"bottom":25,"left":20,"right":20}"#.to_string(),
        annotation_mode: "inline".to_string(),
        template_type: "intensive".to_string(),
        is_builtin: false,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

/// Build the built-in template for the requested `template_type`. Unknown types
/// fall back to the default intensive template so existing callers keep working.
fn template_for(template_type: &str) -> PdfTemplate {
    match template_type {
        "card" => PdfTemplate {
            name: "单词卡片版".to_string(),
            template_type: "card".to_string(),
            ..default_template()
        },
        _ => default_template(),
    }
}
