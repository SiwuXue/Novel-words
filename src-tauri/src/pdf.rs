use genpdf::elements::{Break, Paragraph};
use genpdf::fonts::{FontData, FontFamily};
use genpdf::{Alignment, Document};
use std::path::Path;

use crate::models::novel::Novel;
use crate::models::pdf_template::PdfTemplate;
use crate::models::vocab_word::VocabWord;

/// Try to find a Chinese-capable TTF/OTF font file on the system.
/// TTC (TrueType Collection) files are NOT supported by genpdf's font parser.
fn find_chinese_font() -> Option<String> {
    #[cfg(target_os = "windows")]
    let candidates = [
        // TTF fonts first (supported directly)
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simkai.ttf",
        r"C:\Windows\Fonts\FZSTK.TTF",
        r"C:\Windows\Fonts\FZKTK.TTF",
        r"C:\Windows\Fonts\SIMLI.TTF",
        r"C:\Windows\Fonts\FZYTK.TTF",
        r"C:\Windows\Fonts\arialuni.ttf",
        // TTC fonts — will try to extract first face
        r"C:\Windows\Fonts\simsun.ttc",
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\mingliu.ttc",
    ];
    #[cfg(target_os = "macos")]
    let candidates = [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STSong.ttf",
    ];
    #[cfg(target_os = "linux")]
    let candidates = [
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ];

    for path in &candidates {
        if Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    None
}

/// Load font data from a file path. Handles TTC files by extracting the
/// first font face into a standalone TTF buffer.
fn load_font_data(path: &str) -> Result<Vec<u8>, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("读取字体文件失败: {}", e))?;

    if bytes.len() < 4 {
        return Err("字体文件太小".to_string());
    }

    // Check if it's a TTC (TrueType Collection) file
    let tag = &bytes[0..4];
    if tag == b"ttcf" {
        // Extract the first font face from the TTC
        extract_ttc_face(&bytes, 0)
    } else {
        Ok(bytes)
    }
}

/// Extract a single font face from a TrueType Collection file.
///
/// TTC format:
///   offset 0:  tag "ttcf" (4 bytes)
///   offset 4:  version (u32)
///   offset 8:  numFonts (u32)
///   offset 12: tableDirectoryOffsets[numFonts] (u32 each)
///
/// Each table directory is a standard TTF table directory. We extract
/// the font by rebuilding a TTF file from the selected face's tables.
fn extract_ttc_face(ttc_data: &[u8], face_index: u32) -> Result<Vec<u8>, String> {
    if ttc_data.len() < 12 {
        return Err("TTC 文件不完整".to_string());
    }

    let num_fonts = u32::from_be_bytes([ttc_data[8], ttc_data[9], ttc_data[10], ttc_data[11]]);
    if face_index >= num_fonts {
        return Err(format!("TTC 字体索引超出范围: {}/{}", face_index, num_fonts));
    }

    let offset_offset = 12 + (face_index as usize) * 4;
    if offset_offset + 4 > ttc_data.len() {
        return Err("TTC 偏移表不完整".to_string());
    }

    let table_dir_offset = u32::from_be_bytes([
        ttc_data[offset_offset],
        ttc_data[offset_offset + 1],
        ttc_data[offset_offset + 2],
        ttc_data[offset_offset + 3],
    ]) as usize;

    if table_dir_offset + 12 > ttc_data.len() {
        return Err("TTC 表目录偏移超出文件范围".to_string());
    }

    // Read the table directory at the offset
    let dir = &ttc_data[table_dir_offset..];
    let num_tables = u16::from_be_bytes([dir[4], dir[5]]) as usize;

    if dir.len() < 12 + num_tables * 16 {
        return Err("TTC 表目录不完整".to_string());
    }

    // Collect table records: tag, checksum, offset, length
    struct TableRec {
        tag: [u8; 4],
        checksum: u32,
        offset: u32,
        length: u32,
    }

    let mut tables: Vec<TableRec> = Vec::new();
    for i in 0..num_tables {
        let base = 12 + i * 16;
        let tag: [u8; 4] = [dir[base], dir[base + 1], dir[base + 2], dir[base + 3]];
        let checksum = u32::from_be_bytes([dir[base + 4], dir[base + 5], dir[base + 6], dir[base + 7]]);
        let offset = u32::from_be_bytes([dir[base + 8], dir[base + 9], dir[base + 10], dir[base + 11]]);
        let length = u32::from_be_bytes([dir[base + 12], dir[base + 13], dir[base + 14], dir[base + 15]]);
        tables.push(TableRec { tag, checksum, offset, length });
    }

    // Sort tables by offset for efficient extraction
    tables.sort_by_key(|t| t.offset);

    // Calculate the total size of the new TTF file
    let header_size = 12; // sfVersion + numTables + searchRange + entrySelector + rangeShift
    let table_dir_size = num_tables * 16; // 16 bytes per table record
    let tables_start = header_size + table_dir_size;

    // Each table must be 4-byte aligned within its data section
    let mut current_offset = tables_start as u32;
    let mut table_data: Vec<(&TableRec, u32, Vec<u8>)> = Vec::new(); // (rec, new_offset, padded_data)

    for rec in &tables {
        // Align to 4-byte boundary
        if current_offset % 4 != 0 {
            current_offset += 4 - (current_offset % 4);
        }

        let data_start = table_dir_offset + rec.offset as usize;
        let data_end = data_start + rec.length as usize;
        if data_end > ttc_data.len() {
            return Err(format!("TTC 表 '{}' 数据超出文件范围", String::from_utf8_lossy(&rec.tag)));
        }

        let mut data = ttc_data[data_start..data_end].to_vec();
        // Pad to 4-byte alignment
        while data.len() % 4 != 0 {
            data.push(0);
        }
        let data_len = data.len() as u32;

        table_data.push((rec, current_offset, data));
        current_offset += data_len;
    }

    // Build the new TTF file
    let total_size = current_offset as usize;
    let mut ttf = vec![0u8; total_size];

    // Write header
    ttf[0..4].copy_from_slice(&dir[0..4]); // sfVersion
    ttf[4..6].copy_from_slice(&(num_tables as u16).to_be_bytes());
    // searchRange = max power of 2 <= numTables * 16
    let search_range = (num_tables as u16).next_power_of_two() / 2 * 16;
    ttf[6..8].copy_from_slice(&search_range.to_be_bytes());
    // entrySelector = log2(max power of 2)
    let entry_selector = ((num_tables as u16).next_power_of_two() / 2).trailing_zeros() as u16;
    ttf[8..10].copy_from_slice(&entry_selector.to_be_bytes());
    // rangeShift = numTables * 16 - searchRange
    let range_shift = (num_tables as u16) * 16 - search_range;
    ttf[10..12].copy_from_slice(&range_shift.to_be_bytes());

    // Write table directory (sorted by tag for standard TTF ordering)
    let mut sorted_tables: Vec<&(&TableRec, u32, Vec<u8>)> = table_data.iter().collect();
    sorted_tables.sort_by_key(|t| t.0.tag);

    for (i, (rec, new_offset, _data)) in sorted_tables.iter().enumerate() {
        let base = header_size + i * 16;
        ttf[base..base + 4].copy_from_slice(&rec.tag);
        ttf[base + 4..base + 8].copy_from_slice(&rec.checksum.to_be_bytes());
        ttf[base + 8..base + 12].copy_from_slice(&new_offset.to_be_bytes());
        ttf[base + 12..base + 16].copy_from_slice(&rec.length.to_be_bytes());
    }

    // Write table data
    for (_rec, new_offset, data) in &table_data {
        let start = *new_offset as usize;
        ttf[start..start + data.len()].copy_from_slice(data);
    }

    Ok(ttf)
}

/// Build a FontFamily from a font file path.
/// Handles both TTF and TTC (TrueType Collection) files.
fn load_font(path: &str) -> Result<FontFamily<FontData>, String> {
    let font_bytes = load_font_data(path)?;
    let font_data =
        FontData::new(font_bytes, None).map_err(|e| format!("解析字体失败: {}", e))?;
    // Clone the same font for bold/italic — Chinese fonts rarely have
    // separate variants; genpdf will simulate them.
    Ok(FontFamily {
        regular: font_data.clone(),
        bold: font_data.clone(),
        italic: font_data.clone(),
        bold_italic: font_data,
    })
}

/// Extract plain text from novel, handling the case where cleaned_text has
/// been overwritten with HTML by the editor autosave.
fn get_novel_text(novel: &Novel) -> String {
    let raw = if !novel.cleaned_text.is_empty() {
        &novel.cleaned_text
    } else {
        &novel.raw_text
    };

    let trimmed = raw.trim_start();
    if trimmed.starts_with('<') {
        strip_html_tags(raw)
    } else {
        raw.clone()
    }
}

/// Strip HTML tags, recovering plain text.
fn strip_html_tags(html: &str) -> String {
    let mut text = html
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");

    for tag in &[
        "</p>", "</h1>", "</h2>", "</h3>", "</h4>", "</h5>", "</h6>",
        "</li>", "</div>", "</tr>",
    ] {
        text = text.replace(tag, "\n");
    }

    let mut result = String::new();
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result = result
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    // Normalize whitespace
    let lines: Vec<&str> = result.lines().collect();
    let mut out = String::new();
    let mut prev_empty = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_empty {
                out.push('\n');
                prev_empty = true;
            }
        } else {
            out.push_str(trimmed);
            out.push('\n');
            prev_empty = false;
        }
    }
    out.trim().to_string()
}

/// Build and render the PDF document.
pub fn generate_pdf(
    novel: &Novel,
    template: &PdfTemplate,
    vocabs: &[VocabWord],
    output_path: &str,
) -> Result<(), String> {
    // ---- Font ----
    let font_path =
        find_chinese_font().ok_or_else(|| "未找到系统中文字体".to_string())?;
    let font_family = load_font(&font_path)?;

    // ---- Margins ----
    let margins: serde_json::Value = serde_json::from_str(&template.margins)
        .unwrap_or(serde_json::json!({"top":25,"bottom":25,"left":20,"right":20}));
    let _margin_top = margins["top"].as_u64().unwrap_or(25) as u32;
    let _margin_bottom = margins["bottom"].as_u64().unwrap_or(25) as u32;
    let _margin_left = margins["left"].as_u64().unwrap_or(20) as u32;
    let _margin_right = margins["right"].as_u64().unwrap_or(20) as u32;

    let font_size = template.font_size.max(8).min(24) as u8;
    let line_spacing = template.line_spacing.max(1.0).min(3.0);

    let mut doc = Document::new(font_family);
    doc.set_font_size(font_size);
    doc.set_line_spacing(line_spacing);

    // ---- Title page ----
    let title = if novel.title.is_empty() {
        "未命名"
    } else {
        &novel.title
    };
    let date = if !novel.updated_at.is_empty() {
        &novel.updated_at[..novel.updated_at.len().min(10)]
    } else if !novel.created_at.is_empty() {
        &novel.created_at[..novel.created_at.len().min(10)]
    } else {
        ""
    };

    for _ in 0..8 {
        doc.push(Paragraph::new(""));
    }
    let mut title_para = Paragraph::new(title);
    title_para.set_alignment(Alignment::Center);
    doc.push(title_para);

    if !novel.author.is_empty() {
        let mut author_para = Paragraph::new(&novel.author);
        author_para.set_alignment(Alignment::Center);
        doc.push(author_para);
    }
    if !date.is_empty() {
        let mut date_para = Paragraph::new(date);
        date_para.set_alignment(Alignment::Center);
        doc.push(date_para);
    }

    // Force page break after title page
    doc.push(Break::new(999.0));

    // ---- Body text ----
    let text = get_novel_text(novel);

    match template.annotation_mode.as_str() {
        "inline" if !vocabs.is_empty() => {
            render_inline_body(&mut doc, &text, vocabs);
        }
        _ => {
            render_plain_body(&mut doc, &text);
        }
    }

    // ---- Appendix (vocab table formatted as text) ----
    if (template.annotation_mode == "appendix"
        || template.annotation_mode == "sidebar")
        && !vocabs.is_empty()
    {
        doc.push(Break::new(999.0));
        render_appendix(&mut doc, vocabs);
    }

    doc.render_to_file(output_path)
        .map_err(|e| format!("PDF 生成失败: {}", e))?;

    Ok(())
}

/// Render plain text body — one paragraph per blank-line-separated block.
fn render_plain_body(doc: &mut Document, text: &str) {
    for para in text.split("\n\n") {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Replace intra-paragraph newlines with spaces
        let single_line = trimmed.replace('\n', "");
        if !single_line.is_empty() {
            doc.push(Paragraph::new(single_line));
        }
    }
}

/// Render body with inline vocab annotations.
fn render_inline_body(doc: &mut Document, text: &str, vocabs: &[VocabWord]) {
    let mut sorted: Vec<&VocabWord> = vocabs.iter().collect();
    sorted.sort_by(|a, b| b.word.len().cmp(&a.word.len()));

    if sorted.is_empty() {
        render_plain_body(doc, text);
        return;
    }

    for para in text.split("\n\n") {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }
        let single_line = trimmed.replace('\n', " ");

        // Find all word matches with word-boundary check
        let lower = single_line.to_lowercase();
        let mut matches: Vec<(usize, usize, &VocabWord)> = Vec::new();
        for v in &sorted {
            let word_lower = v.word.to_lowercase();
            let mut start = 0;
            while let Some(pos) = lower[start..].find(&word_lower) {
                let abs_pos = start + pos;
                let end = abs_pos + v.word.len();
                let left_ok = abs_pos == 0
                    || !lower.as_bytes()[abs_pos - 1].is_ascii_alphabetic();
                let right_ok = end >= lower.len()
                    || !lower.as_bytes()[end].is_ascii_alphabetic();
                if left_ok && right_ok {
                    matches.push((abs_pos, end, v));
                }
                start = end;
            }
        }
        matches.sort_by_key(|m| m.0);

        // Remove overlapping matches
        let mut filtered: Vec<(usize, usize, &VocabWord)> = Vec::new();
        for m in matches {
            if !filtered.iter().any(|f| m.0 < f.1 && f.0 < m.1) {
                filtered.push(m);
            }
        }

        if filtered.is_empty() {
            doc.push(Paragraph::new(single_line));
        } else {
            let mut result = String::new();
            let mut last = 0;
            for (start, end, v) in &filtered {
                result.push_str(&single_line[last..*start]);
                result.push_str(&single_line[*start..*end]);
                if !v.definition.is_empty() {
                    result.push_str(&format!("【{}】", v.definition));
                }
                last = *end;
            }
            result.push_str(&single_line[last..]);
            doc.push(Paragraph::new(result));
        }
    }
}

/// Render vocabulary appendix as formatted text (genpdf v0.2 has no Table support).
fn render_appendix(doc: &mut Document, vocabs: &[VocabWord]) {
    let mut title = Paragraph::new("词汇附录");
    title.set_alignment(Alignment::Center);
    doc.push(title);
    doc.push(Paragraph::new(""));

    // Deduplicate
    let mut seen = std::collections::HashSet::new();
    let unique: Vec<&VocabWord> = vocabs
        .iter()
        .filter(|v| seen.insert(v.word.to_lowercase()))
        .collect();

    for v in &unique {
        let proficiency = match v.proficiency.as_str() {
            "mastered" => "已掌握",
            "familiar" => "熟悉",
            _ => "生疏",
        };
        let line = format!(
            "{}  {}  {}  [{}]",
            v.word,
            if v.phonetic.is_empty() { "—" } else { &v.phonetic },
            if v.definition.is_empty() { "—" } else { &v.definition },
            proficiency,
        );
        doc.push(Paragraph::new(line));
    }
}
