use super::long_term::LongTermMemory;
use super::short_term::ShortTermMemory;
use super::types::MemoryKind;

/// Build a memory-context string for injection into LLM prompts.
pub fn build_memory_context(stm: &ShortTermMemory, ltm: &LongTermMemory) -> String {
    let mut parts: Vec<String> = Vec::new();

    // 1. Recent conversations from STM.
    let convos: Vec<&super::types::MemoryEntry> = stm.conversations();
    if !convos.is_empty() {
        parts.push("最近对话：".to_string());
        for e in convos.iter().rev().take(10).rev() {
            parts.push(format!("- {}", e.content));
        }
    }

    // 2. Recent event summary from STM.
    let non_conv: Vec<&super::types::MemoryEntry> = stm
        .all()
        .filter(|e| !matches!(e.kind, MemoryKind::Conversation))
        .collect();
    if !non_conv.is_empty() {
        parts.push("\n最近事件：".to_string());
        for e in non_conv.iter().rev().take(10).rev() {
            parts.push(format!("- [{}] {}", e.kind, e.content));
        }
    }

    // 3. Relevant long-term memories (top 5 by importance).
    let ltm_context = stm.summary(); // use STM summary as query
    let relevant = ltm.retrieve(&ltm_context, 5);
    if !relevant.is_empty() {
        parts.push("\n关于用户的记忆：".to_string());
        for e in &relevant {
            parts.push(format!(
                "- {} (重要性: {:.1})",
                e.content, e.importance
            ));
        }
    }

    if parts.is_empty() {
        return "(暂无记忆上下文)".to_string();
    }

    format!("[记忆上下文]\n{}\n[/记忆上下文]", parts.join("\n"))
}
