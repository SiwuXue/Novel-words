use std::path::Path;

use crate::models::novel::{Chapter, ImportResult};
use crate::utils::{chapter_detector, ebook, text_cleaner};
use tauri::{AppHandle, Emitter};

/// Progress payload emitted to the frontend during file import.
#[derive(Clone, serde::Serialize)]
pub struct ImportProgress {
    pub percent: u32,
    pub message: String,
}

/// Detect encoding from raw bytes. Tries UTF-8 first, then GBK.
/// Returns the decoded String.
fn detect_and_decode(bytes: &[u8]) -> Result<String, String> {
    // Check BOM
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        // UTF-8 BOM — skip it
        return std::str::from_utf8(&bytes[3..])
            .map(|s| s.to_string())
            .map_err(|e| format!("无效的 UTF-8 (含 BOM): {}", e));
    }

    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        // UTF-16 LE BOM
        let u16s: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16(&u16s)
            .map_err(|e| format!("无效的 UTF-16 LE: {}", e));
    }

    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16 BE BOM
        let u16s: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16(&u16s)
            .map_err(|e| format!("无效的 UTF-16 BE: {}", e));
    }

    // Try UTF-8 without BOM (borrows, doesn't consume)
    if let Ok(s) = std::str::from_utf8(bytes) {
        return Ok(s.to_string());
    }

    // Fallback: decode as GBK/GB18030
    let (cow, _encoding, had_errors) = encoding_rs::GBK.decode(bytes);
    if had_errors {
        // Last resort: try GB18030 (superset of GBK)
        let (cow2, _enc2, had_errors2) = encoding_rs::GB18030.decode(bytes);
        if had_errors2 {
            return Err(format!(
                "无法识别文件编码（已尝试 UTF-8、GBK、GB18030），请确认文件为常见中文编码"
            ));
        }
        return Ok(cow2.into_owned());
    }
    Ok(cow.into_owned())
}

/// Extract a filename-based fallback title.
fn filename_title(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未命名")
        .to_string()
}

fn import_text_file_sync(path: &str, progress: &dyn Fn(u32, &str)) -> Result<ImportResult, String> {
    progress(20, "正在读取文件…");
    let bytes =
        std::fs::read(path).map_err(|e| format!("无法读取文件: {}", e))?;

    if bytes.is_empty() {
        return Err("文件为空".into());
    }

    // 1. Detect encoding and decode
    progress(40, "正在识别编码…");
    let raw_text = detect_and_decode(&bytes)?;

    // 2. Clean text (remove ads, normalize whitespace, strip special chars)
    progress(60, "正在清洗文本…");
    let cleaned_text = text_cleaner::clean_text(&raw_text);

    // 3. Detect title — try first meaningful line, fallback to filename
    progress(80, "正在划分章节…");
    let detected_title = chapter_detector::detect_title_from_text(&cleaned_text);
    let detected_title = if detected_title.is_empty() {
        filename_title(path)
    } else {
        detected_title
    };

    // 4. Split into chapters
    let chapters = chapter_detector::detect_chapters(&cleaned_text);

    progress(98, "解析完成");
    Ok(ImportResult {
        chapters,
        raw_text,
        cleaned_text,
        detected_title,
    })
}

/// Parse an EPUB / FB2 file into the standard ImportResult shape.
fn import_ebook_sync(path: &str, progress: &dyn Fn(u32, &str)) -> Result<ImportResult, String> {
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    progress(30, "正在解析电子书（解压与 HTML 解析）…");
    let ebook::EbookResult { title, chapters } = if ext == "fb2" {
        ebook::parse_fb2(path)?
    } else {
        ebook::parse_epub(path)?
    };

    progress(80, "正在整理全文…");
    let detected_title = if title.is_empty() {
        filename_title(path)
    } else {
        title
    };

    let full = ebook::full_text(&chapters);
    let chapters: Vec<Chapter> = chapters
        .into_iter()
        .enumerate()
        .map(|(i, (t, c))| Chapter {
            id: 0,
            novel_id: 0,
            title: t,
            content: c,
            sort_order: i as i32,
            start_index: 0,
            created_at: String::new(),
        })
        .collect();

    progress(98, "解析完成");
    Ok(ImportResult {
        chapters,
        raw_text: full.clone(),
        cleaned_text: full,
        detected_title,
    })
}

/// Import a novel file, dispatching on extension: `.epub`/`.fb2` use the ebook
/// parsers, everything else goes through the plain-text pipeline.
#[tauri::command]
pub async fn import_file(app: AppHandle, path: String) -> Result<ImportResult, String> {
    let ext = Path::new(&path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Own the AppHandle (clone) inside the closure so it can be moved into
    // `spawn_blocking` (needs 'static).
    let emit = {
        let app = app.clone();
        move |percent: u32, message: &str| {
            let _ = app.emit(
                "import-progress",
                ImportProgress {
                    percent,
                    message: message.to_string(),
                },
            );
        }
    };
    emit(5, "正在读取文件…");

    let result = if ext == "epub" || ext == "fb2" {
        let inner = emit.clone();
        tokio::task::spawn_blocking(move || import_ebook_sync(&path, &inner))
            .await
            .map_err(|e| format!("任务执行失败: {}", e))?
    } else {
        let inner = emit.clone();
        tokio::task::spawn_blocking(move || import_text_file_sync(&path, &inner))
            .await
            .map_err(|e| format!("任务执行失败: {}", e))?
    };

    emit(100, "解析完成");
    result
}

/// Write a UTF-8 text file (used for lightweight JSON export).
#[tauri::command]
pub fn write_text_file(path: String, contents: String) -> Result<(), String> {
    std::fs::write(&path, contents).map_err(|e| format!("写入文件失败: {}", e))
}

/// Read a UTF-8 text file (used for lightweight JSON import).
#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("读取文件失败: {}", e))
}
