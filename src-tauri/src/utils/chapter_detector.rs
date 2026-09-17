use crate::models::novel::Chapter;

/// 常见章节标题前缀（"第X章"、"Chapter N"、楔子/序章等）。
const HEADING_PATTERNS: &[&str] = &[
    // "第X章" — most common. X can be Arabic digits or Chinese numerals.
    "第",           // "第X章", "第X节", "第X回", "第X卷"
    "Chapter ",     // English chapter marker
    "CHAPTER ",     // Uppercase
    "chaper ",      // Common OCR typo
    // Special chapter types
    "楔子",         // Prologue
    "序章",         // Preface chapter
    "序言",         // Preface
    "终章",         // Final chapter
    "尾声",         // Epilogue
    "后记",         // Afterword
    "番外",         // Extra/Side story
    "番外篇",
    "尾声·",
    "卷",           // Volume marker "卷X"
];

fn is_pattern_heading(trimmed: &str) -> bool {
    HEADING_PATTERNS.iter().any(|pat| trimmed.starts_with(pat))
        && trimmed.chars().count() <= 30
}

/// Detect chapters from cleaned text.
/// 优先尝试目录（CONTENTS / 目录）驱动的识别（适合无 "Chapter N" 前缀、
/// 仅以标题行分章的英文小说），失败时回退到前缀模式匹配。
/// Returns chapters with their titles, content, and byte offsets.
pub fn detect_chapters(text: &str) -> Vec<Chapter> {
    if let Some(chapters) = detect_chapters_via_toc(text) {
        return chapters;
    }

    let mut chapters: Vec<Chapter> = Vec::new();
    let mut last_pos = 0usize;
    let mut last_title = String::new();
    let mut found_first = false;

    for (line_start, line) in line_starts(text) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Check if this line is a chapter heading
        let is_heading = is_pattern_heading(trimmed);

        if is_heading {
            let title = trimmed.to_string();

            // If we already found chapters, save the previous one's content
            if found_first {
                let content = text[last_pos..line_start].trim().to_string();
                if !content.is_empty() {
                    chapters.push(Chapter {
                        id: 0,
                        novel_id: 0,
                        title: std::mem::take(&mut last_title),
                        content,
                        sort_order: chapters.len() as i32,
                        start_index: last_pos,
                        created_at: String::new(),
                    });
                }
            } else {
                // Everything before the first chapter heading is preamble
                found_first = true;
            }

            last_title = title;
            // Skip past the heading line so chapter content doesn't include it
            last_pos = line_start + line.len();
        }
    }

    // Don't forget the last chapter (or the entire text if no headings found)
    if found_first {
        let content = text[last_pos..].trim().to_string();
        if !content.is_empty() || !last_title.is_empty() {
            chapters.push(Chapter {
                id: 0,
                novel_id: 0,
                title: last_title,
                content,
                sort_order: chapters.len() as i32,
                start_index: last_pos,
                created_at: String::new(),
            });
        }
    } else {
        // No chapters detected — treat the whole text as one chapter
        if !text.trim().is_empty() {
            chapters.push(Chapter {
                id: 0,
                novel_id: 0,
                title: "全文".to_string(),
                content: text.trim().to_string(),
                sort_order: 0,
                start_index: 0,
                created_at: String::new(),
            });
        }
    }

    chapters
}

/// 目录驱动识别：适配无 "Chapter N" 前缀、以 `CONTENTS`/`目录` + 标题行分章的
/// 英文小说（如 Flipped）。目录首个条目在目录块之后再次出现即为正文起点，
/// 随后按目录顺序逐行匹配切章；任一候选未按序命中则放弃（返回 None 回退）。
fn detect_chapters_via_toc(text: &str) -> Option<Vec<Chapter>> {
    let lines = line_starts(text);

    // 1. 目录标记行（CONTENTS 大小写不敏感 / 目录）
    let marker_idx = lines.iter().position(|(_, line)| {
        let trimmed = line.trim();
        trimmed.eq_ignore_ascii_case("CONTENTS") || trimmed == "目录"
    })?;

    // 2. 收集目录条目：标记行之后的连续短行（允许空行间隔）。
    //    首条目重复出现 → 正文从此开始；长行 / 模式行 / 上限则终止收集。
    let mut candidates: Vec<String> = Vec::new();
    let mut body_start_idx: Option<usize> = None;
    let mut i = marker_idx + 1;
    while i < lines.len() {
        let trimmed = lines[i].1.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        if candidates.len() >= 500 || is_pattern_heading(trimmed) || trimmed.chars().count() > 60 {
            break;
        }
        if !candidates.is_empty() && trimmed == candidates[0] {
            body_start_idx = Some(i);
            break;
        }
        candidates.push(trimmed.to_string());
        i += 1;
    }
    if candidates.len() < 2 {
        return None;
    }
    let body_start_idx = body_start_idx?;

    // 3. 从正文起点按目录顺序切章；全部候选按序命中才算成功
    let mut chapters: Vec<Chapter> = Vec::new();
    // 书名页 / 版权页 / 目录本身 → 前言
    let preamble = text[..lines[body_start_idx].0].trim().to_string();
    if !preamble.is_empty() {
        chapters.push(Chapter {
            id: 0,
            novel_id: 0,
            title: "前言".to_string(),
            content: preamble,
            sort_order: 0,
            start_index: 0,
            created_at: String::new(),
        });
    }
    let mut expected = 0usize;
    let mut last_title = String::new();
    let mut last_pos = 0usize;
    let mut in_body = false;
    for j in body_start_idx..lines.len() {
        let (start, line) = &lines[j];
        let trimmed = line.trim();
        if expected < candidates.len() && trimmed == candidates[expected].as_str() {
            if in_body {
                let content = text[last_pos..*start].trim().to_string();
                chapters.push(Chapter {
                    id: 0,
                    novel_id: 0,
                    title: std::mem::take(&mut last_title),
                    content,
                    sort_order: chapters.len() as i32,
                    start_index: last_pos,
                    created_at: String::new(),
                });
            }
            last_title = trimmed.to_string();
            last_pos = start + line.len();
            in_body = true;
            expected += 1;
        }
    }
    if expected != candidates.len() || !in_body {
        return None;
    }
    chapters.push(Chapter {
        id: 0,
        novel_id: 0,
        title: last_title,
        content: text[last_pos..].trim().to_string(),
        sort_order: chapters.len() as i32,
        start_index: last_pos,
        created_at: String::new(),
    });
    Some(chapters)
}

/// Detect a likely title from the first non-empty line or filename
pub fn detect_title_from_text(text: &str) -> String {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let len = trimmed.chars().count();
            if len <= 50 {
                return trimmed.to_string();
            }
            // Long first line — truncate
            return trimmed.chars().take(50).collect();
        }
    }
    String::new()
}

/// Yield (byte_offset, line_content) pairs for each line in the text.
fn line_starts(text: &str) -> Vec<(usize, &str)> {
    let mut result = Vec::new();
    let mut pos = 0usize;
    let bytes = text.as_bytes();
    for line in text.lines() {
        // Skip \r and \n bytes to land at the start of the next line
        while pos < bytes.len() && (bytes[pos] == b'\n' || bytes[pos] == b'\r') {
            pos += 1;
        }
        result.push((pos, line));
        pos += line.len();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Flipped\n\nby Wendelin Van Draanen\n\nCONTENTS\n\nDiving Under\n\nFlipped\n\nBuddy, Beware!\n\nDiving Under\n\nAll I've ever wanted is for Juli Baker to leave me alone.\n\nFlipped\n\nThe first day I met Juli Baker, I lost my mind.\n\nBuddy, Beware!\n\nThe third chapter body.\n";

    #[test]
    fn toc_driven_detection_splits_named_chapters() {
        let chapters = detect_chapters(SAMPLE);
        let titles: Vec<&str> = chapters.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(titles, vec!["前言", "Diving Under", "Flipped", "Buddy, Beware!"]);
        assert!(chapters[1].content.contains("Juli Baker to leave me alone"));
        assert!(chapters[2].content.contains("lost my mind"));
        assert_eq!(chapters[3].content, "The third chapter body.");
        // 前言包含书名与目录块
        assert!(chapters[0].content.contains("CONTENTS"));
    }

    #[test]
    fn pattern_detection_still_works() {
        let text = "序章\nprologue body\n第一章 开端\nchapter body\n";
        let chapters = detect_chapters(text);
        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].title, "序章");
        assert_eq!(chapters[1].title, "第一章 开端");
    }

    #[test]
    fn no_markers_yields_single_chapter() {
        let chapters = detect_chapters("just some text without chapters");
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, "全文");
    }
}
