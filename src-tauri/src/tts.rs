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

fn wss_url() -> String {
    format!(
        "{WSS_URL}?TrustedClientToken={TRUSTED_CLIENT_TOKEN}&Sec-MS-GEC={}&Sec-MS-GEC-Version=1-{CHROMIUM_VERSION}",
        sec_ms_gec(current_secs())
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

/// Edge TTS 合成：文本 → MP3 字节。
pub async fn synthesize(text: &str, voice: &str, rate: i32, pitch: i32, volume: i32) -> Result<Vec<u8>, String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::Message;

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
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{}.0.0.0 Safari/537.36 Edg/{}",
            CHROMIUM_MAJOR_VERSION, CHROMIUM_VERSION
        )
        .parse()
        .unwrap(),
    );
    headers.insert(
        "Sec-CH-UA",
        format!(
            "\" Not;A Brand\";v=\"99\", \"Microsoft Edge\";v=\"{}\", \"Chromium\";v=\"{}\"",
            CHROMIUM_MAJOR_VERSION, CHROMIUM_MAJOR_VERSION
        )
        .parse()
        .unwrap(),
    );

    let (ws, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| format!("Edge TTS 连接失败: {}（将回退系统语音）", e))?;
    let (mut sink, mut stream) = ws.split();

    let timestamp = chrono::Utc::now().format("%a %b %d %Y %H:%M:%S GMT+0000 (Coordinated Universal Time)");
    let config = format!(
        "X-Timestamp:{timestamp}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{{\"context\":{{\"synthesis\":{{\"audio\":{{\"metadataoptions\":{{\"sentenceBoundaryEnabled\":\"false\",\"wordBoundaryEnabled\":\"false\"}},\"outputFormat\":\"{AUDIO_FORMAT}\"}}}}}}}}"
    );
    let request_id: String = {
        let bytes = Sha256::digest(format!("{}", current_secs()).as_bytes());
        bytes.iter().take(8).map(|b| format!("{b:02X}")).collect()
    };
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
    let mut finished = false;
    while let Some(item) = stream.next().await {
        let msg = item.map_err(|e| format!("Edge TTS 连接中断: {}", e))?;
        match msg {
            Message::Text(t) => {
                if t.contains("Path:turn.end") {
                    finished = true;
                    break;
                }
                messages.push(WsMessage::Text(t));
            }
            Message::Binary(b) => messages.push(WsMessage::Binary(b)),
            Message::Close(_) => break,
            _ => {}
        }
    }
    if !finished {
        return Err("Edge TTS 响应未完成（将回退系统语音）".into());
    }
    let audio = collect_audio(&messages);
    if audio.is_empty() {
        return Err("Edge TTS 未返回音频（将回退系统语音）".into());
    }
    Ok(audio)
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
}
