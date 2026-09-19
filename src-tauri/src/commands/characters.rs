//! 多角色朗读：novel_characters 表 CRUD + AI 识别角色性别。
//!
//! AI 识别复用通用 OpenAI 兼容客户端（ai_enhancer 的配置：base_url /
//! api_key / model 均在设置页「AI 增强」中配置，DashScope 兼容端点同样可用）。

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use super::ai_enhancer::{chat_completion, load_ai_config};
use crate::db::DbState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelCharacter {
    pub id: i64,
    pub novel_id: i64,
    pub name: String,
    /// male | female | unknown
    pub gender: String,
    /// 音色 id（空 = 跟随章节默认音色）
    pub voice: String,
    /// 同一角色的其它称呼（归因时一并匹配）
    pub aliases: Vec<String>,
    /// ai | manual | heuristic
    pub source: String,
    /// AI 置信度 0-1
    pub confidence: f64,
    /// 判定依据的原文片段
    pub evidence: String,
}

/// 角色表统一 SELECT 列（顺序须与 row_to_character 一致）。
const CHARACTER_COLUMNS: &str = "id, novel_id, name, gender, voice, aliases, source, confidence, evidence";

fn row_to_character(row: &rusqlite::Row) -> rusqlite::Result<NovelCharacter> {
    let aliases_raw: String = row.get(5)?;
    Ok(NovelCharacter {
        id: row.get(0)?,
        novel_id: row.get(1)?,
        name: row.get(2)?,
        gender: row.get(3)?,
        voice: row.get(4)?,
        aliases: serde_json::from_str::<Vec<String>>(&aliases_raw).unwrap_or_default(),
        source: row.get(6)?,
        confidence: row.get(7)?,
        evidence: row.get(8)?,
    })
}

fn valid_gender(gender: &str) -> bool {
    matches!(gender, "male" | "female" | "unknown")
}

#[tauri::command]
pub fn list_novel_characters(
    state: State<DbState>,
    novel_id: i64,
) -> Result<Vec<NovelCharacter>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(&format!(
            "SELECT {CHARACTER_COLUMNS} FROM novel_characters WHERE novel_id = ?1 ORDER BY name"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([novel_id], row_to_character)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// 新增或更新（按 novel_id + name 幂等）。返回最新记录。
#[tauri::command]
pub fn upsert_novel_character(
    state: State<DbState>,
    novel_id: i64,
    name: String,
    gender: String,
    voice: String,
) -> Result<NovelCharacter, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("角色名不能为空".into());
    }
    if !valid_gender(&gender) {
        return Err(format!("无效的性别: {}", gender));
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO novel_characters (novel_id, name, gender, voice)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(novel_id, name)
         DO UPDATE SET gender = excluded.gender,
                       voice = excluded.voice,
                       updated_at = datetime('now', 'localtime')",
        params![novel_id, name, gender, voice.trim()],
    )
    .map_err(|e| format!("保存角色失败: {}", e))?;
    let id = db.last_insert_rowid();
    let character = db
        .query_row(
            &format!("SELECT {CHARACTER_COLUMNS} FROM novel_characters WHERE id = ?1"),
            [id],
            row_to_character,
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or("保存角色后读取失败")?;
    Ok(character)
}

#[tauri::command]
pub fn delete_novel_character(state: State<DbState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM novel_characters WHERE id = ?1", [id])
        .map_err(|e| format!("删除角色失败: {}", e))?;
    Ok(())
}

/// AI 识别出的单个角色（P1：带别名、置信度与判定依据）。
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedCharacter {
    pub name: String,
    /// 同一角色的其它称呼（「铁柱」与「铁柱父亲」应合并为一条）
    pub aliases: Vec<String>,
    /// male | female | unknown
    pub gender: String,
    pub confidence: f64,
    pub evidence: String,
}

/// 明显不是人名的候选：AI 偶尔仍会返回动作词/代词/群体词，落库前再挡一层。
fn is_plausible_name(name: &str) -> bool {
    const REJECT: &[&str] = &[
        "他", "她", "它", "我", "你", "咱", "他们", "她们", "它们", "我们", "你们", "咱们",
        "自己", "众人", "大家", "有人", "那人", "旁人", "所有人", "人们", "路人",
    ];
    if REJECT.contains(&name) {
        return false;
    }
    let ascii = name.chars().all(|c| c.is_ascii());
    let limit = if ascii { 24 } else { 6 };
    if name.chars().count() > limit {
        return false;
    }
    // 短的动作/言语残片（「说」「摇头」「感慨」）
    const BAD_TAIL: &[&str] = &["道", "说", "笑", "问", "喊", "叫", "答", "摇头", "点头", "叹气", "沉吟"];
    if name.chars().count() <= 3 && BAD_TAIL.iter().any(|tail| name.ends_with(tail)) {
        return false;
    }
    true
}

fn normalize_gender(raw: &str) -> &'static str {
    match raw {
        "male" | "男" | "男性" => "male",
        "female" | "女" | "女性" => "female",
        _ => "unknown",
    }
}

/// 从模型返回内容里提取角色 JSON 数组（容忍 markdown 代码块包裹、容忍旧格式）。
/// 旧格式 `[{"name","gender"}]` 仍然接受（aliases 视为空、confidence 视为 0）。
fn parse_detected_characters(content: &str) -> Option<Vec<DetectedCharacter>> {
    let trimmed = content.trim();
    let inner = trimmed
        .strip_prefix("```")
        .map(|rest| {
            let rest = rest.trim_start_matches(['j', 's', 'o', 'n', '\n', '\r']);
            rest.trim()
        })
        .unwrap_or(trimmed)
        .trim_end_matches("```")
        .trim();
    let start = inner.find('[')?;
    let end = inner.rfind(']')?;
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&inner[start..=end]).ok()?;
    let items: Vec<DetectedCharacter> = parsed
        .iter()
        .filter_map(|value| {
            let name = value["name"].as_str()?.trim().to_string();
            if name.is_empty() || !is_plausible_name(&name) {
                return None;
            }
            let aliases = value["aliases"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|a| a.as_str())
                        .map(|a| a.trim().to_string())
                        .filter(|a| !a.is_empty() && a != &name && is_plausible_name(a))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let gender = normalize_gender(value["gender"].as_str().unwrap_or("unknown")).to_string();
            let confidence = value["confidence"].as_f64().unwrap_or(0.0).clamp(0.0, 1.0);
            let evidence: String = value["evidence"]
                .as_str()
                .unwrap_or("")
                .trim()
                .chars()
                .take(60)
                .collect();
            Some(DetectedCharacter {
                name,
                aliases,
                gender,
                confidence,
                evidence,
            })
        })
        .collect();
    Some(items)
}

/// 单次送入模型的字符上限（按行/段边界切，避免把句子截断）。
const AI_DETECT_BATCH_CHARS: usize = 6000;

/// 按行边界把章节切成若干批（单行超长时该批会略超上限，可接受）。
fn split_text_batches(text: &str, max_chars: usize) -> Vec<String> {
    let mut batches: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut count = 0usize;
    for line in text.split_inclusive('\n') {
        let line_chars = line.chars().count();
        if count > 0 && count + line_chars > max_chars {
            batches.push(std::mem::take(&mut current));
            count = 0;
        }
        current.push_str(line);
        count += line_chars;
    }
    if !current.trim().is_empty() {
        batches.push(current);
    }
    batches
}

/// 合并多批识别结果：同名合并别名并集、性别取非 unknown、置信度取最大、证据取首个。
fn merge_detected(target: &mut Vec<DetectedCharacter>, incoming: Vec<DetectedCharacter>) {
    for item in incoming {
        match target.iter_mut().find(|existing| existing.name == item.name) {
            Some(existing) => {
                for alias in item.aliases {
                    if !existing.aliases.contains(&alias) {
                        existing.aliases.push(alias);
                    }
                }
                if existing.gender == "unknown" && item.gender != "unknown" {
                    existing.gender = item.gender;
                }
                if item.confidence > existing.confidence {
                    existing.confidence = item.confidence;
                }
                if existing.evidence.is_empty() {
                    existing.evidence = item.evidence;
                }
            }
            None => target.push(item),
        }
    }
}

const AI_CHARACTER_PROMPT: &str = "你是小说人物分析助手，结果用于给对白分配朗读音色。\
    从用户提供的小说文本中**只抽取真正的人物 / 称谓**：\
    1) 只输出人名、称谓（如「王小明」「李老师」「铁柱父亲」「船长」）；\
    2) 不要输出动作词、神态短语、代词、群体词——「摇头」「笑着说」「感慨」「他」「她」「众人」「大家」都不是角色；\
    3) aliases 填同一角色在文中的其它称呼（如「铁柱」「铁柱父亲」「老铁」实为一人时，放进同一个条目的 aliases）；\
    4) gender 取 male / female / unknown；\
    5) evidence 填判定依据的原文片段（不超过 30 字），confidence 为 0-1 的置信度。\
    最多 20 个角色，按对白重要程度排序。只输出 JSON 数组，不要任何解释，格式：\
    [{\"name\":\"角色名\",\"aliases\":[\"别名\"],\"gender\":\"male|female|unknown\",\"confidence\":0.9,\"evidence\":\"原文片段\"}]";

/// 调用模型抽取一批文本里的角色。
async fn extract_characters(
    config: &super::ai_enhancer::AiConfig,
    text: &str,
) -> Result<Vec<DetectedCharacter>, String> {
    let content = chat_completion(config, AI_CHARACTER_PROMPT, text, true).await?;
    parse_detected_characters(&content).ok_or_else(|| "AI 返回内容无法解析为角色列表".to_string())
}

/// AI 识别章节文本中的角色：整章分批抽取（含别名/证据/置信度）后合并落库。
/// 已有记录保留其音色；性别仅 unknown 时更新；别名与置信度取并集/较大值。
#[tauri::command]
pub async fn ai_detect_characters(
    state: State<'_, DbState>,
    novel_id: i64,
    text: String,
) -> Result<Vec<NovelCharacter>, String> {
    let config = load_ai_config(&state)?;
    if !config.enabled {
        return Err("AI 增强未启用（请在设置页配置并启用）".into());
    }
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("章节内容为空".into());
    }

    let batches = split_text_batches(&text, AI_DETECT_BATCH_CHARS);
    let mut detected: Vec<DetectedCharacter> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for batch in batches {
        match extract_characters(&config, &batch).await {
            Ok(items) => merge_detected(&mut detected, items),
            Err(e) => errors.push(e),
        }
    }
    if detected.is_empty() {
        return Err(errors
            .first()
            .cloned()
            .unwrap_or_else(|| "AI 未从文本中识别到角色".to_string()));
    }

    let detected = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        merge_characters(&db, novel_id, detected)?
    };
    Ok(detected)
}

/// 合并识别结果：新角色插入，已有角色保留音色、补性别/别名/证据。
/// 返回合并后的完整角色列表。
fn merge_characters(
    db: &Connection,
    novel_id: i64,
    items: Vec<DetectedCharacter>,
) -> Result<Vec<NovelCharacter>, String> {
    for item in &items {
        let name = item.name.trim();
        if name.is_empty() {
            continue;
        }
        let aliases_json = serde_json::to_string(&item.aliases).unwrap_or_else(|_| "[]".to_string());
        db.execute(
            "INSERT INTO novel_characters (novel_id, name, gender, voice, aliases, source, confidence, evidence)
             VALUES (?1, ?2, ?3, '', ?4, 'ai', ?5, ?6)
             ON CONFLICT(novel_id, name)
             DO UPDATE SET gender = CASE
                 WHEN novel_characters.gender = 'unknown' AND excluded.gender != 'unknown'
                     THEN excluded.gender
                 ELSE novel_characters.gender END,
                 aliases = CASE
                 WHEN novel_characters.aliases = '[]' THEN excluded.aliases
                     ELSE novel_characters.aliases END,
                 confidence = MAX(novel_characters.confidence, excluded.confidence),
                 evidence = CASE
                 WHEN novel_characters.evidence = '' THEN excluded.evidence
                     ELSE novel_characters.evidence END,
                 source = CASE
                 WHEN novel_characters.source = 'manual' THEN 'manual'
                     ELSE 'ai' END,
                 updated_at = datetime('now', 'localtime')",
            params![
                novel_id,
                name,
                item.gender,
                aliases_json,
                item.confidence,
                item.evidence
            ],
        )
        .map_err(|e| format!("合并角色失败: {}", e))?;
    }
    let mut stmt = db
        .prepare(&format!(
            "SELECT {CHARACTER_COLUMNS} FROM novel_characters WHERE novel_id = ?1 ORDER BY name"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([novel_id], row_to_character)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_detected_characters_handles_plain_fenced_and_legacy_json() {
        // 新格式：别名 / 置信度 / 证据
        let items = parse_detected_characters(
            r#"[{"name":"铁柱","aliases":["铁柱父亲"],"gender":"male","confidence":0.9,"evidence":"铁柱父亲摇头道：\"…\""}]"#,
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "铁柱");
        assert_eq!(items[0].aliases, vec!["铁柱父亲".to_string()]);
        assert_eq!(items[0].gender, "male");
        assert!((items[0].confidence - 0.9).abs() < 1e-6);

        // 旧格式（无 aliases/confidence）仍兼容
        let legacy = parse_detected_characters(
            r#"[{"name":"王小明","gender":"male"},{"name":"李老师","gender":"女"}]"#,
        )
        .unwrap();
        assert_eq!(legacy.len(), 2);
        assert_eq!(legacy[1].gender, "female", "中文性别归一化");
        assert!(legacy[0].aliases.is_empty());

        // markdown 围栏
        let fenced = "```json\n[{\"name\":\"Tom\",\"gender\":\"男\"}]\n```";
        assert_eq!(parse_detected_characters(fenced).unwrap()[0].name, "Tom");

        // 非人名条目被丢弃（动作词 / 代词 / 群体词 / 超长短语）
        let noisy = r#"[{"name":"摇头"},{"name":"他"},{"name":"众人"},{"name":"船人不在意在铁柱耳边"},{"name":"张三","gender":"未知"}]"#;
        let cleaned = parse_detected_characters(noisy).unwrap();
        assert_eq!(cleaned.len(), 1);
        assert_eq!(cleaned[0].name, "张三");

        assert!(parse_detected_characters("没有 JSON").is_none());
    }

    #[test]
    fn split_text_batches_cuts_on_line_boundaries() {
        // 每行 10 字符（含换行），上限 25 → 每批 2 行
        let text = "012345678\n012345678\n012345678\n012345678\n";
        let batches = split_text_batches(text, 25);
        assert_eq!(batches.len(), 2);
        assert!(batches.iter().all(|b| b.chars().count() <= 25));
        assert_eq!(batches.concat(), text, "切分不丢字符");

        // 空文本
        assert!(split_text_batches("   \n ", 10).is_empty());
    }

    #[test]
    fn merge_detected_unions_aliases_and_prefers_known_gender() {
        let mut target = vec![DetectedCharacter {
            name: "铁柱".into(),
            aliases: vec!["铁柱父亲".into()],
            gender: "unknown".into(),
            confidence: 0.4,
            evidence: String::new(),
        }];
        merge_detected(
            &mut target,
            vec![
                DetectedCharacter {
                    name: "铁柱".into(),
                    aliases: vec!["老铁".into(), "铁柱父亲".into()],
                    gender: "male".into(),
                    confidence: 0.8,
                    evidence: "铁柱道：…".into(),
                },
                DetectedCharacter {
                    name: "阿黄".into(),
                    aliases: vec![],
                    gender: "unknown".into(),
                    confidence: 0.3,
                    evidence: String::new(),
                },
            ],
        );
        assert_eq!(target.len(), 2);
        let tie = target.iter().find(|c| c.name == "铁柱").unwrap();
        assert_eq!(tie.aliases, vec!["铁柱父亲".to_string(), "老铁".to_string()]);
        assert_eq!(tie.gender, "male");
        assert!((tie.confidence - 0.8).abs() < 1e-6);
        assert_eq!(tie.evidence, "铁柱道：…");
    }

    #[test]
    fn merge_characters_inserts_and_preserves_voice() {
        let dir = std::env::temp_dir().join(format!(
            "nw-chars-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let state = crate::db::init_db(&dir).unwrap();
        let db = state.db.lock().unwrap();
        db.execute("INSERT INTO novel (title) VALUES ('T')", []).unwrap();

        // 预置一个已有音色的角色
        db.execute(
            "INSERT INTO novel_characters (novel_id, name, gender, voice) VALUES (1, '王小明', 'unknown', 'zh-CN-YunjianNeural')",
            [],
        )
        .unwrap();

        let items = vec![
            DetectedCharacter {
                name: "王小明".into(),
                aliases: vec!["小明".into()],
                gender: "male".into(),
                confidence: 0.9,
                evidence: "王小明说：…".into(),
            },
            DetectedCharacter {
                name: "新角色".into(),
                aliases: vec![],
                gender: "female".into(),
                confidence: 0.5,
                evidence: String::new(),
            },
        ];
        let all = merge_characters(&db, 1, items).unwrap();
        let ming = all.iter().find(|c| c.name == "王小明").unwrap();
        assert_eq!(ming.gender, "male", "unknown 性别应被 AI 结果更新");
        assert_eq!(ming.voice, "zh-CN-YunjianNeural", "已有音色必须保留");
        assert_eq!(ming.aliases, vec!["小明".to_string()], "别名应落库");
        assert_eq!(ming.source, "manual", "手工创建过的角色不被标记为 ai");
        assert_eq!(ming.evidence, "王小明说：…");
        let new_char = all.iter().find(|c| c.name == "新角色").unwrap();
        assert_eq!(new_char.gender, "female");
        assert_eq!(new_char.source, "ai", "AI 新插入的角色标记为 ai");

        // 再次合并：gender 已是 male，不被 unknown 覆盖；已有别名不被空别名覆盖
        merge_characters(
            &db,
            1,
            vec![DetectedCharacter {
                name: "王小明".into(),
                aliases: vec![],
                gender: "unknown".into(),
                confidence: 0.1,
                evidence: String::new(),
            }],
        )
        .unwrap();
        let (gender, aliases, confidence): (String, String, f64) = db
            .query_row(
                "SELECT gender, aliases, confidence FROM novel_characters WHERE name='王小明'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(gender, "male");
        assert_eq!(aliases, "[\"小明\"]", "已有别名保留");
        assert!((confidence - 0.9).abs() < 1e-6, "置信度取较大值");
    }
}
