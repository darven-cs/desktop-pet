use super::types::{MemoryEntry, MemoryKind};

#[allow(dead_code)]
pub fn should_digest(
    entries_since_last: usize,
    last_digest_at: i64,
    min_entries: usize,
    min_interval_ms: i64,
) -> bool {
    if entries_since_last >= min_entries {
        return true;
    }
    if last_digest_at > 0 {
        let now = chrono::Utc::now().timestamp_millis();
        if now - last_digest_at >= min_interval_ms {
            return true;
        }
    }
    false
}

#[allow(dead_code)]
pub fn build_digestion_prompt(entries: &[&MemoryEntry]) -> String {
    let events: Vec<String> = entries
        .iter()
        .map(|e| format!("- [{}] {} (重要性:{})", e.kind, e.content, e.importance))
        .collect();

    format!(
        r#"你是宠物记忆系统。以下是你最近观察到的事件，请总结：
- 用户做了什么？
- 宠物做了什么？
- 有什么值得记住的重要信息？

事件列表：
{}

返回 JSON：{{"summary":"...", "key_facts":["..."], "importance":0.0-1.0}}"#,
        events.join("\n")
    )
}

#[allow(dead_code)]
pub fn parse_digestion_response(
    body: &str,
    entry_id: &str,
) -> Result<MemoryEntry, String> {
    let root: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("parse error: {}", e))?;

    let content = root["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");

    let trimmed = content.trim();
    let parsed: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| format!("parse error: {}", e))?;

    let summary = parsed["summary"].as_str().unwrap_or("").to_string();
    let key_facts: Vec<String> = parsed["key_facts"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let importance = parsed["importance"]
        .as_f64()
        .unwrap_or(0.5) as f32;

    Ok(MemoryEntry {
        id: entry_id.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        kind: MemoryKind::Reflection,
        content: summary,
        importance: importance.clamp(0.0, 1.0),
        metadata: serde_json::json!({ "key_facts": key_facts }),
    })
}
