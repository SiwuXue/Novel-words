use crate::models::novel::Chapter;

/// Detect chapters from cleaned text using common Chinese novel chapter patterns.
/// Returns chapters with their titles, content, and byte offsets.
pub fn detect_chapters(text: &str) -> Vec<Chapter> {
    let patterns = [
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
        let is_heading = patterns.iter().any(|pat| trimmed.starts_with(pat))
            && trimmed.chars().count() <= 30; // Chapter headings are short

        if is_heading {
            let title = trimmed.to_string();

            // If we already found chapters, save the previous one's content
            if found_first {
                let content = text[last_pos..line_start].trim().to_string();
                if !content.is_empty() {
                    chapters.push(Chapter {
                        title: std::mem::take(&mut last_title),
                        content,
                        start_index: last_pos,
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
                title: last_title,
                content,
                start_index: last_pos,
            });
        }
    } else {
        // No chapters detected — treat the whole text as one chapter
        if !text.trim().is_empty() {
            chapters.push(Chapter {
                title: "全文".to_string(),
                content: text.trim().to_string(),
                start_index: 0,
            });
        }
    }

    chapters
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
