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

/// 根据内容判断语言：采样统计 CJK 字符与 ASCII 字母占比。
/// 日文假名一并计入 CJK（非英文）。英文小说几乎无 CJK → "en"。
pub fn detect_language(text: &str) -> &'static str {
    let sample: String = text.chars().take(20_000).collect();
    let mut cjk = 0usize;
    let mut ascii_letters = 0usize;
    for ch in sample.chars() {
        if ('\u{4e00}'..='\u{9fff}').contains(&ch)
            || ('\u{3400}'..='\u{4dbf}').contains(&ch)
            || ('\u{3040}'..='\u{30ff}').contains(&ch)
        {
            cjk += 1;
        } else if ch.is_ascii_alphabetic() {
            ascii_letters += 1;
        }
    }
    if ascii_letters > 0 && cjk * 5 < ascii_letters {
        "en"
    } else {
        "zh"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_language_english_and_chinese() {
        let english = "All I've ever wanted is for Juli Baker to leave me alone.\nShe barged into my life.";
        assert_eq!(detect_language(english), "en");
        let chinese = "我只想让朱莉·贝克别来烦我。她硬是闯进了我的生活，还推推搡搡。";
        assert_eq!(detect_language(chinese), "zh");
        assert_eq!(detect_language(""), "zh");
    }
}
