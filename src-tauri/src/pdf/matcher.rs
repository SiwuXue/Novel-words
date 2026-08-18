//! Match English vocab words against Chinese body text via their Chinese definitions.
//!
//! The novels are Chinese; vocab words are English. There is no English text in
//! the body to search for, so instead we extract the Chinese terms from each
//! word's `definition` and locate those terms in the body text. A hit means the
//! word's meaning appears there, so we can annotate the English word at that spot.
use crate::models::vocab_word::VocabWord;

/// Is `c` a CJK ideograph (rough range, good enough for term extraction).
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // Extension A
        | '\u{F900}'..='\u{FAFF}' // Compatibility Ideographs
    )
}

/// Extract candidate Chinese terms from an English word's definition.
///
/// A definition looks like `"n. 苹果；(水果) 苹果树, vt. 落地"`. We split on common
/// separators, drop part-of-speech markers / ASCII, drop parenthetical notes, and
/// keep contiguous CJK runs of length >= 2 (single chars cause too many false hits).
pub fn extract_cn_terms(definition: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let separators = |c: char| {
        matches!(c,
            ';' | '；' | ',' | '，' | '、' | '/' | '|' | '\n' | '.'
            | '(' | ')' | '（' | '）' | '[' | ']' | '【' | '】'
            | ' ' | '\t' | '～' | '~' | '"' | '\u{201C}' | '\u{201D}'
        )
    };
    for segment in definition.split(separators) {
        // Keep only the contiguous leading/embedded CJK run of this segment.
        let mut run = String::new();
        for ch in segment.chars() {
            if is_cjk(ch) {
                run.push(ch);
            } else if !run.is_empty() {
                // run ended
                push_term(&mut terms, &run);
                run.clear();
            }
        }
        if !run.is_empty() {
            push_term(&mut terms, &run);
        }
    }
    terms
}

fn push_term(terms: &mut Vec<String>, run: &str) {
    // Require >= 2 CJK chars to avoid matching ubiquitous single characters.
    if run.chars().count() >= 2 && !terms.iter().any(|t| t == run) {
        terms.push(run.to_string());
    }
}

/// One matched occurrence of a vocab word inside a line, by its Chinese meaning.
pub struct DefMatch<'a> {
    /// byte offset in the line where the Chinese term starts
    pub start: usize,
    /// byte offset where it ends
    pub end: usize,
    /// the matched Chinese term (substring of the line)
    pub term_len: usize,
    pub word: &'a VocabWord,
}

/// Find all non-overlapping matches of vocab words in `line`, matched by their
/// Chinese definitions. Longer terms win over shorter overlapping ones.
pub fn find_matches_in_line<'a>(
    line: &str,
    words: &'a [VocabWord],
) -> Vec<DefMatch<'a>> {
    let mut raw: Vec<DefMatch> = Vec::new();
    for w in words {
        for term in extract_cn_terms(&w.definition) {
            let mut start = 0;
            while let Some(pos) = line[start..].find(&term) {
                let abs = start + pos;
                let end = abs + term.len();
                raw.push(DefMatch {
                    start: abs,
                    end,
                    term_len: term.chars().count(),
                    word: w,
                });
                if end <= start {
                    break;
                }
                start = end;
            }
        }
    }
    // Sort by position, then by longer term first so the longest wins on overlap.
    raw.sort_by(|a, b| a.start.cmp(&b.start).then(b.term_len.cmp(&a.term_len)));

    let mut filtered: Vec<DefMatch> = Vec::new();
    for m in raw {
        let overlaps = filtered.iter().any(|f| m.start < f.end && f.start < m.end);
        if !overlaps {
            filtered.push(m);
        }
    }
    filtered
}

/// Return the subset of `words` whose Chinese meaning appears anywhere in `text`,
/// deduplicated by the English word (case-insensitive), preserving input order.
pub fn words_found_in_text<'a>(text: &str, words: &'a [VocabWord]) -> Vec<&'a VocabWord> {
    let mut found: Vec<&VocabWord> = Vec::new();
    for w in words {
        let hit = extract_cn_terms(&w.definition)
            .iter()
            .any(|term| text.contains(term.as_str()));
        if hit {
            let key = w.word.to_lowercase();
            if !found.iter().any(|f| f.word.to_lowercase() == key) {
                found.push(w);
            }
        }
    }
    found
}

// ============================================================
// English-novel mode: locate the English word directly in the
// (English) body text using word-boundary case-insensitive match.
// ============================================================

/// One occurrence of a vocab word found as an English token in the line.
pub struct EnMatch<'a> {
    /// byte offset where the word starts
    pub start: usize,
    /// byte offset where the word ends
    pub end: usize,
    pub word: &'a VocabWord,
}

/// True if `c` is an ASCII letter (used for word-boundary checks).
fn is_ascii_letter(c: char) -> bool {
    matches!(c, 'a'..='z' | 'A'..='Z')
}

/// Case-insensitive contains() for ASCII — used for the "found anywhere"
/// check on English text. Sufficient for vocab words (pure ASCII).
fn contains_ascii_ci(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    let needle_l = needle.to_lowercase();
    let h_chars: Vec<char> = haystack.chars().collect();
    let n_chars: Vec<char> = needle_l.chars().collect();
    let n = n_chars.len();
    if h_chars.len() < n {
        return false;
    }
    for i in 0..=(h_chars.len() - n) {
        let mut ok = true;
        for j in 0..n {
            let h = h_chars[i + j].to_ascii_lowercase();
            if h != n_chars[j] {
                ok = false;
                break;
            }
        }
        if ok {
            return true;
        }
    }
    false
}

/// Find all whole-word, case-insensitive occurrences of each vocab word
/// in `line` (English body text). Non-overlapping; longer words win on
/// overlap. Apostrophes inside the word (e.g. "don't") are tolerated by
/// treating `'` as a word character when it sits between letters.
pub fn find_matches_in_line_en<'a>(
    line: &str,
    words: &'a [VocabWord],
) -> Vec<EnMatch<'a>> {
    // Pre-build lowercase char array of the line for fast scanning.
    let line_chars: Vec<char> = line.chars().collect();
    let line_lower: String = line_chars.iter().map(|c| c.to_ascii_lowercase()).collect();
    let line_lower_chars: Vec<char> = line_lower.chars().collect();

    // Byte offsets of each char in the original line, for converting
    // char-index back to byte-offset.
    let mut char_to_byte: Vec<usize> = Vec::with_capacity(line_chars.len());
    let mut byte = 0;
    for c in &line_chars {
        char_to_byte.push(byte);
        byte += c.len_utf8();
    }

    let n = line_chars.len();
    let mut raw: Vec<EnMatch> = Vec::new();

    for w in words {
        let needle = w.word.trim();
        if needle.is_empty() {
            continue;
        }
        let needle_l: String = needle.to_lowercase();
        let needle_chars: Vec<char> = needle_l.chars().collect();
        let m = needle_chars.len();
        if m == 0 || n < m {
            continue;
        }

        for i in 0..=(n - m) {
            // whole-word boundary check (left): previous char must not be a letter/'-'
            if i > 0 {
                let prev = line_chars[i - 1];
                if is_ascii_letter(prev) || prev == '\'' || prev == '-' {
                    continue;
                }
            }
            // match chars
            let mut ok = true;
            for j in 0..m {
                if line_lower_chars[i + j] != needle_chars[j] {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }
            // whole-word boundary check (right): next char must not be a letter/'-'
            let end_idx = i + m;
            if end_idx < n {
                let nxt = line_chars[end_idx];
                if is_ascii_letter(nxt) || nxt == '\'' || nxt == '-' {
                    continue;
                }
            }
            let start_byte = char_to_byte[i];
            let end_byte = char_to_byte[i] + needle.len();
            raw.push(EnMatch {
                start: start_byte,
                end: end_byte,
                word: w,
            });
        }
    }

    // Sort by position then by longer word first to let the longest win on overlap.
    raw.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then((b.end - b.start).cmp(&(a.end - a.start)))
    });
    let mut filtered: Vec<EnMatch> = Vec::new();
    for m in raw {
        let overlaps = filtered.iter().any(|f| m.start < f.end && f.start < m.end);
        if !overlaps {
            filtered.push(m);
        }
    }
    filtered
}

/// English-novel version of words_found_in_text: return the subset of
/// `words` whose English word field appears as a whole word anywhere in
/// `text` (case-insensitive), deduplicated by lowercase word.
pub fn words_found_in_text_en<'a>(text: &str, words: &'a [VocabWord]) -> Vec<&'a VocabWord> {
    let mut found: Vec<&VocabWord> = Vec::new();
    for w in words {
        let needle = w.word.trim();
        if needle.is_empty() {
            continue;
        }
        if contains_ascii_ci(text, needle) {
            let key = w.word.to_lowercase();
            if !found.iter().any(|f| f.word.to_lowercase() == key) {
                found.push(w);
            }
        }
    }
    found
}
