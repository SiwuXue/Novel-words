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
