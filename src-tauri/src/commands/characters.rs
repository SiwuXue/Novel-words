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
}

fn row_to_character(row: &rusqlite::Row) -> rusqlite::Result<NovelCharacter> {
    Ok(NovelCharacter {
        id: row.get(0)?,
        novel_id: row.get(1)?,
        name: row.get(2)?,
        gender: row.get(3)?,
        voice: row.get(4)?,
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
        .prepare(
            "SELECT id, novel_id, name, gender, voice
             FROM novel_characters WHERE novel_id = ?1
             ORDER BY name",
        )
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
            "SELECT id, novel_id, name, gender, voice FROM novel_characters WHERE id = ?1",
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

/// 从模型返回内容里提取 JSON 数组（容忍 markdown 代码块包裹）。
fn extract_json_array(content: &str) -> Option<Vec<(String, String)>> {
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
    let slice = &inner[start..=end];
    let parsed: Vec<serde_json::Value> = serde_json::from_str(slice).ok()?;
    let items = parsed
        .iter()
        .filter_map(|v| {
            let name = v["name"].as_str()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let gender = match v["gender"].as_str().unwrap_or("unknown") {
                "male" | "男" => "male",
                "female" | "女" => "female",
                _ => "unknown",
            };
            Some((name, gender.to_string()))
        })
        .collect();
    Some(items)
}

/// AI 识别章节文本中的角色与性别：调用通用 AI 配置，结果合并进
/// novel_characters（已有记录的 voice 保留，仅更新性别 / 补充新角色）。
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
    let sample: String = text.chars().take(6000).collect();
    let system = "你是小说文本分析助手。从用户提供的小说文本中识别出有对白或明确活动的角色（说话人），\
        为每个角色判断性别。只输出 JSON 数组，不要任何解释，格式：\
        [{\"name\":\"角色名\",\"gender\":\"male|female|unknown\"}]。\
        最多列出 20 个角色，按重要程度排序。";
    let content = chat_completion(&config, system, &sample).await?;
    let items = extract_json_array(&content).ok_or("AI 返回内容无法解析为角色列表")?;
    if items.is_empty() {
        return Err("AI 未从文本中识别到角色".into());
    }

    let detected = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        merge_characters(&db, novel_id, items)?
    };
    Ok(detected)
}

/// 合并识别结果：新角色插入，已有角色仅当 gender 为 unknown 时更新。
/// 返回合并后的完整角色列表。
fn merge_characters(
    db: &Connection,
    novel_id: i64,
    items: Vec<(String, String)>,
) -> Result<Vec<NovelCharacter>, String> {
    for (name, gender) in &items {
        db.execute(
            "INSERT INTO novel_characters (novel_id, name, gender, voice)
             VALUES (?1, ?2, ?3, '')
             ON CONFLICT(novel_id, name)
             DO UPDATE SET gender = CASE
                 WHEN novel_characters.gender = 'unknown' AND excluded.gender != 'unknown'
                     THEN excluded.gender
                 ELSE novel_characters.gender END,
                 updated_at = datetime('now', 'localtime')",
            params![novel_id, name.trim(), gender],
        )
        .map_err(|e| format!("合并角色失败: {}", e))?;
    }
    let mut stmt = db
        .prepare(
            "SELECT id, novel_id, name, gender, voice
             FROM novel_characters WHERE novel_id = ?1 ORDER BY name",
        )
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
    fn extract_json_array_handles_plain_and_fenced_json() {
        let items = extract_json_array(r#"[{"name":"王小明","gender":"male"},{"name":"李老师","gender":"female"}]"#).unwrap();
        assert_eq!(items, vec![("王小明".into(), "male".into()), ("李老师".into(), "female".into())]);

        let fenced = "```json\n[{\"name\":\"Tom\",\"gender\":\"男\"}]\n```";
        let items = extract_json_array(fenced).unwrap();
        assert_eq!(items, vec![("Tom".into(), "male".into())]);

        // 中文性别归一化 + 无效条目过滤
        let mixed = "说明文字 [ {\"name\":\"张三\",\"gender\":\"未知\"}, {\"name\":\"\",\"gender\":\"male\"} ] 尾部";
        let items = extract_json_array(mixed).unwrap();
        assert_eq!(items, vec![("张三".into(), "unknown".into())]);

        assert!(extract_json_array("没有 JSON").is_none());
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
            ("王小明".to_string(), "male".to_string()),
            ("新角色".to_string(), "female".to_string()),
        ];
        let all = merge_characters(&db, 1, items).unwrap();
        let ming = all.iter().find(|c| c.name == "王小明").unwrap();
        assert_eq!(ming.gender, "male", "unknown 性别应被 AI 结果更新");
        assert_eq!(ming.voice, "zh-CN-YunjianNeural", "已有音色必须保留");
        let new_char = all.iter().find(|c| c.name == "新角色").unwrap();
        assert_eq!(new_char.gender, "female");

        // 再次合并：gender 已是 male，不被 unknown 覆盖
        merge_characters(&db, 1, vec![("王小明".to_string(), "unknown".to_string())]).unwrap();
        let gender: String = db
            .query_row("SELECT gender FROM novel_characters WHERE name='王小明'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(gender, "male");
    }
}
