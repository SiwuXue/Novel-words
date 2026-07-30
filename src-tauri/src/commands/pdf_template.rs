use crate::db::DbState;
use crate::models::PdfTemplate;
use tauri::State;

#[tauri::command]
pub fn create_pdf_template(
    state: State<DbState>,
    name: String,
    paper_size: String,
    font_family: String,
    font_size: i32,
    line_spacing: f64,
    margins: String,
    annotation_mode: String,
    template_type: String,
    is_builtin: bool,
) -> Result<PdfTemplate, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO pdf_template (name, paper_size, font_family, font_size, line_spacing, margins, annotation_mode, template_type, is_builtin) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![name, paper_size, font_family, font_size, line_spacing, margins, annotation_mode, template_type, is_builtin as i32],
    )
    .map_err(|e| format!("创建 PDF 模板失败: {}", e))?;

    let id = db.last_insert_rowid();
    get_pdf_template_by_id(&db, id)
}

#[tauri::command]
pub fn get_all_pdf_templates(state: State<DbState>) -> Result<Vec<PdfTemplate>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, name, paper_size, font_family, font_size, line_spacing, margins, annotation_mode, template_type, is_builtin, created_at, updated_at FROM pdf_template ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let templates = stmt
        .query_map([], row_to_pdf_template)
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(templates)
}

#[tauri::command]
pub fn update_pdf_template(
    state: State<DbState>,
    id: i64,
    name: String,
    paper_size: String,
    font_family: String,
    font_size: i32,
    line_spacing: f64,
    margins: String,
    annotation_mode: String,
    template_type: String,
    is_builtin: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let affected = db
        .execute(
            "UPDATE pdf_template SET name=?1, paper_size=?2, font_family=?3, font_size=?4, line_spacing=?5, margins=?6, annotation_mode=?7, template_type=?8, is_builtin=?9, updated_at=datetime('now','localtime') WHERE id=?10",
            rusqlite::params![name, paper_size, font_family, font_size, line_spacing, margins, annotation_mode, template_type, is_builtin as i32, id],
        )
        .map_err(|e| format!("更新 PDF 模板失败: {}", e))?;

    if affected == 0 {
        return Err("PDF 模板不存在".into());
    }
    Ok(())
}

#[tauri::command]
pub fn delete_pdf_template(state: State<DbState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let affected = db
        .execute(
            "DELETE FROM pdf_template WHERE id=?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("删除 PDF 模板失败: {}", e))?;

    if affected == 0 {
        return Err("PDF 模板不存在".into());
    }
    Ok(())
}

fn get_pdf_template_by_id(db: &rusqlite::Connection, id: i64) -> Result<PdfTemplate, String> {
    db.query_row(
        "SELECT id, name, paper_size, font_family, font_size, line_spacing, margins, annotation_mode, template_type, is_builtin, created_at, updated_at FROM pdf_template WHERE id=?1",
        rusqlite::params![id],
        row_to_pdf_template,
    )
    .map_err(|e| format!("未找到该 PDF 模板: {}", e))
}

fn row_to_pdf_template(row: &rusqlite::Row) -> rusqlite::Result<PdfTemplate> {
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
}

/// Return hardcoded built-in templates. Not stored in DB.
#[tauri::command]
pub fn get_builtin_templates() -> Result<Vec<PdfTemplate>, String> {
    Ok(builtin_templates())
}

fn builtin_templates() -> Vec<PdfTemplate> {
    vec![
        PdfTemplate {
            id: -1,
            name: "精读版".into(),
            paper_size: "A4".into(),
            font_family: "SimSun".into(),
            font_size: 14,
            line_spacing: 1.8,
            margins: r#"{"top":25,"bottom":25,"left":20,"right":20}"#.into(),
            annotation_mode: "inline".into(),
            template_type: "intensive".into(),
            is_builtin: true,
            created_at: String::new(),
            updated_at: String::new(),
        },
        PdfTemplate {
            id: -2,
            name: "侧边注释版".into(),
            paper_size: "A4".into(),
            font_family: "SimSun".into(),
            font_size: 12,
            line_spacing: 1.5,
            margins: r#"{"top":25,"bottom":25,"left":20,"right":20}"#.into(),
            annotation_mode: "sidebar".into(),
            template_type: "sidebar".into(),
            is_builtin: true,
            created_at: String::new(),
            updated_at: String::new(),
        },
        PdfTemplate {
            id: -3,
            name: "背诵专用版".into(),
            paper_size: "A4".into(),
            font_family: "SimSun".into(),
            font_size: 11,
            line_spacing: 1.3,
            margins: r#"{"top":25,"bottom":25,"left":20,"right":20}"#.into(),
            annotation_mode: "none".into(),
            template_type: "recitation".into(),
            is_builtin: true,
            created_at: String::new(),
            updated_at: String::new(),
        },
        PdfTemplate {
            id: -4,
            name: "默写空白版".into(),
            paper_size: "A4".into(),
            font_family: "SimSun".into(),
            font_size: 14,
            line_spacing: 2.0,
            margins: r#"{"top":25,"bottom":25,"left":20,"right":20}"#.into(),
            annotation_mode: "none".into(),
            template_type: "dictation".into(),
            is_builtin: true,
            created_at: String::new(),
            updated_at: String::new(),
        },
    ]
}
