file_path = r"c:\Users\34889\.trae-cn\worktrees\novel-words\export-pdf-provided-SnJzEa\src-tauri\src\pdf\mod.rs"
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

# Fix 1: Replace PdfContext init + title rendering
old_block = """    let mut ctx = PdfContext {
        doc,
        font_id,
        latin_font_id,
        cjk_parsed: parsed_font,
        font_size,
        small_font_size: font_size * 0.65,
        line_height,
        margins: margins.clone(),
        usable_width: usable_w,
        usable_height: usable_h,
        current_y: paper_h - margins.top - 60.0,
        page_count: 0,
        paper_width: paper_w,
        paper_height: paper_h,
        current_ops: Vec::new(),
        bookmarks: Vec::new(),
    };

    // 3. Render title (y is now bottom-based, matching current_y)
    let title_y = paper_h - margins.top - 5.0;
    let author_y = paper_h - margins.top - 25.0;
    let title_str = if novel.title.is_empty() { "未命名" } else { &novel.title };
    ctx.draw_text(title_str, margins.left, title_y, font_size + 4.0);
    if !novel.author.is_empty() {
        ctx.draw_text(&novel.author, margins.left, author_y, font_size);
    }

    ctx.new_page();

    // 4. Dispatch"""

new_block = """    let is_intensive = template.template_type == "intensive";

    let mut ctx = PdfContext {
        doc,
        font_id,
        latin_font_id,
        cjk_parsed: parsed_font,
        font_size,
        small_font_size: font_size * 0.65,
        line_height,
        margins: margins.clone(),
        usable_width: usable_w,
        usable_height: usable_h,
        current_y: if is_intensive { paper_h - margins.top - 5.0 } else { paper_h - margins.top - 60.0 },
        page_count: 0,
        paper_width: paper_w,
        paper_height: paper_h,
        current_ops: Vec::new(),
        bookmarks: Vec::new(),
        show_chrome: is_intensive,
        chapter_page: 1,
        novel_title: if novel.title.is_empty() { String::new() } else { novel.title.clone() },
        novel_author: if novel.author.is_empty() { String::new() } else { novel.author.clone() },
    };

    // 3. Render title page (skip for intensive \u2014 it has its own chapter headers)
    if !is_intensive {
        let title_y = paper_h - margins.top - 5.0;
        let author_y = paper_h - margins.top - 25.0;
        let title_str = if novel.title.is_empty() { "未命名" } else { &novel.title };
        ctx.draw_text(title_str, margins.left, title_y, font_size + 4.0);
        if !novel.author.is_empty() {
            ctx.draw_text(&novel.author, margins.left, author_y, font_size);
        }
        ctx.new_page();
    }

    // 4. Dispatch"""

if old_block in content:
    content = content.replace(old_block, new_block)
    print("Fix 1: PdfContext init + title page - OK")
else:
    print("Fix 1: ERROR - old_block not found")

# Fix 2: Final page finalization - add chrome
old_final = """    // 6. Finalize last page
    if !ctx.current_ops.is_empty() {
        let ops = std::mem::take(&mut ctx.current_ops);
        ctx.doc.pages.push(PdfPage::new(Mm(paper_w), Mm(paper_h), ops));
    }"""

new_final = """    // 6. Finalize last page
    if !ctx.current_ops.is_empty() {
        if ctx.show_chrome {
            ctx.render_page_chrome();
        }
        let ops = std::mem::take(&mut ctx.current_ops);
        ctx.doc.pages.push(PdfPage::new(Mm(paper_w), Mm(paper_h), ops));
    }"""

if old_final in content:
    content = content.replace(old_final, new_final)
    print("Fix 2: Final page chrome - OK")
else:
    print("Fix 2: ERROR - old_final not found")

with open(file_path, "w", encoding="utf-8") as f:
    f.write(content)
