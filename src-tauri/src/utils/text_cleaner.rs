/// Clean raw text: remove ad lines, normalize whitespace, strip special chars
pub fn clean_text(raw: &str) -> String {
    let mut cleaned = remove_ad_lines(raw);
    cleaned = normalize_whitespace(&cleaned);
    cleaned = strip_special_chars(&cleaned);
    cleaned
}

/// Remove lines that are likely ads or site watermarks
fn remove_ad_lines(text: &str) -> String {
    let ad_patterns = [
        "请收藏",
        "本章未完",
        "求推荐",
        "求月票",
        "求订阅",
        "求打赏",
        "求收藏",
        "本章完",
        "www.",
        "http://",
        "https://",
        ".com",
        "笔趣阁",
        "顶点小说",
        "请记住",
        "永久免费",
        "最快更新",
        "手机阅读",
        "电脑阅读",
    ];

    text.lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return true; // keep empty lines
            }
            !ad_patterns.iter().any(|pat| trimmed.contains(pat))
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Collapse 3+ blank lines into 2
fn normalize_whitespace(text: &str) -> String {
    // Replace 3+ newlines with 2 newlines
    let result = text
        .lines()
        .fold((Vec::new(), 0u32), |(mut lines, empty_count), line| {
            if line.trim().is_empty() {
                if empty_count < 2 {
                    lines.push("");
                }
                (lines, empty_count + 1)
            } else {
                lines.push(line);
                (lines, 0)
            }
        })
        .0
        .join("\n");
    result
}

/// Strip zero-width characters, BOM, and control chars (except newline/tab)
fn strip_special_chars(text: &str) -> String {
    text.chars()
        .filter(|c| {
            // Keep: newline, tab, ASCII printable, and all Unicode above 0x7F
            // (covers CJK, punctuation, fullwidth forms — everything useful)
            matches!(*c, '\n' | '\r' | '\t' | '\u{0020}'..='\u{007E}' | '\u{0080}'..='\u{FFFF}')
        })
        .filter(|c| {
            // Exclude zero-width and invisible chars
            !matches!(
                *c,
                '\u{200B}' | // zero-width space
                '\u{200C}' | // zero-width non-joiner
                '\u{200D}' | // zero-width joiner
                '\u{FEFF}' | // BOM / zero-width no-break space
                '\u{00AD}'   // soft hyphen
            )
        })
        .collect()
}
