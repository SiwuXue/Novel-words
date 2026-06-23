use std::path::Path;

use crate::models::novel::ImportResult;
use crate::utils::{chapter_detector, text_cleaner};

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

fn import_text_file_sync(path: &str) -> Result<ImportResult, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("无法读取文件: {}", e))?;

    if bytes.is_empty() {
        return Err("文件为空".into());
    }

    // 1. Detect encoding and decode
    let raw_text = detect_and_decode(&bytes)?;

    // 2. Clean text (remove ads, normalize whitespace, strip special chars)
    let cleaned_text = text_cleaner::clean_text(&raw_text);

    // 3. Detect title — try first meaningful line, fallback to filename
    let detected_title = chapter_detector::detect_title_from_text(&cleaned_text);
    let detected_title = if detected_title.is_empty() {
        filename_title(path)
    } else {
        detected_title
    };

    // 4. Split into chapters
    let chapters = chapter_detector::detect_chapters(&cleaned_text);

    Ok(ImportResult {
        chapters,
        raw_text,
        cleaned_text,
        detected_title,
    })
}

#[tauri::command]
pub async fn import_text_file(path: String) -> Result<ImportResult, String> {
    tokio::task::spawn_blocking(move || import_text_file_sync(&path))
        .await
        .map_err(|e| format!("任务执行失败: {}", e))?
}
