//! Generic OpenAI-compatible chat-completions client used by preset tailoring.
//! The request intentionally sticks to the smallest common payload shared by
//! hosted providers and local OpenAI-compatible servers.

use std::time::Duration;

use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONNECTION};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;

use crate::db::DbState;

const KEY_ENABLED: &str = "ai_enhancer_enabled";
const KEY_PROVIDER: &str = "ai_provider";
const KEY_BASE_URL: &str = "ai_base_url";
const KEY_API_KEY: &str = "ai_api_key";
const KEY_MODEL: &str = "ai_model";
const KEY_TEMPERATURE: &str = "ai_temperature";
const KEY_TOP_P: &str = "ai_top_p";
const KEY_MAX_TOKENS: &str = "ai_max_tokens";
const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub enabled: bool,
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
}

impl AiConfig {
    pub fn validate(&self) -> Result<(), String> {
        let base = self.base_url.trim();
        if !(base.starts_with("https://") || base.starts_with("http://")) {
            return Err("Base URL 必须以 http:// 或 https:// 开头".into());
        }
        if base.chars().any(char::is_whitespace) {
            return Err("Base URL 不能包含空格".into());
        }
        if self.model.trim().is_empty() {
            return Err("模型名称不能为空".into());
        }
        if self
            .temperature
            .is_some_and(|value| !(0.0..=2.0).contains(&value))
        {
            return Err("Temperature 必须在 0 到 2 之间".into());
        }
        if self
            .top_p
            .is_some_and(|value| !(0.0..=1.0).contains(&value))
        {
            return Err("Top P 必须在 0 到 1 之间".into());
        }
        if self
            .max_tokens
            .is_some_and(|value| value == 0 || value > 131_072)
        {
            return Err("最大输出 Token 必须在 1 到 131072 之间".into());
        }
        Ok(())
    }

    fn chat_completions_url(&self) -> String {
        let base = self.base_url.trim().trim_end_matches('/');
        if base.ends_with("/chat/completions") {
            base.to_string()
        } else {
            format!("{}/chat/completions", base)
        }
    }

    fn models_url(&self) -> String {
        let base = self.base_url.trim().trim_end_matches('/');
        if let Some(prefix) = base.strip_suffix("/chat/completions") {
            format!("{}/models", prefix)
        } else {
            format!("{}/models", base)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsView {
    pub enabled: bool,
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub api_key_configured: bool,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWordInput {
    pub word: String,
    pub definition: String,
    pub example_sentence: String,
    pub matched_terms: Vec<String>,
    pub hit_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWordDecision {
    pub word: String,
    #[serde(default = "default_keep")]
    pub keep: bool,
    #[serde(default, alias = "context_definition", alias = "definition")]
    pub context_definition: String,
    #[serde(default, alias = "example_sentence", alias = "rewritten_example")]
    pub example_sentence: String,
}

fn default_keep() -> bool {
    true
}

fn read_setting(conn: &Connection, key: &str) -> Result<String, String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key=?1",
        rusqlite::params![key],
        |row| row.get(0),
    )
    .optional()
    .map(|value| value.unwrap_or_default())
    .map_err(|e| format!("读取 AI 设置失败: {}", e))
}

fn parse_optional_setting<T: std::str::FromStr>(value: String) -> Option<T> {
    let trimmed = value.trim();
    (!trimmed.is_empty())
        .then(|| trimmed.parse().ok())
        .flatten()
}

pub fn load_ai_config(state: &State<'_, DbState>) -> Result<AiConfig, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    Ok(AiConfig {
        enabled: read_setting(&db, KEY_ENABLED)? == "true",
        provider: {
            let value = read_setting(&db, KEY_PROVIDER)?;
            if value.trim().is_empty() {
                "openai".to_string()
            } else {
                value
            }
        },
        base_url: {
            let value = read_setting(&db, KEY_BASE_URL)?;
            if value.trim().is_empty() {
                DEFAULT_BASE_URL.to_string()
            } else {
                value
            }
        },
        api_key: read_setting(&db, KEY_API_KEY)?,
        model: read_setting(&db, KEY_MODEL)?,
        temperature: parse_optional_setting(read_setting(&db, KEY_TEMPERATURE)?),
        top_p: parse_optional_setting(read_setting(&db, KEY_TOP_P)?),
        max_tokens: parse_optional_setting(read_setting(&db, KEY_MAX_TOKENS)?),
    })
}

#[tauri::command]
pub fn get_ai_settings(state: State<DbState>) -> Result<AiSettingsView, String> {
    let config = load_ai_config(&state)?;
    Ok(AiSettingsView {
        enabled: config.enabled,
        provider: config.provider,
        base_url: config.base_url,
        model: config.model,
        api_key_configured: !config.api_key.trim().is_empty(),
        temperature: config.temperature,
        top_p: config.top_p,
        max_tokens: config.max_tokens,
    })
}

#[tauri::command]
pub fn save_ai_settings(
    state: State<DbState>,
    enabled: bool,
    provider: String,
    base_url: String,
    api_key: Option<String>,
    clear_api_key: bool,
    model: String,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_tokens: Option<u32>,
) -> Result<(), String> {
    let candidate = AiConfig {
        enabled,
        provider: if provider.trim().is_empty() {
            "custom".to_string()
        } else {
            provider.trim().to_string()
        },
        base_url: base_url.trim().trim_end_matches('/').to_string(),
        api_key: api_key.clone().unwrap_or_default(),
        model: model.trim().to_string(),
        temperature,
        top_p,
        max_tokens,
    };
    if enabled {
        candidate.validate()?;
    } else if !candidate.base_url.is_empty()
        && !(candidate.base_url.starts_with("https://")
            || candidate.base_url.starts_with("http://"))
    {
        return Err("Base URL 必须以 http:// 或 https:// 开头".into());
    }

    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let previous_provider = read_setting(&db, KEY_PROVIDER)?;
    let previous_base_url = read_setting(&db, KEY_BASE_URL)?;
    let provider_changed =
        !previous_provider.trim().is_empty() && previous_provider.trim() != candidate.provider;
    let endpoint_changed = !previous_base_url.trim().is_empty()
        && previous_base_url.trim().trim_end_matches('/') != candidate.base_url;
    let temperature_value = candidate
        .temperature
        .map(|value| value.to_string())
        .unwrap_or_default();
    let top_p_value = candidate
        .top_p
        .map(|value| value.to_string())
        .unwrap_or_default();
    let max_tokens_value = candidate
        .max_tokens
        .map(|value| value.to_string())
        .unwrap_or_default();
    let tx = db.transaction().map_err(|e| e.to_string())?;
    for (key, value) in [
        (KEY_ENABLED, if enabled { "true" } else { "false" }),
        (KEY_PROVIDER, candidate.provider.as_str()),
        (KEY_BASE_URL, candidate.base_url.as_str()),
        (KEY_MODEL, candidate.model.as_str()),
        (KEY_TEMPERATURE, temperature_value.as_str()),
        (KEY_TOP_P, top_p_value.as_str()),
        (KEY_MAX_TOKENS, max_tokens_value.as_str()),
    ] {
        tx.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )
        .map_err(|e| format!("保存 AI 设置失败: {}", e))?;
    }
    if clear_api_key
        || ((provider_changed || endpoint_changed)
            && api_key.as_deref().map(str::trim).unwrap_or("").is_empty())
    {
        tx.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, '')",
            rusqlite::params![KEY_API_KEY],
        )
        .map_err(|e| format!("清除 API Key 失败: {}", e))?;
    } else if let Some(key) = api_key.filter(|value| !value.trim().is_empty()) {
        tx.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![KEY_API_KEY, key.trim()],
        )
        .map_err(|e| format!("保存 API Key 失败: {}", e))?;
    }
    tx.commit()
        .map_err(|e| format!("提交 AI 设置失败: {}", e))?;
    Ok(())
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

pub(crate) async fn chat_completion(
    config: &AiConfig,
    system: &str,
    user: &str,
    json_mode: bool,
) -> Result<String, String> {
    config.validate()?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(15))
        // A few OpenAI-compatible gateways occasionally terminate long HTTP/2
        // response bodies early. HTTP/1.1 is universally supported by the
        // configured providers and is more reliable for these small requests.
        .http1_only()
        .user_agent("novel-words/0.1")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let payload = build_chat_payload(config, system, user, json_mode);
    const MAX_TRANSPORT_ATTEMPTS: usize = 2;
    let mut completed_body = None;
    for attempt in 1..=MAX_TRANSPORT_ATTEMPTS {
        let mut request = client
            .post(config.chat_completions_url())
            .header(ACCEPT, "application/json")
            // Some compatible gateways return a broken gzip/br stream even when
            // the client did not advertise compression. Identity avoids that
            // interoperability problem and keeps error bodies readable.
            .header(ACCEPT_ENCODING, "identity")
            .header(CONNECTION, "close")
            .json(&payload);
        if !config.api_key.trim().is_empty() {
            request = request.bearer_auth(config.api_key.trim());
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                if attempt < MAX_TRANSPORT_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(800)).await;
                    continue;
                }
                return Err(format!(
                    "连接模型服务失败（已自动重试 {} 次）: {}",
                    MAX_TRANSPORT_ATTEMPTS - 1,
                    error
                ));
            }
        };
        let status = response.status();
        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => {
                if attempt < MAX_TRANSPORT_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(800)).await;
                    continue;
                }
                return Err(format!(
                    "读取模型响应失败（服务端可能中断了传输，已自动重试 {} 次）: {}",
                    MAX_TRANSPORT_ATTEMPTS - 1,
                    error
                ));
            }
        };
        let body = String::from_utf8_lossy(&bytes).into_owned();
        if !status.is_success() {
            let detail: String = body.chars().take(800).collect();
            let suffix = if request_id.is_empty() {
                String::new()
            } else {
                format!(" (request_id: {})", request_id)
            };
            return Err(format!(
                "模型服务返回 HTTP {}{}: {}",
                status, suffix, detail
            ));
        }
        completed_body = Some(body);
        break;
    }
    let body = completed_body.ok_or_else(|| "模型请求未返回响应".to_string())?;

    let value: Value =
        serde_json::from_str(&body).map_err(|e| format!("模型响应不是有效 JSON: {}", e))?;
    let content = value
        .pointer("/choices/0/message/content")
        .ok_or_else(|| "模型响应缺少 choices[0].message.content".to_string())?;
    content_to_text(content).ok_or_else(|| "模型响应内容为空".to_string())
}

/// Builds the chat/completions request body. `json_mode` opts the caller into
/// strict JSON output; it is honoured only for DeepSeek V4 models, where it
/// also keeps the provider rule satisfied: DeepSeek rejects
/// `response_format: json_object` unless the prompt itself mentions "json".
fn build_chat_payload(config: &AiConfig, system: &str, user: &str, json_mode: bool) -> Value {
    let mut payload = json!({
        "model": config.model.trim(),
        "messages": [
            ChatMessage { role: "system", content: system },
            ChatMessage { role: "user", content: user }
        ],
        "stream": false
    });
    if let Some(value) = config.temperature {
        payload["temperature"] = json!(value);
    }
    if let Some(value) = config.top_p {
        payload["top_p"] = json!(value);
    }
    if let Some(value) = config.max_tokens {
        payload["max_tokens"] = json!(value);
    }
    let is_deepseek_v4 = config.provider.eq_ignore_ascii_case("deepseek")
        && config
            .model
            .trim()
            .to_ascii_lowercase()
            .starts_with("deepseek-v4");
    if is_deepseek_v4 {
        // DeepSeek V4 enables thinking by default. Vocabulary validation and
        // similar short tasks pay a large hidden reasoning latency, so
        // thinking is disabled explicitly.
        payload["thinking"] = json!({ "type": "disabled" });
        if config.max_tokens.is_none() {
            payload["max_tokens"] = json!(4096);
        }
        if json_mode {
            payload["response_format"] = json!({ "type": "json_object" });
            // Callers normally describe the expected JSON shape in their
            // prompts; append a fallback instruction when they do not, or the
            // provider rejects the request with HTTP 400.
            if !format!("{}\n{}", system, user)
                .to_ascii_lowercase()
                .contains("json")
            {
                payload["messages"][0]["content"] =
                    json!(format!("{}\nRespond with a single JSON object.", system));
            }
        }
    }
    payload
}

fn content_to_text(content: &Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return Some(text.to_string());
    }
    let parts = content.as_array()?;
    let text = parts
        .iter()
        .filter_map(|part| {
            part.get("text")
                .and_then(|value| value.as_str())
                .or_else(|| part.get("content").and_then(|value| value.as_str()))
        })
        .collect::<Vec<_>>()
        .join("");
    (!text.is_empty()).then_some(text)
}

#[tauri::command]
pub async fn list_ai_models(
    state: State<'_, DbState>,
    provider: String,
    base_url: String,
    api_key: Option<String>,
) -> Result<Vec<String>, String> {
    let saved = load_ai_config(&state)?;
    let use_saved_key = saved.provider == provider
        && saved.base_url.trim().trim_end_matches('/') == base_url.trim().trim_end_matches('/');
    let key = api_key
        .filter(|value| !value.trim().is_empty())
        .or_else(|| use_saved_key.then_some(saved.api_key))
        .unwrap_or_default();
    let config = AiConfig {
        enabled: true,
        provider,
        base_url: base_url.trim().trim_end_matches('/').to_string(),
        api_key: key,
        model: "model-discovery".into(),
        temperature: None,
        top_p: None,
        max_tokens: None,
    };
    config.validate()?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("novel-words/0.1")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let mut request = client.get(config.models_url());
    if !config.api_key.trim().is_empty() {
        request = request.bearer_auth(config.api_key.trim());
    }
    let response = request
        .send()
        .await
        .map_err(|e| format!("获取模型列表失败: {}", e))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("读取模型列表失败: {}", e))?;
    if !status.is_success() {
        let detail: String = body.chars().take(800).collect();
        return Err(format!("模型列表接口返回 HTTP {}: {}", status, detail));
    }

    let value: Value =
        serde_json::from_str(&body).map_err(|e| format!("模型列表不是有效 JSON: {}", e))?;
    let candidates = value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.get("models").and_then(Value::as_array))
        .or_else(|| value.as_array())
        .ok_or_else(|| "模型列表响应中未找到 data 或 models 数组".to_string())?;
    let mut models: Vec<String> = candidates
        .iter()
        .filter_map(|item| {
            item.as_str()
                .map(str::to_string)
                .or_else(|| item.get("id").and_then(Value::as_str).map(str::to_string))
                .or_else(|| item.get("name").and_then(Value::as_str).map(str::to_string))
        })
        .filter(|id| !id.trim().is_empty())
        .collect();
    models.sort_by_key(|id| id.to_lowercase());
    models.dedup();
    if models.is_empty() {
        return Err("服务商返回了空模型列表，可直接手动填写模型 ID".into());
    }
    Ok(models)
}

#[tauri::command]
pub async fn test_ai_connection(
    state: State<'_, DbState>,
    provider: String,
    base_url: String,
    api_key: Option<String>,
    model: String,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_tokens: Option<u32>,
) -> Result<String, String> {
    let saved = load_ai_config(&state)?;
    let config = AiConfig {
        enabled: true,
        provider: provider.clone(),
        base_url: base_url.trim().trim_end_matches('/').to_string(),
        api_key: api_key
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                (saved.provider == provider
                    && saved.base_url.trim().trim_end_matches('/')
                        == base_url.trim().trim_end_matches('/'))
                .then_some(saved.api_key)
            })
            .unwrap_or_default(),
        model: model.trim().to_string(),
        temperature,
        top_p,
        max_tokens,
    };
    let answer = chat_completion(
        &config,
        "You are a connection test. Follow the user's output instruction exactly.",
        "Reply with exactly: OK",
        false,
    )
    .await?;
    Ok(format!("连接成功：{}", answer.trim()))
}

pub async fn enhance_vocab_items<F>(
    config: &AiConfig,
    items: &[AiWordInput],
    mut progress: F,
) -> Result<Vec<AiWordDecision>, String>
where
    F: FnMut(usize, usize),
{
    const BATCH_SIZE: usize = 10;
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let total_batches = items.len().div_ceil(BATCH_SIZE);
    let system = "You validate English vocabulary selected from a Chinese novel. Return one JSON object only. Every contextDefinition must start with the correct English part-of-speech abbreviation copied from the input definition (for example: n., v., adj., adv., pron., conj., prep., modal verb.). You may shorten an example, but must preserve its facts and an exact matched Chinese term. Never invent story facts. Emit compact JSON and never insert raw line breaks or control characters inside string values.";
    let mut decisions = Vec::with_capacity(items.len());

    // Report the AI stage before the first network request. Otherwise the UI
    // remains at the completed local-stage percentage while the model thinks.
    progress(0, total_batches);
    for (batch_index, batch) in items.chunks(BATCH_SIZE).enumerate() {
        let compact: Vec<Value> = batch
            .iter()
            .map(|item| {
                json!({
                    "word": item.word,
                    "definition": item.definition,
                    "exampleSentence": item.example_sentence,
                    "matchedTerms": item.matched_terms,
                    "hitCount": item.hit_count
                })
            })
            .collect();
        let user = format!(
            "Review every item below. `keep` is false only when the matched Chinese term does not express a valid sense of the English word in that sentence. `contextDefinition` must use the format `<part-of-speech> <concise Chinese meaning>` (for example `n. 天赋`), retain the contextually correct part of speech from the input definition, be concise (max 30 characters total), and contain one exact string from matchedTerms. `exampleSentence` must be a natural, concise Chinese rewrite (max 80 Chinese characters) grounded only in the original exampleSentence and contain one exact string from matchedTerms. Return one item per input in the same order as a JSON object shaped exactly like {{\"items\":[{{\"word\":\"...\",\"keep\":true,\"contextDefinition\":\"n. ...\",\"exampleSentence\":\"...\"}}]}}.\n\n{}",
            serde_json::to_string(&compact).map_err(|e| e.to_string())?
        );
        let answer = chat_completion(config, system, &user, true).await?;
        decisions.extend(parse_decisions(&answer)?);
        progress(batch_index + 1, total_batches);
    }
    Ok(decisions)
}

/// 模型偶尔会在 JSON 字符串值里输出未转义的控制字符（典型：重写长例句时
/// 直接在字符串内部换行），严格 JSON 解析会整体失败。把字符串内部的原始
/// 控制字符转义为合法转义序列；字符串外的换行/缩进（格式化空白）原样保留。
fn sanitize_json_control_chars(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 16);
    let mut in_string = false;
    let mut escaped = false;
    for c in raw.chars() {
        if !in_string {
            if c == '"' {
                in_string = true;
            }
            out.push(c);
            continue;
        }
        if escaped {
            // 已有转义符的下一字符原样保留（\n \" \\ 等）
            escaped = false;
            out.push(c);
            continue;
        }
        match c {
            '\\' => {
                escaped = true;
                out.push(c);
            }
            '"' => {
                in_string = false;
                out.push(c);
            }
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn parse_decisions(text: &str) -> Result<Vec<AiWordDecision>, String> {
    let trimmed = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```");
    let trimmed = trimmed.trim_end_matches("```").trim();
    // 先净化字符串值内的原始控制字符（合法 JSON 不受影响），再走严格解析
    let sanitized = sanitize_json_control_chars(trimmed);
    let start = sanitized
        .find('[')
        .ok_or_else(|| "AI 响应中未找到 JSON 数组".to_string())?;
    let end = sanitized
        .rfind(']')
        .ok_or_else(|| "AI 响应中的 JSON 数组不完整".to_string())?;
    if let Ok(decisions) = serde_json::from_str(&sanitized[start..=end]) {
        return Ok(decisions);
    }

    let value: Value =
        serde_json::from_str(&sanitized).map_err(|e| format!("解析 AI 精选结果失败: {}", e))?;
    for key in ["items", "results", "decisions", "data"] {
        if let Some(array) = value.get(key) {
            return serde_json::from_value(array.clone())
                .map_err(|e| format!("解析 AI 精选结果失败: {}", e));
        }
    }
    Err("AI 响应中未找到可识别的精选结果".into())
}

#[cfg(test)]
mod tests {
    use super::{build_chat_payload, parse_decisions, AiConfig};

    fn deepseek_v4_config() -> AiConfig {
        AiConfig {
            enabled: true,
            provider: "deepseek".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            api_key: String::new(),
            model: "deepseek-v4-flash".into(),
            temperature: None,
            top_p: None,
            max_tokens: None,
        }
    }

    #[test]
    fn normalizes_chat_completions_endpoint() {
        let mut config = AiConfig {
            enabled: true,
            provider: "custom".into(),
            base_url: "https://api.example.com/v1/".into(),
            api_key: String::new(),
            model: "model".into(),
            temperature: None,
            top_p: None,
            max_tokens: None,
        };
        assert_eq!(
            config.chat_completions_url(),
            "https://api.example.com/v1/chat/completions"
        );
        config.base_url = "https://api.example.com/v1/chat/completions".into();
        assert_eq!(
            config.chat_completions_url(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn parses_json_from_markdown_fence() {
        let parsed = parse_decisions(
            "```json\n[{\"word\":\"gift\",\"keep\":true,\"contextDefinition\":\"天赋\",\"exampleSentence\":\"他展现了绘画天赋。\"}]\n```",
        )
        .unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].word, "gift");
        assert_eq!(parsed[0].context_definition, "天赋");
        assert_eq!(parsed[0].example_sentence, "他展现了绘画天赋。");
    }

    /// 模型在字符串值内部直接换行（未转义控制字符）——
    /// 「control character found while parsing a string」报错场景
    #[test]
    fn parses_object_with_unescaped_newline_inside_string() {
        let raw = concat!(
            "{\n",
            "  \"items\": [\n",
            "    {\n",
            "      \"word\": \"gift\",\n",
            "      \"keep\": true,\n",
            "      \"contextDefinition\": \"n. 天赋\",\n",
            "      \"exampleSentence\": \"他展现了绘画\n天赋，令人惊叹。\"\n",
            "    }\n",
            "  ]\n",
            "}"
        );
        let parsed = parse_decisions(raw).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].word, "gift");
        assert_eq!(parsed[0].example_sentence, "他展现了绘画\n天赋，令人惊叹。");
    }

    #[test]
    fn parses_array_with_unescaped_newline_inside_string() {
        let parsed = parse_decisions(
            "```json\n[{\"word\":\"gift\",\"keep\":true,\"contextDefinition\":\"n. 绘\n画天赋\",\"exampleSentence\":\"他展现了绘画天赋。\"}]\n```",
        )
        .unwrap();
        assert_eq!(parsed[0].context_definition, "n. 绘\n画天赋");
    }

    /// 字符串外的格式化空白必须原样保留（合法 JSON 不能被净化破坏）
    #[test]
    fn sanitize_keeps_whitespace_outside_strings() {
        let raw = "{\n  \"items\": []\n}";
        let parsed = parse_decisions(raw).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn parses_decisions_from_json_object() {
        let parsed = parse_decisions(
            r#"{"items":[{"word":"gift","keep":true,"contextDefinition":"天赋","exampleSentence":"他展现了绘画天赋。"}]}"#,
        )
        .unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].word, "gift");
    }

    #[test]
    fn json_mode_appends_json_hint_when_prompt_lacks_it() {
        let payload = build_chat_payload(
            &deepseek_v4_config(),
            "You are a connection test.",
            "Reply with exactly: OK",
            true,
        );
        assert_eq!(payload["response_format"]["type"], "json_object");
        let system = payload["messages"][0]["content"].as_str().unwrap();
        assert!(system.to_ascii_lowercase().contains("json"));
    }

    #[test]
    fn json_mode_keeps_prompt_when_it_already_mentions_json() {
        let payload = build_chat_payload(
            &deepseek_v4_config(),
            "Return one JSON object only.",
            "Review every item below.",
            true,
        );
        assert_eq!(payload["response_format"]["type"], "json_object");
        assert_eq!(
            payload["messages"][0]["content"],
            "Return one JSON object only."
        );
    }

    #[test]
    fn connection_test_omits_response_format() {
        let payload = build_chat_payload(
            &deepseek_v4_config(),
            "You are a connection test.",
            "Reply with exactly: OK",
            false,
        );
        assert!(payload.get("response_format").is_none());
        assert_eq!(payload["thinking"]["type"], "disabled");
    }

    #[test]
    fn non_deepseek_payload_has_no_deepseek_extras() {
        let mut config = deepseek_v4_config();
        config.provider = "openai".into();
        config.model = "gpt-4o-mini".into();
        let payload = build_chat_payload(&config, "sys", "user", true);
        assert!(payload.get("response_format").is_none());
        assert!(payload.get("thinking").is_none());
    }
}
