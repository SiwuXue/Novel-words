//! Edge TTS（微软朗读接口）客户端：WebSocket 合成 → MP3 字节。
//!
//! 非公开接口：协议可能变化。合成失败返回 Err，由前端回退系统语音。
//! 参考 edge-tts（Python）社区的协议实现：Sec-MS-GEC 防护令牌 + SSML 请求。

use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const WSS_URL: &str = "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1";
const CHROMIUM_VERSION: &str = "143.0.3650.75";
const AUDIO_FORMAT: &str = "audio-24khz-48kbitrate-mono-mp3";
const CHROMIUM_MAJOR_VERSION: &str = "143";

/// 内置音色清单（前端按语言过滤展示）。
pub const VOICES: &[(&str, &str, &str)] = &[
    ("zh-CN-XiaoxiaoNeural", "晓晓（女·普通话）", "zh"),
    ("zh-CN-XiaoyiNeural", "晓伊（女·普通话）", "zh"),
    ("zh-CN-YunjianNeural", "云健（男·普通话）", "zh"),
    ("zh-CN-YunxiNeural", "云希（男·普通话）", "zh"),
    ("zh-CN-YunyangNeural", "云扬（男·新闻）", "zh"),
    ("en-US-AriaNeural", "Aria（Female·US）", "en"),
    ("en-US-AnaNeural", "Ana（Female·US·Child）", "en"),
    ("en-US-ChristopherNeural", "Christopher（Male·US）", "en"),
    ("en-US-EricNeural", "Eric（Male·US）", "en"),
    ("en-US-GuyNeural", "Guy（Male·US）", "en"),
    ("en-GB-SoniaNeural", "Sonia（Female·UK）", "en"),
    ("en-GB-RyanNeural", "Ryan（Male·UK）", "en"),
];

/// Sec-MS-GEC 防护令牌：Windows 纪元时间按 5 分钟窗口取整后
/// 拼接 TrustedClientToken 做 SHA256，十六进制大写。
fn sec_ms_gec(now_secs: u64) -> String {
    let win_epoch_secs = now_secs + 11_644_473_600;
    let window = 300u64 * 10_000_000;
    let ticks = win_epoch_secs * 10_000_000 / window * window;
    let input = format!("{ticks}{TRUSTED_CLIENT_TOKEN}");
    let digest = Sha256::digest(input.as_bytes());
    digest.iter().map(|b| format!("{b:02X}")).collect()
}

fn current_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 16 字节随机数 → 32 字符小写 hex（ConnectionId / X-RequestId 同值）。
fn random_connection_id() -> String {
    let bytes = *uuid::Uuid::new_v4().as_bytes();
    hex::encode(bytes)
}

/// 32 字符随机大写 hex（Cookie muid，对照 ColorTxt 实现）。
fn random_muid() -> String {
    let bytes = *uuid::Uuid::new_v4().as_bytes();
    hex::encode(bytes).to_uppercase()
}

fn wss_url() -> String {
    format!(
        "{WSS_URL}?TrustedClientToken={TRUSTED_CLIENT_TOKEN}&Sec-MS-GEC={}&Sec-MS-GEC-Version=1-{CHROMIUM_VERSION}&ConnectionId={}",
        sec_ms_gec(current_secs()),
        random_connection_id()
    )
}

fn escape_ssml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// 构造 SSML（独立出来便于单测）。
fn build_ssml(text: &str, voice: &str, rate: i32, pitch: i32, volume: i32) -> String {
    format!(
        "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'><voice name='{}'><prosody pitch='{}Hz' rate='{}%' volume='{}%'>{}</prosody></voice></speak>",
        voice,
        pitch,   // "+0" / "-20" / "10" (Hz)
        rate,    // "+0" / "50" / "-30" (%)
        volume,  // "+0" / "-50" / "100" (%)
        escape_ssml(text)
    )
}

#[derive(Debug, PartialEq)]
enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
}

/// 合并 Path=audio 的二进制分片：2 字节大头序头长度 + 头部（含 Path:audio）+ 音频数据。
fn collect_audio(messages: &[WsMessage]) -> Vec<u8> {
    let mut audio = Vec::new();
    for msg in messages {
        if let WsMessage::Binary(bytes) = msg {
            if bytes.len() > 2 {
                let header_len = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
                if bytes.len() > header_len + 2 {
                    let header = &bytes[2..2 + header_len];
                    if header.windows(10).any(|w| w == b"Path:audio") {
                        audio.extend_from_slice(&bytes[2 + header_len..]);
                    }
                }
            }
        }
    }
    audio
}

/// Edge TTS 合成：文本 → MP3 字节（带 3 次重试，间隔 250ms 递增，对照 ColorTxt）。
pub async fn synthesize(text: &str, voice: &str, rate: i32, pitch: i32, volume: i32) -> Result<Vec<u8>, String> {
    const MAX_ATTEMPTS: usize = 3;
    let mut last_err = String::new();
    for attempt in 1..=MAX_ATTEMPTS {
        match synthesize_once(text, voice, rate, pitch, volume).await {
            Ok(bytes) => return Ok(bytes),
            Err(e) => {
                last_err = e;
                let retryable = !last_err.contains("无可朗读内容")
                    && (last_err.contains("无音频")
                        || last_err.contains("响应未完成")
                        || last_err.contains("连接")
                        || last_err.contains("中断")
                        || last_err.contains("超时")
                        || last_err.contains("WebSocket"));
                if !retryable || attempt == MAX_ATTEMPTS {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(250 * attempt as u64)).await;
            }
        }
    }
    Err(last_err)
}

/// Edge TTS 单次合成（对照 ColorTxt voiceReadEdgeTts.ts 的连接参数与消息流）。
async fn synthesize_once(
    text: &str,
    voice: &str,
    rate: i32,
    pitch: i32,
    volume: i32,
) -> Result<Vec<u8>, String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::Message;

    let connection_id = random_connection_id();
    let url = wss_url();
    let mut request = url
        .into_client_request()
        .map_err(|e| format!("Edge TTS 请求构造失败: {}", e))?;
    let headers = request.headers_mut();
    headers.insert(
        "Origin",
        "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold".parse().unwrap(),
    );
    headers.insert(
        "User-Agent",
        format!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{}.0.0.0 Safari/537.36 Edg/{}.0.0.0",
            CHROMIUM_MAJOR_VERSION, CHROMIUM_MAJOR_VERSION
        )
        .parse()
        .unwrap(),
    );
    headers.insert(
        "Cookie",
        format!("muid={};", random_muid()).parse().unwrap(),
    );
    headers.insert("Accept-Encoding", "gzip, deflate, br, zstd".parse().unwrap());
    headers.insert("Accept-Language", "en-US,en;q=0.9".parse().unwrap());
    headers.insert("Pragma", "no-cache".parse().unwrap());
    headers.insert("Cache-Control", "no-cache".parse().unwrap());

    let (ws, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| format!("Edge TTS 连接失败: {}（将回退系统语音）", e))?;
    let (mut sink, mut stream) = ws.split();

    let timestamp = chrono::Utc::now().format("%a %b %d %Y %H:%M:%S GMT+0000 (Coordinated Universal Time)");
    let config = format!(
        "X-Timestamp:{timestamp}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"true\"}},\"outputFormat\":\"{AUDIO_FORMAT}\"}}}}}}}}"
    );
    // X-RequestId 与 ConnectionId 同值（对照 ColorTxt）
    let request_id = connection_id;
    let ssml = format!(
        "X-RequestId:{request_id}\r\nContent-Type:application/ssml+xml\r\nX-Timestamp:{timestamp}Z\r\nPath:ssml\r\n\r\n{}",
        build_ssml(text, voice, rate, pitch, volume)
    );

    sink.send(Message::Text(config))
        .await
        .map_err(|e| format!("Edge TTS 发送失败: {}", e))?;
    sink.send(Message::Text(ssml))
        .await
        .map_err(|e| format!("Edge TTS 发送失败: {}", e))?;

    let mut messages: Vec<WsMessage> = Vec::new();
    let mut response_body = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(Message::Text(t)) => {
                if t.contains("Path:turn.end") {
                    break;
                }
                if t.contains("Path:response") {
                    if let Some(idx) = t.find("\r\n\r\n") {
                        response_body = t[idx + 4..].to_string();
                    }
                }
                messages.push(WsMessage::Text(t));
            }
            Ok(Message::Binary(b)) => messages.push(WsMessage::Binary(b)),
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(e) => {
                // 连接中断但已收到音频：视为成功（对照 ColorTxt close 处理）
                let audio = collect_audio(&messages);
                if !audio.is_empty() {
                    return Ok(audio);
                }
                return Err(format!("Edge TTS 连接中断: {}（将回退系统语音）", e));
            }
        }
    }
    // Close / turn.end 收尾：只要收集到音频即成功
    let audio = collect_audio(&messages);
    if !audio.is_empty() {
        return Ok(audio);
    }
    if !response_body.is_empty() {
        let detail: String = response_body.chars().take(200).collect();
        return Err(format!("Edge TTS 失败，服务端响应: {detail}"));
    }
    Err("Edge TTS 响应未完成且无音频（将回退系统语音）".into())
}

#[tauri::command]
pub async fn tts_synthesize(
    text: String,
    voice: String,
    rate: i32,
    pitch: i32,
    volume: i32,
) -> Result<Vec<u8>, String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("朗读内容为空".into());
    }
    if text.chars().count() > 1200 {
        return Err("单次朗读文本过长，请按句切分".into());
    }
    tokio::time::timeout(
        std::time::Duration::from_secs(45),
        synthesize(&text, &voice, rate, pitch, volume),
    )
    .await
    .map_err(|_| "Edge TTS 合成超时（将回退系统语音）".to_string())?
}

/// 内置音色清单：[(id, 名称, 语言)]
#[tauri::command]
pub fn tts_voices() -> Vec<(String, String, String)> {
    VOICES
        .iter()
        .map(|(id, name, lang)| (id.to_string(), name.to_string(), lang.to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// 云服务商 TTS（REST）：阿里云 DashScope Qwen-TTS / MiniMax T2A v2
// ---------------------------------------------------------------------------

const DASHSCOPE_TTS_URL: &str =
    "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation";
const DASHSCOPE_TTS_MODEL: &str = "qwen3-tts-flash";
const MINIMAX_TTS_URL: &str = "https://api.minimaxi.com/v1/t2a_v2";
const MINIMAX_TTS_MODEL: &str = "speech-2.8-hd";

/// DashScope / MiniMax 预置音色清单（id 可自由填写，不限于清单）。
pub const CLOUD_VOICES: &[(&str, &str, &str, &str)] = &[
    // (provider, id, 名称, 语言)
    ("dashscope", "Cherry", "Cherry（女·知性）", "zh"),
    ("dashscope", "Serena", "Serena（女·清亮）", "zh"),
    ("dashscope", "Ethan", "Ethan（男·醇厚）", "zh"),
    ("dashscope", "Chelsie", "Chelsie（女·活泼）", "zh"),
    ("dashscope", "Aria", "Aria（Female·EN）", "en"),
    ("minimax", "female-shaonv", "少女（女）", "zh"),
    ("minimax", "female-yujie", "御姐（女）", "zh"),
    ("minimax", "female-chengshu", "成熟女性（女）", "zh"),
    ("minimax", "male-qn-qingse", "青涩青年（男）", "zh"),
    ("minimax", "male-qn-jingying", "精英青年（男）", "zh"),
    ("minimax", "male-qn-bada", "霸道青年（男）", "zh"),
    ("minimax", "presenter_female", "女主播（女）", "zh"),
];

#[tauri::command]
pub fn tts_cloud_voices(provider: String) -> Vec<(String, String, String)> {
    CLOUD_VOICES
        .iter()
        .filter(|(p, _, _, _)| *p == provider)
        .map(|(_, id, name, lang)| (id.to_string(), name.to_string(), lang.to_string()))
        .collect()
}

/// 按字符占比猜测 language_type：DashScope 指定语种比 Auto 合成质量更好。
fn guess_language_type(text: &str) -> &'static str {
    let total = text.chars().count().max(1);
    let cjk = text
        .chars()
        .filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c))
        .count();
    if cjk * 4 >= total {
        "Chinese"
    } else {
        "English"
    }
}

/// DashScope Qwen-TTS 非流式合成：POST → output.audio.url（24h 有效）→ 下载。
async fn synthesize_dashscope(api_key: &str, text: &str, voice: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .http1_only()
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let payload = serde_json::json!({
        "model": DASHSCOPE_TTS_MODEL,
        "input": {
            "text": text,
            "voice": voice,
            "language_type": guess_language_type(text),
        }
    });
    let resp = client
        .post(DASHSCOPE_TTS_URL)
        .bearer_auth(api_key.trim())
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("DashScope 请求失败: {}", e))?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.map_err(|e| format!("DashScope 响应解析失败: {}", e))?;
    if !status.is_success() {
        let msg = body["message"].as_str().unwrap_or("未知错误");
        return Err(format!("DashScope 合成失败（{}）: {}", status, msg));
    }
    let audio_url = body["output"]["audio"]["url"]
        .as_str()
        .filter(|u| !u.is_empty())
        .ok_or_else(|| format!("DashScope 未返回音频 URL: {}", body))?;
    let audio = client
        .get(audio_url)
        .send()
        .await
        .map_err(|e| format!("DashScope 音频下载失败: {}", e))?;
    if !audio.status().is_success() {
        return Err(format!("DashScope 音频下载失败（{}）", audio.status()));
    }
    let bytes = audio.bytes().await.map_err(|e| format!("DashScope 音频读取失败: {}", e))?;
    let bytes = bytes.to_vec();
    if bytes.is_empty() {
        return Err("DashScope 返回空音频".into());
    }
    Ok(bytes)
}

/// MiniMax T2A v2 合成：POST → data.audio（hex 编码 MP3）→ 解码。
/// 新 API（api.minimaxi.com/v1/t2a_v2）仅需 Bearer API Key，GroupId 已不再需要；
/// 仅在用户填写了 GroupId 时才拼 `?GroupId=`（兼容老账号/老控制台习惯）。
async fn synthesize_minimax(
    api_key: &str,
    group_id: &str,
    text: &str,
    voice: &str,
    rate: f64,
) -> Result<Vec<u8>, String> {
    let group_id = group_id.trim();
    let url = if group_id.is_empty() {
        MINIMAX_TTS_URL.to_string()
    } else {
        format!("{MINIMAX_TTS_URL}?GroupId={group_id}")
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .http1_only()
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let speed = if rate.is_finite() { rate.clamp(0.5, 2.0) } else { 1.0 };
    let payload = serde_json::json!({
        "model": MINIMAX_TTS_MODEL,
        "text": text,
        "stream": false,
        "voice_setting": {
            "voice_id": voice,
            "speed": speed,
            "vol": 1.0,
            "pitch": 0
        },
        "audio_setting": {
            "sample_rate": 32000,
            "bitrate": 128000,
            "format": "mp3",
            "channel": 1
        }
    });
    let resp = client
        .post(url)
        .bearer_auth(api_key.trim())
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("MiniMax 请求失败: {}", e))?;
    let body: serde_json::Value = resp.json().await.map_err(|e| format!("MiniMax 响应解析失败: {}", e))?;
    let status_code = body["base_resp"]["status_code"].as_i64().unwrap_or(-1);
    if status_code != 0 {
        let msg = body["base_resp"]["status_msg"].as_str().unwrap_or("未知错误");
        return Err(format!("MiniMax 合成失败（{}）: {}", status_code, msg));
    }
    let hex_audio = body["data"]["audio"].as_str().unwrap_or_default();
    if hex_audio.is_empty() {
        return Err("MiniMax 返回空音频".into());
    }
    hex::decode(hex_audio).map_err(|e| format!("MiniMax 音频解码失败: {}", e))
}

/// 云服务商合成命令：provider = dashscope | minimax。
#[tauri::command]
pub async fn tts_synthesize_cloud(
    provider: String,
    api_key: String,
    group_id: Option<String>,
    text: String,
    voice: String,
) -> Result<Vec<u8>, String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("朗读内容为空".into());
    }
    if api_key.trim().is_empty() {
        return Err(format!("{} 需要 API Key（在设置页填写）", provider));
    }
    let fut: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<u8>, String>> + Send>,
    > = match provider.as_str() {
        "dashscope" => Box::pin(synthesize_dashscope(&api_key, &text, &voice)),
        "minimax" => Box::pin(synthesize_minimax(
            &api_key,
            group_id.as_deref().unwrap_or(""),
            &text,
            &voice,
            1.0,
        )),
        other => return Err(format!("不支持的 TTS 服务商: {}", other)),
    };
    tokio::time::timeout(std::time::Duration::from_secs(75), fut)
        .await
        .map_err(|_| format!("{} 合成超时（将回退系统语音）", provider))?
}

/// 连接测试：按当前服务商合成一句短文本验证连通性，成功返回音频字节数描述。
#[tauri::command]
pub async fn tts_test_connection(
    provider: String,
    api_key: Option<String>,
    group_id: Option<String>,
    voice: String,
) -> Result<String, String> {
    let text = "你好";
    let key = api_key.unwrap_or_default();
    let fut: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<u8>, String>> + Send>,
    > = match provider.as_str() {
        "edge" => Box::pin(synthesize(text, &voice, 0, 0, 0)),
        "dashscope" => Box::pin(synthesize_dashscope(&key, text, &voice)),
        "minimax" => Box::pin(synthesize_minimax(
            &key,
            group_id.as_deref().unwrap_or(""),
            text,
            &voice,
            1.0,
        )),
        "volcengine" => Box::pin(synthesize_volcengine(&key, text, &voice, 1.0)),
        "mimo" => Box::pin(synthesize_mimo(&key, text, &voice, 1.0)),
        "sapi" => Box::pin(async move {
            crate::sapi::sapi_synthesize_blocking(text, &voice, 1.0)
        }),
        other => return Err(format!("不支持的 TTS 服务商: {}", other)),
    };
    let bytes = tokio::time::timeout(std::time::Duration::from_secs(30), fut)
        .await
        .map_err(|_| format!("{} 连接测试超时", provider))??;
    Ok(format!("ok {}B", bytes.len()))
}

// ---------------------------------------------------------------------------
// 火山引擎（豆包语音合成大模型 2.0，REST unidirectional，对照 ColorTxt）
// ---------------------------------------------------------------------------

const VOLCENGINE_TTS_URL: &str =
    "https://openspeech.bytedance.com/api/v3/tts/unidirectional";
const VOLCENGINE_RESOURCE_ID: &str = "seed-tts-2.0";

/// PCM(16bit 单语) → WAV（44 字节 RIFF 头，便于前端 <audio> 直接播放）。
pub fn wrap_wav_header(pcm: &[u8], sample_rate: u32, channels: u16, bits_per_sample: u16) -> Vec<u8> {
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let data_len = pcm.len() as u32;
    let mut out = Vec::with_capacity(44 + pcm.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt 块长度
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits_per_sample.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

/// 火山 unidirectional 响应逐行解析（NDJSON）：code=0 收集 base64 音频分片，
/// 20000000 完成，其他码报错。独立 fn 便于单测。
fn parse_volcengine_ndjson(body: &str) -> Result<Vec<u8>, String> {
    use base64::Engine as _;
    let mut pcm: Vec<u8> = Vec::new();
    let mut finished = false;
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| format!("火山引擎响应解析失败: {}（行: {}）", e, &line[..line.len().min(120)]))?;
        let code = v["code"].as_i64().unwrap_or(-1);
        if code == 20000000 {
            finished = true;
            continue;
        }
        if code != 0 {
            let msg = v["message"].as_str().unwrap_or("未知错误");
            return Err(format!("火山引擎合成失败（{}）: {}", code, msg));
        }
        if let Some(data) = v["data"].as_str() {
            if !data.is_empty() {
                let chunk = base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .map_err(|e| format!("火山引擎音频解码失败: {}", e))?;
                pcm.extend_from_slice(&chunk);
            }
        }
    }
    if pcm.is_empty() {
        return Err(if finished {
            "火山引擎返回空音频".into()
        } else {
            "火山引擎响应未完成且无音频".into()
        });
    }
    Ok(pcm)
}

async fn synthesize_volcengine(
    api_key: &str,
    text: &str,
    voice: &str,
    rate: f64,
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .http1_only()
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    // 语速 0.5–2.0 → speech_rate -50~100（对照 ColorTxt）
    let speech_rate = ((rate - 1.0) * 100.0).round().clamp(-50.0, 100.0) as i64;
    let payload = serde_json::json!({
        "user": { "uid": "novel-words" },
        "req_params": {
            "text": text,
            "speaker": voice,
            "audio_params": {
                "format": "pcm",
                "sample_rate": 24000,
                "speech_rate": speech_rate
            }
        }
    });
    let resp = client
        .post(VOLCENGINE_TTS_URL)
        .header("X-Api-Key", api_key.trim())
        .header("X-Api-Resource-Id", VOLCENGINE_RESOURCE_ID)
        .header("X-Api-Request-Id", uuid::Uuid::new_v4().to_string())
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("火山引擎请求失败: {}", e))?;
    let status = resp.status();
    let body = resp.text().await.map_err(|e| format!("火山引擎响应读取失败: {}", e))?;
    if !status.is_success() {
        let detail: String = body.chars().take(300).collect();
        return Err(format!("火山引擎合成失败（{}）: {}", status, detail));
    }
    let pcm = parse_volcengine_ndjson(&body)?;
    Ok(wrap_wav_header(&pcm, 24000, 1, 16))
}

// ---------------------------------------------------------------------------
// 小米 MiMo（OpenAI 兼容 chat/completions 出音频，对照 ColorTxt）
// ---------------------------------------------------------------------------

const MIMO_TTS_MODEL: &str = "mimo-v2.5-tts";

/// 按密钥前缀解析 MiMo API base：tp- → Token Plan 中国集群，其余 → 按量付费。
fn mimo_api_root(api_key: &str) -> &'static str {
    if api_key.trim().to_ascii_lowercase().starts_with("tp-") {
        "https://token-plan-cn.xiaomimimo.com/v1"
    } else {
        "https://api.xiaomimimo.com/v1"
    }
}

/// 语速提示（MiMo 无 numeric rate 参数，以自然语言提示控制）。
fn mimo_rate_hint(rate: f64) -> Option<&'static str> {
    if !(rate - 1.0).is_finite() || (rate - 1.0).abs() < 0.08 {
        return None;
    }
    if rate <= 0.75 {
        Some("语速较慢，节奏舒缓。")
    } else if rate >= 1.25 {
        Some("语速较快，节奏紧凑。")
    } else {
        None
    }
}

async fn synthesize_mimo(api_key: &str, text: &str, voice: &str, rate: f64) -> Result<Vec<u8>, String> {
    use base64::Engine as _;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .http1_only()
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let mut messages = Vec::new();
    if let Some(hint) = mimo_rate_hint(rate) {
        messages.push(serde_json::json!({ "role": "user", "content": hint }));
    }
    messages.push(serde_json::json!({ "role": "assistant", "content": text }));
    let payload = serde_json::json!({
        "model": MIMO_TTS_MODEL,
        "messages": messages,
        "audio": { "format": "wav", "voice": voice }
    });
    let resp = client
        .post(format!("{}/chat/completions", mimo_api_root(api_key)))
        .header("api-key", api_key.trim())
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("MiMo 请求失败: {}", e))?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.map_err(|e| format!("MiMo 响应解析失败: {}", e))?;
    if !status.is_success() {
        let msg = body["error"]["message"]
            .as_str()
            .or_else(|| body["message"].as_str())
            .unwrap_or("未知错误");
        return Err(format!("MiMo 合成失败（{}）: {}", status, msg));
    }
    let b64 = body["choices"][0]["message"]["audio"]["data"]
        .as_str()
        .unwrap_or_default();
    if b64.is_empty() {
        return Err("MiMo 返回空音频".into());
    }
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("MiMo 音频解码失败: {}", e))
}

// ---------------------------------------------------------------------------
// MiniMax 动态音色（POST /v1/get_voice，失败由前端回退静态表）
// ---------------------------------------------------------------------------

/// MiniMax 音色目录拉取：system_voice/voice_cloning/voice_generation 合并返回
/// [(id, label, group, description)]。
pub async fn minimax_fetch_voices(api_key: &str) -> Result<Vec<(String, String, String, String)>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .http1_only()
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let resp = client
        .post("https://api.minimaxi.com/v1/get_voice")
        .bearer_auth(api_key.trim())
        .json(&serde_json::json!({ "voice_type": "all" }))
        .send()
        .await
        .map_err(|e| format!("MiniMax 音色拉取失败: {}", e))?;
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("MiniMax 音色响应解析失败: {}", e))?;
    let status_code = body["base_resp"]["status_code"].as_i64().unwrap_or(-1);
    if status_code != 0 {
        let msg = body["base_resp"]["status_msg"].as_str().unwrap_or("未知错误");
        return Err(format!("MiniMax 音色拉取失败（{}）: {}", status_code, msg));
    }
    let mut out: Vec<(String, String, String, String)> = Vec::new();
    for (key, group) in [
        ("system_voice", "系统音色"),
        ("voice_cloning", "快速复刻"),
        ("voice_generation", "文生音色"),
    ] {
        if let Some(arr) = body[key].as_array() {
            for v in arr {
                let id = v["voice_id"].as_str().unwrap_or_default().trim().to_string();
                if id.is_empty() {
                    continue;
                }
                let name = v["voice_name"].as_str().unwrap_or_default().trim().to_string();
                let desc = v["description"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|d| d.as_str())
                    .unwrap_or_default()
                    .to_string();
                let label = if name.is_empty() { id.clone() } else { name };
                out.push((id, label, group.to_string(), desc));
            }
        }
    }
    if out.is_empty() {
        return Err("MiniMax 音色列表为空".into());
    }
    Ok(out)
}

#[tauri::command]
pub async fn tts_voices_v3(
    provider: String,
    api_key: Option<String>,
) -> Result<Vec<(String, String, String)>, String> {
    if provider == "minimax" {
        let key = api_key.unwrap_or_default();
        if key.trim().is_empty() {
            return Ok(Vec::new()); // 无 Key：前端回退静态表
        }
        match minimax_fetch_voices(&key).await {
            Ok(list) => {
                return Ok(list
                    .into_iter()
                    .map(|(id, label, group, desc)| (id, label, format!("{group} · {desc}")))
                    .collect())
            }
            Err(_) => return Ok(Vec::new()), // 拉取失败：前端回退静态表
        }
    }
    Ok(Vec::new())
}

/// SAPI5 本机音色枚举（仅 Windows；spawn_blocking 避免阻塞异步运行时）。
#[tauri::command]
pub async fn tts_sapi_voices() -> Result<Vec<(String, String, String, String)>, String> {
    tokio::task::spawn_blocking(crate::sapi::sapi_list_voices_blocking)
        .await
        .map_err(|e| format!("SAPI 任务失败: {}", e))?
}

/// 统一合成命令（v3）：所有在线/本地服务商单一入口，rate/pitch 为倍率（1.0 正常）。
#[tauri::command]
pub async fn tts_synthesize_v3(
    provider: String,
    api_key: Option<String>,
    group_id: Option<String>,
    text: String,
    voice: String,
    rate: f64,
    pitch: f64,
    volume: i32,
) -> Result<Vec<u8>, String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("朗读内容为空".into());
    }
    let key = api_key.unwrap_or_default();
    let rate = if rate.is_finite() && rate > 0.0 { rate } else { 1.0 };
    let fut: std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<u8>, String>> + Send>,
    > = match provider.as_str() {
        "edge" => Box::pin(synthesize(
            &text,
            &voice,
            ((rate - 1.0) * 100.0).round() as i32,
            ((pitch - 1.0) * 100.0).round() as i32,
            volume - 100,
        )),
        "dashscope" => Box::pin(synthesize_dashscope(&key, &text, &voice)),
        "minimax" => Box::pin(synthesize_minimax(
            &key,
            group_id.as_deref().unwrap_or(""),
            &text,
            &voice,
            rate,
        )),
        "volcengine" => Box::pin(synthesize_volcengine(&key, &text, &voice, rate)),
        "mimo" => Box::pin(synthesize_mimo(&key, &text, &voice, rate)),
        "sapi" => Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                crate::sapi::sapi_synthesize_blocking(&text, &voice, rate)
            })
            .await
            .map_err(|e| format!("SAPI 任务失败: {}", e))?
        }),
        other => return Err(format!("不支持的 TTS 服务商: {}", other)),
    };
    tokio::time::timeout(std::time::Duration::from_secs(90), fut)
        .await
        .map_err(|_| format!("{} 合成超时（将回退系统语音）", provider))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssml_escapes_and_formats() {
        let ssml = build_ssml("Tom & Jerry <3", "zh-CN-XiaoxiaoNeural", 0, 0, 0);
        assert!(ssml.contains("Tom &amp; Jerry &lt;3"));
        assert!(ssml.contains("voice name='zh-CN-XiaoxiaoNeural'"));
        assert!(ssml.contains("rate='0%'"));
    }

    #[test]
    fn sec_ms_gec_is_deterministic_uppercase_hex() {
        // 固定时间：2026-09-18 00:00:00 UTC → 1785312000 unix secs
        let secs = 1_785_312_000u64;
        let gec = sec_ms_gec(secs);
        assert_eq!(gec.len(), 64);
        assert!(gec.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase()));
        // 相同 5 分钟窗口内结果一致
        assert_eq!(gec, sec_ms_gec(secs + 299));
    }

    /// 真实联网合成验证：默认忽略，手动 `cargo test --lib tts -- --ignored` 运行。
    #[test]
    #[ignore = "requires network"]
    fn edge_tts_real_synthesis() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let audio = rt
            .block_on(synthesize("你好，这是词阅的朗读测试。", "zh-CN-XiaoxiaoNeural", 0, 0, 0))
            .expect("synthesis should succeed");
        assert!(audio.len() > 1000, "MP3 字节量: {}", audio.len());
        assert_eq!(&audio[..3], b"ID3");
    }

    #[test]
    fn collect_audio_skips_headers_and_metadata() {
        let header = b"X-RequestId:abc\r\nPath:audio\r\n";
        let mut bin = (header.len() as u16).to_be_bytes().to_vec();
        bin.extend_from_slice(header);
        bin.extend_from_slice(b"MP3DATA");
        let mut meta = vec![0u8, 4];
        meta.extend_from_slice(b"Meta");
        let messages = vec![
            WsMessage::Text("{\"meta\":\"data\"}".into()),
            WsMessage::Binary(meta),
            WsMessage::Binary(bin),
        ];
        assert_eq!(collect_audio(&messages), b"MP3DATA");
    }

    #[test]
    fn language_type_guesses_by_cjk_ratio() {
        assert_eq!(guess_language_type("你好，这是词阅的朗读测试。"), "Chinese");
        assert_eq!(guess_language_type("It was a quiet morning in the valley."), "English");
        // 混合少量汉字的英文文本仍判为英文
        assert_eq!(
            guess_language_type("This is English text with 汉字 few."),
            "English"
        );
    }

    #[test]
    fn wav_header_wraps_pcm_with_correct_fields() {
        let pcm = vec![0u8; 4800]; // 0.1s @24kHz 16bit mono
        let wav = wrap_wav_header(&pcm, 24000, 1, 16);
        assert_eq!(wav.len(), 44 + pcm.len());
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(u32::from_le_bytes(wav[16..20].try_into().unwrap()), 16);
        assert_eq!(u16::from_le_bytes(wav[20..22].try_into().unwrap()), 1); // PCM
        assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 1); // mono
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 24000);
        assert_eq!(u32::from_le_bytes(wav[28..32].try_into().unwrap()), 48000); // byte rate
        assert_eq!(u16::from_le_bytes(wav[32..34].try_into().unwrap()), 2); // block align
        assert_eq!(u16::from_le_bytes(wav[34..36].try_into().unwrap()), 16);
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 4800);
        assert_eq!(&wav[44..], &pcm[..]);
    }

    #[test]
    fn volcengine_ndjson_collects_fragments_and_reports_errors() {
        use base64::Engine as _;
        let chunk1 = base64::engine::general_purpose::STANDARD.encode([1u8, 2, 3]);
        let chunk2 = base64::engine::general_purpose::STANDARD.encode([4u8, 5]);
        let body = format!(
            "{{\"code\":0,\"data\":\"{chunk1}\"}}\n{{\"code\":0,\"data\":\"{chunk2}\"}}\n{{\"code\":20000000,\"message\":\"success\"}}\n"
        );
        let pcm = parse_volcengine_ndjson(&body).unwrap();
        assert_eq!(pcm, vec![1, 2, 3, 4, 5]);

        // 错误码
        let err = parse_volcengine_ndjson("{\"code\":3001,\"message\":\"bad key\"}");
        assert!(err.unwrap_err().contains("bad key"));

        // 空 data 行跳过；无音频报错
        assert!(parse_volcengine_ndjson("{\"code\":0,\"data\":\"\"}").is_err());
        // 空行跳过
        let with_blank = format!("\n{{\"code\":0,\"data\":\"{chunk1}\"}}\n\n{{\"code\":20000000}}\n");
        assert_eq!(parse_volcengine_ndjson(&with_blank).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn mimo_base_resolved_from_key_prefix() {
        assert_eq!(
            mimo_api_root("tp-abc123"),
            "https://token-plan-cn.xiaomimimo.com/v1"
        );
        assert_eq!(
            mimo_api_root("sk-abc123"),
            "https://api.xiaomimimo.com/v1"
        );
    }

    #[test]
    fn mimo_rate_hint_only_for_notable_deviation() {
        assert!(mimo_rate_hint(1.0).is_none());
        assert!(mimo_rate_hint(1.1).is_none());
        assert_eq!(mimo_rate_hint(0.6), Some("语速较慢，节奏舒缓。"));
        assert_eq!(mimo_rate_hint(1.5), Some("语速较快，节奏紧凑。"));
    }

    /// 真实云合成验证（需要有效 Key，手动运行时先填入）。
    #[test]
    #[ignore = "requires network + api key"]
    fn dashscope_real_synthesis() {
        let key = std::env::var("DASHSCOPE_API_KEY").unwrap_or_default();
        if key.is_empty() {
            return;
        }
        let rt = tokio::runtime::Runtime::new().unwrap();
        let audio = rt
            .block_on(synthesize_dashscope(&key, "你好，这是词阅的朗读测试。", "Cherry"))
            .expect("dashscope synthesis should succeed");
        assert!(audio.len() > 1000, "音频字节量: {}", audio.len());
    }
}
