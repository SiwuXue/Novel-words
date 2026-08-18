import re

file_path = r'c:\Users\34889\.trae-cn\worktrees\novel-words\export-pdf-provided-SnJzEa\src-tauri\src\pdf\mod.rs'
with open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Replace new_page and new_page_for_chapter, add render_page_chrome and reset_chapter_page
old_block = """impl PdfContext {
    pub fn new_page(&mut self) {
        let ops = std::mem::take(&mut self.current_ops);
        let page = PdfPage::new(Mm(self.paper_width), Mm(self.paper_height), ops);
        self.doc.pages.push(page);
        self.page_count += 1;
        self.current_y = self.paper_height - self.margins.top;
    }

    /// Start a fresh page for a new chapter, but avoid emitting a blank page when
    /// the current page has nothing drawn on it yet (e.g. right after another
    /// chapter already forced a page break).
    pub fn new_page_for_chapter(&mut self) {
        if self.current_ops.is_empty() {
            // Current page is empty \u2014 just reset the cursor to the top, don't push
            // an empty page.
            self.current_y = self.paper_height - self.margins.top;
        } else {
            self.new_page();
        }
    }"""

new_block = """impl PdfContext {
    pub fn new_page(&mut self) {
        if self.show_chrome {
            self.render_page_chrome();
        }
        let ops = std::mem::take(&mut self.current_ops);
        let page = PdfPage::new(Mm(self.paper_width), Mm(self.paper_height), ops);
        self.doc.pages.push(page);
        self.page_count += 1;
        self.chapter_page += 1;
        self.current_y = self.paper_height - self.margins.top;
    }

    /// Start a fresh page for a new chapter, but avoid emitting a blank page when
    /// the current page has nothing drawn on it yet (e.g. right after another
    /// chapter already forced a page break).
    pub fn new_page_for_chapter(&mut self) {
        if self.current_ops.is_empty() {
            self.current_y = self.paper_height - self.margins.top;
        } else {
            self.new_page();
        }
    }

    /// Reset the per-chapter page counter to 1.
    pub fn reset_chapter_page(&mut self) {
        self.chapter_page = 1;
    }

    /// Draw page header (copyright info) and footer (page number).
    fn render_page_chrome(&mut self) {
        let pn = self.chapter_page;
        let header_y = self.paper_height - self.margins.top + 8.0;
        let copyright = if self.novel_author.is_empty() {
            "\u5df2\u7533\u8bf7\u77e5\u8bc6\u4ea7\u6743\uff01\u7981\u6b62\u5012\u5356".to_string()
        } else {
            format!("\u4f5c\u8005\uff1a{}  \u5df2\u7533\u8bf7\u77e5\u8bc6\u4ea7\u6743\uff01\u7981\u6b62\u5012\u5356", self.novel_author)
        };
        self.draw_text_colored(&copyright, self.margins.left, header_y, self.small_font_size * 0.75, TEXT_RED);
        let title_line = if self.novel_title.is_empty() {
            "\u8bcd\u5b66\u4e60\u5c0f\u8bf4".to_string()
        } else {
            format!("\u300a{}\u300b\u2014 \u8bcd\u5b66\u4e60\u5c0f\u8bf4", self.novel_title)
        };
        self.draw_text_colored(&title_line, self.margins.left, header_y - 3.5, self.small_font_size * 0.75, TEXT_RED);
        let page_str = format!("\u7b2c {} \u9875", pn);
        let pw = self.measure_text_width(&page_str, self.small_font_size);
        let cx = (self.paper_width - pw) / 2.0;
        self.draw_text(&page_str, cx, self.margins.bottom - 5.0, self.small_font_size);
    }"""

if old_block in content:
    content = content.replace(old_block, new_block)
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(content)
    print('Success: new_page and render_page_chrome added')
else:
    print('ERROR: old_block not found')
    idx = content.find('impl PdfContext')
    if idx >= 0:
        print(repr(content[idx:idx+600]))
