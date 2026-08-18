use crate::db::DbState;
use crate::models::novel::{Chapter, Novel};
use crate::models::pdf_export_response::PdfExportResponse;
use crate::models::pdf_template::PdfTemplate;
use crate::models::vocab_word::VocabWord;
use crate::pdf;
use crate::pdf::matcher::words_found_in_text;
use crate::pdf::{parse_steps_from_db, IntensiveSteps};
use tauri::State;

fn steps_label(steps: IntensiveSteps) -> String {
    let mut parts = Vec::new();
    if steps.step1 { parts.push("Step 1") }
    if steps.step2 { parts.push("Step 2") }
    if steps.step3 { parts.push("Step 3") }
    parts.join(" + ")
}

#[tauri::command]
pub fn export_pdf(
    state: State<DbState>,
    novel_id: i64,
    _template_id: Option<i64>,
    _template_type: Option<String>,
    vocab_book_id: Option<i64>,
    steps: Option<Vec<i64>>,
    output_path: String,
) -> Result<PdfExportResponse, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Load novel
    let novel = db
        .query_row(
            "SELECT id, title, author, category, raw_text, cleaned_text, is_favorite, created_at, updated_at
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
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )
        .map_err(|e| format!("查询小说失败: {}", e))?;

    // Use default template (intensive reading only)
    let template = default_template();

    // Load vocab words if a book is selected
    let vocabs: Vec<VocabWord> = if let Some(book_id) = vocab_book_id {
        let mut stmt = db
            .prepare(
                "SELECT id, vocab_book_id, word, definition, phonetic, example_sentence, novel_id, proficiency, memory_tag, created_at
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

    drop(db);

    // ===== Compute coverage stats before generating =====
    let total_vocab = vocabs.len();
    let chapter_count = chapters.len();
    let matched_words: usize = {
        let mut all_found = std::collections::HashSet::new();
        for ch in &chapters {
            for w in words_found_in_text(&ch.content, &vocabs) {
                all_found.insert(w.id);
            }
        }
        all_found.len()
    };

    let steps_str = steps_label(steps);
    pdf::generate_pdf(&novel, &template, &vocabs, &chapters, steps, &output_path)?;

    Ok(PdfExportResponse {
        path: output_path,
        total_vocab,
        matched_words,
        chapter_count,
        steps_used: steps_str,
    })
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
