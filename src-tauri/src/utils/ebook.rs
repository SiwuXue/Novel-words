//! EPUB / FB2 ebook parsing: extract a title and ordered chapters as plain text.
//! Both parsers produce `Vec<(title, content)>`, which the import command turns
//! into the standard `ImportResult` shape.

pub struct EbookResult {
    pub title: String,
    pub chapters: Vec<(String, String)>, // (title, content)
}

/// Build a single plain-text body from chapter title + content, matching the
/// layout of a TXT import (chapter heading, blank line, then content).
pub fn full_text(chapters: &[(String, String)]) -> String {
    let mut out = String::new();
    for (title, content) in chapters {
        if !title.is_empty() {
            out.push_str(title.trim());
            out.push_str("\n\n");
        }
        out.push_str(content.trim());
        out.push_str("\n\n");
    }
    out.trim().to_string()
}

// ---------------------------------------------------------------------------
// EPUB
// ---------------------------------------------------------------------------

pub fn parse_epub(path: &str) -> Result<EbookResult, String> {
    let mut doc = epub::doc::EpubDoc::new(path).map_err(|e| format!("无法打开 EPUB: {}", e))?;
    let title = doc.get_title().unwrap_or_default().trim().to_string();

    let spine = doc.spine.clone();
    let mut chapters: Vec<(String, String)> = Vec::new();
    for item in &spine {
        let (html, _mime) = match doc.get_resource_str(&item.idref) {
            Some(v) => v,
            None => continue,
        };
        let content = html_to_text(&html);
        if content.is_empty() {
            continue;
        }
        let ch_title = extract_heading(&html)
            .unwrap_or_else(|| format!("第 {} 章", chapters.len() + 1));
        chapters.push((ch_title, content));
    }

    if chapters.is_empty() {
        return Err("EPUB 中没有可读取的正文".into());
    }
    Ok(EbookResult { title, chapters })
}

/// Convert a single XHTML chapter to plain text with paragraph breaks.
fn html_to_text(html: &str) -> String {
    let mut s = remove_element(html, "script");
    s = remove_element(&s, "style");
    s = remove_element(&s, "head");

    for tag in [
        "</p>", "</div>", "</li>", "</h1>", "</h2>", "</h3>", "</h4>", "</h5>", "</h6>",
        "</tr>", "</blockquote>", "</section>", "</article>",
    ] {
        s = s.replace(tag, "\n");
    }
    s = s.replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");

    let stripped = strip_tags(&s);
    let decoded = html_escape::decode_html_entities(&stripped).to_string();
    normalize_paragraphs(&decoded)
}

/// Extract the first heading (h1..h6) text from an XHTML fragment, if any.
fn extract_heading(html: &str) -> Option<String> {
    for tag in ["h1", "h2", "h3", "h4", "h5", "h6"] {
        let open = format!("<{}", tag);
        let close = format!("</{}>", tag);
        let start = match html.find(&open) {
            Some(s) => s,
            None => continue,
        };
        let after_open = &html[start..];
        let content_start = match after_open.find('>') {
            Some(c) => c + 1,
            None => continue,
        };
        let content = &after_open[content_start..];
        let end = match content.find(&close) {
            Some(e) => e,
            None => continue,
        };
        let raw = strip_tags(&content[..end]);
        let text = html_escape::decode_html_entities(&raw).to_string();
        let text = text.trim().to_string();
        if !text.is_empty() {
            return Some(text);
        }
    }
    None
}

/// Remove `<tag ...> ... </tag>` blocks (case-sensitive; EPUB XHTML is lowercase).
fn remove_element(s: &str, tag: &str) -> String {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find(&open) {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        match after.find(&close) {
            Some(end) => rest = &after[end + close.len()..],
            None => {
                out.push_str(after);
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Collapse runs of blank lines to a single blank line and trim each line.
fn normalize_paragraphs(text: &str) -> String {
    let mut out = String::new();
    let mut blank = 0u32;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            if blank == 0 {
                out.push('\n');
            }
            blank += 1;
        } else {
            out.push_str(t);
            out.push('\n');
            blank = 0;
        }
    }
    out.trim().to_string()
}

// ---------------------------------------------------------------------------
// FB2 (FictionBook XML)
// ---------------------------------------------------------------------------

pub fn parse_fb2(path: &str) -> Result<EbookResult, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("无法读取 FB2: {}", e))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| "FB2 文件不是有效的 UTF-8 编码".to_string())?;
    let doc = roxmltree::Document::parse(text).map_err(|e| format!("FB2 XML 解析失败: {}", e))?;
    let root = doc.root_element();

    // Book title from <description><title-info><book-title>
    let mut title = String::new();
    if let Some(desc) = root.children().find(|n| n.has_tag_name("description")) {
        if let Some(ti) = desc.children().find(|n| n.has_tag_name("title-info")) {
            if let Some(bt) = ti.children().find(|n| n.has_tag_name("book-title")) {
                title = node_text(bt);
            }
        }
    }

    let mut chapters: Vec<(String, String)> = Vec::new();
    for body in root.children().filter(|n| n.has_tag_name("body")) {
        collect_sections(body, &mut chapters);
    }

    if chapters.is_empty() {
        return Err("FB2 中没有可读取的正文".into());
    }
    Ok(EbookResult { title, chapters })
}

fn collect_sections(body: roxmltree::Node, chapters: &mut Vec<(String, String)>) {
    let mut count = 0usize;
    for section in body
        .children()
        .filter(|n| n.is_element() && n.has_tag_name("section"))
    {
        count += 1;
        let (title, content) = extract_section(section, chapters.len() + 1);
        if !content.is_empty() {
            chapters.push((title, content));
        }
    }
    // Older FB2 may put <p> directly under <body> with no sections.
    if count == 0 {
        let content = extract_paragraphs(body);
        if !content.is_empty() {
            chapters.push((String::new(), content));
        }
    }
}

fn extract_section(section: roxmltree::Node, idx: usize) -> (String, String) {
    let mut title = String::new();
    if let Some(t) = section.children().find(|n| n.has_tag_name("title")) {
        title = node_text(t);
    }
    let title = if title.is_empty() {
        format!("第 {} 章", idx)
    } else {
        title
    };
    let content = extract_paragraphs(section);
    (title, content)
}

/// Concatenate every <p> outside a <title> block, separated by blank lines.
fn extract_paragraphs(node: roxmltree::Node) -> String {
    let mut paras = Vec::new();
    for p in node
        .descendants()
        .filter(|n| n.is_element() && n.has_tag_name("p"))
    {
        if is_inside_title(p) {
            continue;
        }
        let t = node_text(p);
        if !t.is_empty() {
            paras.push(t);
        }
    }
    paras.join("\n\n")
}

fn is_inside_title(node: roxmltree::Node) -> bool {
    let mut cur = node.parent();
    while let Some(p) = cur {
        if p.is_element() && p.has_tag_name("title") {
            return true;
        }
        cur = p.parent();
    }
    false
}

/// All text descendants of a node, trimmed.
fn node_text(node: roxmltree::Node) -> String {
    node.descendants()
        .filter(|n| n.is_text())
        .map(|n| n.text().unwrap_or(""))
        .collect::<String>()
        .trim()
        .to_string()
}
