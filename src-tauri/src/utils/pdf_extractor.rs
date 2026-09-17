//! PDF 文本提取：基于 pdf-extract（纯 Rust），供小说导入使用。
//! 扫描版（图片型）PDF 无内嵌文本，提取结果为空时报错提示。

use lopdf::Document;
use std::collections::HashMap;

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

/// 按页提取文本（供书签分章使用）。
pub fn extract_pages(bytes: &[u8]) -> Result<Vec<String>, String> {
    pdf_extract::extract_text_from_mem_by_pages(bytes)
        .map_err(|e| format!("PDF 解析失败: {}", e))
}

/// 提取 PDF 书签（outline），返回 (标题, 起始页索引 0-based)，按书签顺序。
/// 无书签或结构异常返回空表。
pub fn extract_outline(bytes: &[u8]) -> Vec<(String, usize)> {
    let Ok(doc) = Document::load_mem(bytes) else {
        return Vec::new();
    };
    let pages = doc.get_pages(); // 页号(1-based) → ObjectId
    let mut page_index: HashMap<lopdf::ObjectId, usize> = HashMap::new();
    for (num, id) in &pages {
        page_index.insert(*id, (*num - 1) as usize);
    }

    let Ok(catalog) = doc.catalog() else {
        return Vec::new();
    };
    let Ok(outlines_obj) = catalog.get(b"Outlines") else {
        return Vec::new();
    };
    let Ok(outlines) = outlines_obj.as_dict() else {
        return Vec::new();
    };

    let mut items: Vec<(String, usize)> = Vec::new();
    let mut cur = outlines
        .get(b"First")
        .ok()
        .and_then(|o| o.as_reference().ok());
    let mut guard = 0;
    while let Some(node_id) = cur {
        guard += 1;
        if guard > 2000 {
            break;
        }
        let Ok(node) = doc.get_object(node_id) else { break };
        let Ok(dict) = node.as_dict() else { break };

        let title = dict
            .get(b"Title")
            .ok()
            .and_then(|t| t.as_string().ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        // 目标页：/Dest 数组或 /A 动作里的 /D 数组，首元素为页对象引用
        let mut dest_obj: Option<&lopdf::Object> = dict.get(b"Dest").ok();
        if dest_obj.is_none() {
            if let Ok(action) = dict.get(b"A").and_then(|a| a.as_dict()) {
                dest_obj = action.get(b"D").ok();
            }
        }
        let page = dest_obj
            .and_then(|d| doc.dereference(d).ok())
            .and_then(|(_, resolved)| resolved.as_array().ok())
            .and_then(|arr| arr.first())
            .and_then(|first| first.as_reference().ok())
            .and_then(|page_id| page_index.get(&page_id).copied());

        if !title.is_empty() {
            if let Some(p) = page {
                items.push((title, p));
            }
        }
        cur = dict.get(b"Next").ok().and_then(|o| o.as_reference().ok());
    }
    items
}

/// 按书签切章：书签 ≥ 2 且页号非降序时有效。
/// 返回 (标题, 拼接正文) 列表；首个书签之前的内容归"前言"。
/// 同时清理跨页重复的页眉/页脚行与每页首尾的纯页码行。
pub fn split_by_outline(
    bytes: &[u8],
) -> Result<Option<Vec<(String, String)>>, String> {
    let bookmarks = extract_outline(bytes);
    if bookmarks.len() < 2 {
        return Ok(None);
    }
    let mut pages = extract_pages(bytes)?;
    if pages.is_empty() {
        return Ok(None);
    }

    // 页数足够时清理跨页重复行（页眉）与每页首尾纯数字行（页码）
    if pages.len() >= 6 {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for page in &pages {
            let mut seen = std::collections::HashSet::new();
            for line in page.lines() {
                let key = collapse_ws(line);
                if key.len() >= 6 && seen.insert(key.clone()) {
                    *counts.entry(key).or_insert(0) += 1;
                }
            }
        }
        let threshold = (pages.len() * 2) / 5; // 40%
        let repeated: std::collections::HashSet<String> = counts
            .into_iter()
            .filter(|(_, n)| *n >= threshold)
            .map(|(k, _)| k)
            .collect();
        if !repeated.is_empty() {
            for page in &mut pages {
                *page = page
                    .lines()
                    .filter(|l| !repeated.contains(&collapse_ws(l)))
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        }
        for page in &mut pages {
            *page = strip_page_number_lines(page);
        }
    }

    // 书签页号需非降序；同页重复书签取首个
    let mut marks: Vec<(String, usize)> = Vec::new();
    let mut last_page = usize::MAX;
    for (title, page) in bookmarks {
        if page >= pages.len() {
            continue;
        }
        if page != last_page || marks.is_empty() {
            if let Some((_, prev)) = marks.last() {
                if page < *prev {
                    continue;
                }
            }
            marks.push((title, page));
            last_page = page;
        }
    }
    if marks.len() < 2 {
        return Ok(None);
    }

    let mut chapters: Vec<(String, String)> = Vec::new();
    let first_page = marks[0].1;
    if first_page > 0 {
        let pre = pages[..first_page].join("\n\n");
        let pre = pre.trim();
        if !pre.is_empty() {
            chapters.push(("前言".to_string(), pre.to_string()));
        }
    }
    for (i, (title, start)) in marks.iter().enumerate() {
        let end = marks.get(i + 1).map(|(_, p)| *p).unwrap_or(pages.len());
        let content = pages[*start..end].join("\n\n");
        chapters.push((title.clone(), content.trim().to_string()));
    }
    // 至少两章且各章有内容才算有效
    let valid = chapters.len() >= 2 && chapters.iter().all(|(_, c)| !c.is_empty());
    if valid {
        Ok(Some(chapters))
    } else {
        Ok(None)
    }
}

fn collapse_ws(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 移除每页首/尾的纯页码行（≤4 位数字的独立行）
fn strip_page_number_lines(page: &str) -> String {
    let mut lines: Vec<&str> = page.lines().collect();
    while let Some(first) = lines.first() {
        let t = first.trim();
        if !t.is_empty() && t.len() <= 4 && t.chars().all(|c| c.is_ascii_digit()) {
            lines.remove(0);
        } else {
            break;
        }
    }
    while let Some(last) = lines.last() {
        let t = last.trim();
        if !t.is_empty() && t.len() <= 4 && t.chars().all(|c| c.is_ascii_digit()) {
            lines.pop();
        } else {
            break;
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用本机真实 PDF 做端到端验证（文件存在时才运行）。
    const REAL_PDF: &str = "E:/Dsektop/老人与海（英文版）.pdf";

    #[test]
    fn dump_outline_of_real_pdf() {
        if !std::path::Path::new(REAL_PDF).exists() {
            return;
        }
        let bytes = std::fs::read(REAL_PDF).unwrap();
        let items = extract_outline(&bytes);
        println!("书签数: {}", items.len());
        for (title, page) in &items {
            println!("  {:?} @ page {}", title, page);
        }
        let chapters = split_by_outline(&bytes).unwrap();
        match chapters {
            Some(list) => {
                println!("按书签切出 {} 章:", list.len());
                for (title, content) in &list {
                    println!("  {:?} — {} 字符", title, content.len());
                }
            }
            None => println!("书签切章不可用"),
        }
    }

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
