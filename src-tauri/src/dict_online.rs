//! 在线词典抓取层 — 有道 / 剑桥网页解析 + DeepLX 整句翻译。
//!
//! 请求经 Rust reqwest 发出（绕过 webview CORS），带浏览器 UA 与超时。
//! 网页解析规则移植自 saladict（参考 obsidian-language-learner 的移植），
//! 词典网站改版可能导致解析失效：此时命令返回 Err，由前端回落离线词典。
//!
//! 测试夹具：tests/fixtures/{youdao,cambridge}_apple.html（真实页面快照）。

use crate::db::DbState;
use scraper::{Html, Selector};
use serde::Serialize;
use std::time::Duration;
use tauri::State;

const DEFAULT_DEEPL_ENDPOINT: &str = "https://deeplx.1stg.me/translate";
const CAMBRIDGE_HOST: &str = "https://dictionary.cambridge.org";
const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";

// ---------------------------------------------------------------------------
// 结果结构
// ---------------------------------------------------------------------------

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct OnlineExample {
    pub en: String,
    pub zh: String,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct OnlineSense {
    /// 词性（如 "n." / "noun"），可为空
    pub pos: String,
    /// 该词性下的释义列表
    pub defs: Vec<String>,
    /// 该词性下的例句
    pub examples: Vec<OnlineExample>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct OnlineDictResult {
    /// "youdao" | "cambridge"
    pub source: String,
    pub word: String,
    pub phonetic_uk: String,
    pub phonetic_us: String,
    /// 有道真人发音 URL（可直接交给 speech 播放），可能为空
    pub audio_uk: String,
    pub audio_us: String,
    /// 扁平释义列表（每条一行，如 "n. 苹果"）
    pub translations: Vec<String>,
    /// 结构化释义（按词性分组），剑桥天然分组，有道合成单组
    pub senses: Vec<OnlineSense>,
    /// 例句（有道权威例句为纯英文；剑桥例句放在 senses 内）
    pub examples: Vec<OnlineExample>,
    /// 网页原链接（供"在浏览器打开"）
    pub url: String,
}

// ---------------------------------------------------------------------------
// HTTP 客户端
// ---------------------------------------------------------------------------

fn http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(12))
            .connect_timeout(Duration::from_secs(6))
            .http1_only()
            .user_agent(BROWSER_UA)
            .build()
            .expect("failed to build http client")
    })
}

async fn fetch_html(url: &str) -> Result<String, String> {
    let resp = http_client()
        .get(url)
        .header("Accept", "text/html,application/xhtml+xml")
        .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("词典网站返回 {}", resp.status()));
    }
    resp.text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))
}

// ---------------------------------------------------------------------------
// HTML 工具
// ---------------------------------------------------------------------------

/// 收集元素内所有可见文本并压缩空白
fn text_of(el: &scraper::ElementRef) -> String {
    let mut out = String::new();
    for t in el.text() {
        out.push_str(t);
    }
    collapse_ws(&out)
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
}

fn selector(sel: &str) -> Selector {
    Selector::parse(sel).expect("invalid css selector")
}

// ---------------------------------------------------------------------------
// 有道
// ---------------------------------------------------------------------------

/// 英→中：抓取有道网页并解析出释义/音标/例句
#[tauri::command]
pub async fn dict_online_youdao(word: String) -> Result<OnlineDictResult, String> {
    let w = word.trim();
    if w.is_empty() {
        return Err("查询词为空".into());
    }
    let url = format!("https://dict.youdao.com/w/{}", urlencode(w));
    let html = fetch_html(&url).await?;
    parse_youdao(&html, &url).ok_or_else(|| "有道词典未收录该词".into())
}

fn parse_youdao(html: &str, url: &str) -> Option<OnlineDictResult> {
    let doc = Html::parse_document(html);

    let word = doc
        .select(&selector(".keyword"))
        .next()
        .map(|e| text_of(&e))
        .unwrap_or_default();

    // 发音：.baav .pronounce → "英 [..]" / "美 [..]" + dictvoice data-rel
    let mut phonetic_uk = String::new();
    let mut phonetic_us = String::new();
    let mut audio_uk = String::new();
    let mut audio_us = String::new();
    for pron in doc.select(&selector(".baav .pronounce")) {
        let full = text_of(&pron);
        let phonetic = pron
            .select(&selector(".phonetic"))
            .next()
            .map(|e| text_of(&e))
            .unwrap_or_default();
        let audio = pron
            .select(&selector(".dictvoice"))
            .next()
            .and_then(|e| e.value().attr("data-rel"))
            .map(|rel| format!("https://dict.youdao.com/dictvoice?audio={}", rel))
            .unwrap_or_default();
        if full.starts_with('英') {
            phonetic_uk = phonetic;
            audio_uk = audio;
        } else if full.starts_with('美') {
            phonetic_us = phonetic;
            audio_us = audio;
        }
    }

    // 释义：#phrsListTab .trans-container 下的 li 行
    let translations: Vec<String> = doc
        .select(&selector("#phrsListTab .trans-container li"))
        .map(|e| text_of(&e))
        .filter(|s| !s.is_empty())
        .collect();

    // 词不在词库时，有道返回机器翻译容器，作为兜底释义
    let machine: Option<String> = doc
        .select(&selector("#fanyiToggle .trans-container p"))
        .next()
        .map(|e| text_of(&e))
        .filter(|s| !s.is_empty());

    // 双语例句：#bilingual ul.li（en p + zh p 成对）不够稳定，改用权威例句（纯英）
    let examples: Vec<OnlineExample> = doc
        .select(&selector("#authority ul.ol li"))
        .filter_map(|li| {
            let ps: Vec<String> = li
                .select(&selector("p"))
                .filter(|p| !p.value().attr("class").map_or(false, |c| c.contains("example-via")))
                .map(|p| text_of(&p))
                .filter(|s| !s.is_empty())
                .collect();
            ps.first()
                .map(|en| OnlineExample { en: en.clone(), zh: String::new() })
        })
        .take(3)
        .collect();

    if word.is_empty() && translations.is_empty() && machine.is_none() {
        return None;
    }

    let mut final_translations = translations;
    if final_translations.is_empty() {
        if let Some(m) = &machine {
            final_translations.push(m.clone());
        }
    }

    Some(OnlineDictResult {
        source: "youdao".into(),
        word,
        phonetic_uk,
        phonetic_us,
        audio_uk,
        audio_us,
        translations: final_translations,
        senses: Vec::new(), // 有道按行展示，不需要结构化分组
        examples,
        url: url.to_string(),
    })
}

// ---------------------------------------------------------------------------
// 剑桥
// ---------------------------------------------------------------------------

/// 英→中：抓取剑桥英汉简体词典并解析
#[tauri::command]
pub async fn dict_online_cambridge(word: String) -> Result<OnlineDictResult, String> {
    let w = word.trim();
    if w.is_empty() {
        return Err("查询词为空".into());
    }
    let url = format!(
        "https://dictionary.cambridge.org/zhs/%E6%90%9C%E7%B4%A2/direct/?datasetsearch=english-chinese-simplified&q={}",
        urlencode(w)
    );
    let html = fetch_html(&url).await?;
    parse_cambridge(&html).ok_or_else(|| "剑桥词典未收录该词".into())
}

fn parse_cambridge(html: &str) -> Option<OnlineDictResult> {
    let doc = Html::parse_document(html);

    let word = doc
        .select(&selector(".headword"))
        .next()
        .map(|e| text_of(&e))
        .unwrap_or_default();
    if word.is_empty() {
        return None;
    }

    // 第一个词条的发音（多词条只取第一组 uk/us）
    let mut phonetic_uk = String::new();
    let mut phonetic_us = String::new();
    let mut audio_uk = String::new();
    let mut audio_us = String::new();
    for pron in doc.select(&selector(".dpron-i")) {
        let classes = pron.value().attr("class").unwrap_or("");
        let is_uk = classes.split_whitespace().any(|w| w == "uk");
        let is_us = classes.split_whitespace().any(|w| w == "us");
        if !(is_uk || is_us) {
            continue;
        }
        let phonetic = pron
            .select(&selector(".ipa"))
            .next()
            .map(|e| text_of(&e))
            .unwrap_or_default();
        let audio = pron
            .select(&selector("source[type='audio/mpeg']"))
            .next()
            .and_then(|e| e.value().attr("src"))
            .map(|src| {
                if src.starts_with("http") {
                    src.to_string()
                } else {
                    format!("{}{}", CAMBRIDGE_HOST, src)
                }
            })
            .unwrap_or_default();
        if is_uk && phonetic_uk.is_empty() {
            phonetic_uk = phonetic;
            audio_uk = audio;
        } else if is_us && phonetic_us.is_empty() {
            phonetic_us = phonetic;
            audio_us = audio;
        }
    }

    // 按词条（词性）分组解析释义与例句
    let mut senses: Vec<OnlineSense> = Vec::new();
    let mut translations: Vec<String> = Vec::new();
    for entry in doc.select(&selector(".entry-body__el")) {
        let pos = entry
            .select(&selector(".posgram"))
            .next()
            .map(|e| text_of(&e))
            .unwrap_or_default();

        let mut defs: Vec<String> = Vec::new();
        let mut examples: Vec<OnlineExample> = Vec::new();
        for def_body in entry.select(&selector(".def-body")) {
            let def = def_body
                .select(&selector(".trans"))
                .next()
                .map(|e| text_of(&e))
                .unwrap_or_default();
            if def.is_empty() {
                continue;
            }
            defs.push(def.clone());
            let label = if pos.is_empty() { def } else { format!("{}: {}", pos, def) };
            translations.push(label);
            for ex in def_body.select(&selector(".examp")) {
                let en = ex
                    .select(&selector(".eg"))
                    .next()
                    .map(|e| text_of(&e))
                    .unwrap_or_default();
                let zh = ex
                    .select(&selector(".trans"))
                    .next()
                    .map(|e| text_of(&e))
                    .unwrap_or_default();
                if !en.is_empty() {
                    examples.push(OnlineExample { en, zh });
                }
            }
        }
        if !defs.is_empty() {
            senses.push(OnlineSense { pos, defs, examples });
        }
    }

    if translations.is_empty() {
        return None;
    }

    Some(OnlineDictResult {
        source: "cambridge".into(),
        word,
        phonetic_uk,
        phonetic_us,
        audio_uk,
        audio_us,
        translations,
        senses,
        examples: Vec::new(),
        url: String::new(),
    })
}

// ---------------------------------------------------------------------------
// DeepLX 整句翻译
// ---------------------------------------------------------------------------

#[derive(Serialize, Debug)]
pub struct SentenceTranslation {
    pub translated: String,
    /// DeepL 检测到的源语言（如 "EN"）
    pub source_lang: String,
}

/// 整句翻译：DeepLX 接口。端点可在设置 `deepl_endpoint` 中自定义。
/// 目标语种自动判断：含中文 → 译英，否则 → 译中。
#[tauri::command]
pub async fn dict_translate_sentence(
    state: State<'_, DbState>,
    text: String,
) -> Result<SentenceTranslation, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("翻译内容为空".into());
    }
    if t.chars().count() > 2000 {
        return Err("翻译内容过长（最多 2000 字符）".into());
    }

    // 读取自定义端点（读取失败不阻断，用默认值）
    let endpoint = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let val: Result<String, _> = db.query_row(
            "SELECT value FROM app_settings WHERE key='deepl_endpoint'",
            [],
            |row| row.get(0),
        );
        match val {
            Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
            _ => DEFAULT_DEEPL_ENDPOINT.to_string(),
        }
    };

    let target = if t.chars().any(is_cjk_char) { "EN" } else { "ZH" };

    let resp = http_client()
        .post(&endpoint)
        .timeout(Duration::from_secs(20))
        .json(&serde_json::json!({
            "text": t,
            "source_lang": "auto",
            "target_lang": target,
        }))
        .send()
        .await
        .map_err(|e| format!("翻译服务请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("翻译服务返回 {}", resp.status()));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析翻译响应失败: {}", e))?;
    if body["code"].as_i64() != Some(200) {
        return Err("翻译服务异常".into());
    }
    let translated = body["data"]
        .as_str()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    if translated.is_empty() {
        return Err("翻译结果为空".into());
    }
    Ok(SentenceTranslation {
        translated,
        source_lang: body["sourceLang"].as_str().unwrap_or("").to_string(),
    })
}

// ---------------------------------------------------------------------------
// 工具
// ---------------------------------------------------------------------------

fn is_cjk_char(c: char) -> bool {
    ('\u{4e00}'..='\u{9fa5}').contains(&c)
}

/// 最小 URL 编码（词典查询词：字母数字与少量安全字符之外全部转义）
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 测试（使用真实页面快照）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(path).expect("fixture missing")
    }

    #[test]
    fn youdao_apple_parses() {
        let html = fixture("youdao_apple.html");
        let r = parse_youdao(&html, "https://dict.youdao.com/w/apple").expect("should parse");
        assert_eq!(r.word, "apple");
        assert!(!r.phonetic_uk.is_empty());
        assert!(!r.phonetic_us.is_empty());
        assert!(r.audio_uk.contains("dictvoice"));
        assert!(r.audio_us.contains("dictvoice"));
        assert!(
            r.translations.iter().any(|t| t.contains("苹果")),
            "translations: {:?}",
            r.translations
        );
        assert!(!r.examples.is_empty());
    }

    #[test]
    fn cambridge_apple_parses() {
        let html = fixture("cambridge_apple.html");
        let r = parse_cambridge(&html).expect("should parse");
        assert_eq!(r.word, "apple");
        assert!(!r.phonetic_uk.is_empty());
        assert!(!r.phonetic_us.is_empty());
        assert!(r.audio_uk.ends_with(".mp3"));
        assert!(r.translations.iter().any(|t| t.contains("苹果")));
        assert!(!r.senses.is_empty());
        let sense = &r.senses[0];
        assert!(sense.pos.starts_with("noun"), "pos: {}", sense.pos);
        assert!(!sense.examples.is_empty());
        assert!(sense.examples.iter().all(|e| !e.zh.is_empty() || !e.en.is_empty()));
    }

    #[test]
    fn youdao_garbage_returns_none() {
        assert!(parse_youdao("<html><body>blocked</body></html>", "u").is_none());
        assert!(parse_cambridge("<html><body>blocked</body></html>").is_none());
    }

    #[test]
    fn urlencode_escapes() {
        assert_eq!(urlencode("apple pie"), "apple%20pie");
        assert_eq!(urlencode("café"), "caf%C3%A9");
        assert_eq!(urlencode("a-b_c.d"), "a-b_c.d");
    }
}
