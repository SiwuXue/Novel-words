//! PDF 文本提取：基于 pdf-extract（纯 Rust），供小说导入使用。
//! 扫描版（图片型）PDF 无内嵌文本，提取结果为空时报错提示。

/// 从 PDF 字节流提取全部文本。失败（加密/损坏/无文本层）返回带原因的 Err。
pub fn extract_text(bytes: &[u8]) -> Result<String, String> {
    let text = pdf_extract::extract_text_from_mem(bytes)
        .map_err(|e| format!("PDF 解析失败: {}（若是扫描版 PDF 则无法提取文字）", e))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("PDF 中没有可提取的文字（可能是扫描版/图片型 PDF）".into());
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用本机真实 PDF 做端到端验证（文件存在时才运行）。
    const REAL_PDF: &str = "E:/Dsektop/老人与海（英文版）.pdf";

    #[test]
    fn real_pdf_full_import_pipeline() {
        if !std::path::Path::new(REAL_PDF).exists() {
            return;
        }
        let bytes = std::fs::read(REAL_PDF).unwrap();
        let raw = extract_text(&bytes).unwrap();
        let cleaned = crate::utils::text_cleaner::clean_text(&raw);
        let chapters = crate::utils::chapter_detector::detect_chapters(&cleaned);
        assert!(!chapters.is_empty());
        for c in &chapters {
            assert!(!c.title.is_empty());
        }
        assert_eq!(
            crate::utils::text_cleaner::detect_language(&cleaned),
            "en",
            "老人与海英文版应判定为英文"
        );
    }

    #[test]
    fn extracts_text_from_real_old_man_and_the_sea() {
        if !std::path::Path::new(REAL_PDF).exists() {
            return;
        }
        let bytes = std::fs::read(REAL_PDF).unwrap();
        let text = extract_text(&bytes).expect("should extract text");
        assert!(text.to_lowercase().contains("old man"), "应包含书名相关词");
        assert!(text.chars().filter(|c| c.is_ascii_alphabetic()).count() > 1000);
        assert_eq!(crate::utils::text_cleaner::detect_language(&text), "en");
    }
}
