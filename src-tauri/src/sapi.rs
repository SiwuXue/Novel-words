//! Windows SAPI5「讲述人自然语音」：PowerShell 调 System.Speech 枚举本机音色与合成。
//!
//! 仅 Windows 生效；COM 对象需 STA 串行访问，用全局互斥锁串行化。
//! 协议对照 ColorTxt src/main/voiceRead/winSapi/voiceReadWinSapi.ts。

use base64::Engine as _;

/// PowerShell 脚本 → `-EncodedCommand` 参数（UTF16LE + base64）。
fn encode_ps_command(script: &str) -> String {
    let utf16: Vec<u8> = script
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    base64::engine::general_purpose::STANDARD.encode(utf16)
}

fn run_powershell(script: &str) -> Result<String, String> {
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-EncodedCommand",
            &encode_ps_command(script),
        ])
        .output()
        .map_err(|e| format!("启动 PowerShell 失败（SAPI5 仅支持 Windows）: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail: String = stderr.chars().take(300).collect();
        return Err(format!("SAPI5 执行失败: {detail}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// 语速倍率 0.5–2.0 → SAPI Rate -10~10。
fn sapi_rate(rate: f64) -> i32 {
    (((rate - 1.0) * 10.0).round() as i32).clamp(-10, 10)
}

/// 枚举本机音色：`[{id,label,locale,gender}]`，Natural 自然语音优先、中文优先。
pub fn sapi_list_voices_blocking() -> Result<Vec<(String, String, String, String)>, String> {
    let script = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$s = New-Object System.Speech.Synthesis.SpeechSynthesizer
$rows = @()
foreach ($v in $s.GetInstalledVoices()) {
    $vi = $v.VoiceInfo
    $rows += @{ id = $vi.Name; label = $vi.Name; locale = $vi.Culture.Name; gender = $v.Gender.ToString().ToLower() }
}
ConvertTo-Json -InputObject @($rows) -Compress -Depth 4
"#;
    let stdout = run_powershell(script)?;
    parse_sapi_voices(&stdout)
}

fn parse_sapi_voices(stdout: &str) -> Result<Vec<(String, String, String, String)>, String> {
    let v: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("SAPI5 音色列表解析失败: {}", e))?;
    let arr: Vec<serde_json::Value> = match v {
        serde_json::Value::Array(a) => a,
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => Vec::new(),
    };
    let mut out: Vec<(String, String, String, String)> = Vec::new();
    for item in arr {
        let id = item["id"].as_str().unwrap_or_default().trim().to_string();
        if id.is_empty() {
            continue;
        }
        let locale = item["locale"].as_str().unwrap_or_default().to_string();
        let gender = item["gender"].as_str().unwrap_or("unknown").to_string();
        let label = item["label"].as_str().unwrap_or(&id).to_string();
        out.push((id, label, locale, gender));
    }
    // Natural 自然语音优先，其次中文
    out.sort_by_key(|(_, _, locale, _)| {
        let is_natural = locale.is_empty();
        let is_zh = locale.starts_with("zh");
        (!is_zh, is_natural)
    });
    Ok(out)
}

/// 合成文本 → WAV 字节（临时文件读写，COM 全局互斥串行）。
pub fn sapi_synthesize_blocking(text: &str, voice: &str, rate: f64) -> Result<Vec<u8>, String> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().map_err(|_| "SAPI5 内部锁错误".to_string())?;

    let dir = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let txt_path = dir.join(format!("nw-sapi-{}.txt", stamp));
    let wav_path = dir.join(format!("nw-sapi-{}.wav", stamp));
    std::fs::write(&txt_path, text).map_err(|e| format!("SAPI5 临时文件写入失败: {}", e))?;

    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Speech
$s = New-Object System.Speech.Synthesis.SpeechSynthesizer
$s.SelectVoice('{}')
$s.Rate = {}
$s.SetOutputToWaveFile('{}')
$s.Speak([System.IO.File]::ReadAllText('{}', [System.Text.Encoding]::UTF8))
$s.Dispose()
"#,
        voice.replace('\'', "''"),
        sapi_rate(rate),
        wav_path.display(),
        txt_path.display(),
    );

    let result = run_powershell(&script);
    let _ = std::fs::remove_file(&txt_path);
    if let Err(e) = result {
        return Err(e);
    }
    let bytes = std::fs::read(&wav_path).map_err(|e| format!("SAPI5 WAV 读取失败: {}", e))?;
    let _ = std::fs::remove_file(&wav_path);
    if bytes.is_empty() {
        return Err("SAPI5 返回空音频".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sapi_rate_maps_multiplier_to_int_range() {
        assert_eq!(sapi_rate(1.0), 0);
        assert_eq!(sapi_rate(2.0), 10);
        assert_eq!(sapi_rate(0.5), -5);
        assert_eq!(sapi_rate(0.0), -10); // clamp
    }

    #[test]
    fn parses_single_object_and_array_voice_lists() {
        let single = r#"{"id":"Microsoft Huihui Desktop","label":"Huihui","locale":"zh-CN","gender":"female"}"#;
        let parsed = parse_sapi_voices(single).unwrap();
        assert_eq!(parsed.len(), 1);

        let arr = r#"[{"id":"A","label":"A","locale":"zh-CN","gender":"female"},{"id":"B","label":"B","locale":"en-US","gender":"male"}]"#;
        let parsed = parse_sapi_voices(arr).unwrap();
        assert_eq!(parsed.len(), 2);
        // 中文优先
        assert_eq!(parsed[0].0, "A");
    }

    #[test]
    fn encodes_utf16le_base64() {
        let enc = encode_ps_command("hi");
        let decoded = base64::engine::general_purpose::STANDARD.decode(enc).unwrap();
        assert_eq!(&decoded, &[0x68, 0x00, 0x69, 0x00]);
    }
}
