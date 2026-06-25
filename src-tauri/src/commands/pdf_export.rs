use crate::db::DbState;
use crate::models::novel::Novel;
use crate::models::pdf_template::PdfTemplate;
use crate::models::vocab_word::VocabWord;
use crate::pdf;
use tauri::State;

#[tauri::command]
pub fn export_pdf(
    state: State<DbState>,
    novel_id: i64,
    template_id: Option<i64>,
    vocab_book_id: Option<i64>,
    output_path: String,
) -> Result<String, String> {
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

    // Load template or use defaults
    let template = if let Some(tid) = template_id {
        db.query_row(
            "SELECT id, name, paper_size, font_family, font_size, line_spacing, margins, annotation_mode, template_type, is_builtin, created_at, updated_at
             FROM pdf_template WHERE id = ?1",
            rusqlite::params![tid],
            |row| {
                Ok(PdfTemplate {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    paper_size: row.get(2)?,
                    font_family: row.get(3)?,
                    font_size: row.get(4)?,
                    line_spacing: row.get(5)?,
                    margins: row.get(6)?,
                    annotation_mode: row.get(7)?,
                    template_type: row.get(8)?,
                    is_builtin: row.get::<_, i32>(9)? != 0,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )
        .unwrap_or_else(|_| default_template())
    } else {
        default_template()
    };

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

    drop(db);

    pdf::generate_pdf(&novel, &template, &vocabs, &output_path)?;

    Ok(output_path)
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
        annotation_mode: "appendix".to_string(),
        template_type: "appendix".to_string(),
        is_builtin: false,
        created_at: String::new(),
        updated_at: String::new(),
    }
}
