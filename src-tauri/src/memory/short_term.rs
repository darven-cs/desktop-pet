use std::collections::VecDeque;

use super::types::{MemoryEntry, MemoryKind};

pub struct ShortTermMemory {
    entries: VecDeque<MemoryEntry>,
    max_entries: usize,
    max_age_ms: i64,
}

impl ShortTermMemory {
    pub fn new(max_entries: usize, max_age_ms: i64) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
            max_age_ms,
        }
    }

    /// Push an entry; auto-evict overflow and expired entries.
    pub fn push(&mut self, entry: MemoryEntry) {
        self.entries.push_back(entry);
        self.evict();
    }

    #[allow(dead_code)]
    pub fn recent(&self, n: usize) -> Vec<&MemoryEntry> {
        let len = self.entries.len();
        let start = len.saturating_sub(n);
        self.entries.iter().skip(start).collect()
    }

    /// Only conversation-type entries.
    pub fn conversations(&self) -> Vec<&MemoryEntry> {
        self.entries
            .iter()
            .filter(|e| matches!(e.kind, MemoryKind::Conversation))
            .collect()
    }

    #[allow(dead_code)]
    pub fn since(&self, ts: i64) -> Vec<&MemoryEntry> {
        self.entries.iter().filter(|e| e.timestamp >= ts).collect()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// All entries as a slice for iteration.
    pub fn all(&self) -> std::collections::vec_deque::Iter<'_, MemoryEntry> {
        self.entries.iter()
    }

    /// Brief text summary for prompt injection.
    pub fn summary(&self) -> String {
        if self.entries.is_empty() {
            return "(空)".to_string();
        }
        let mut lines: Vec<String> = Vec::new();
        for e in &self.entries {
            lines.push(format!("- [{}] {}", e.kind, e.content));
        }
        lines.join("\n")
    }

    fn evict(&mut self) {
        let now = chrono::Utc::now().timestamp_millis();
        let cutoff = now - self.max_age_ms;
        // Remove expired entries from front (oldest first).
        while let Some(front) = self.entries.front() {
            if front.timestamp < cutoff {
                self.entries.pop_front();
            } else {
                break;
            }
        }
        // Remove oldest if over capacity.
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: &str, ts: i64, content: &str) -> MemoryEntry {
        MemoryEntry {
            id: id.to_string(),
            timestamp: ts,
            kind: MemoryKind::Observation,
            content: content.to_string(),
            importance: 0.5,
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_push_and_recent() {
        let mut stm = ShortTermMemory::new(100, 7_200_000);
        stm.push(make_entry("1", 1000, "a"));
        stm.push(make_entry("2", 2000, "b"));
        stm.push(make_entry("3", 3000, "c"));
        let recent = stm.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].content, "b");
        assert_eq!(recent[1].content, "c");
    }

    #[test]
    fn test_capacity_eviction() {
        let mut stm = ShortTermMemory::new(3, 7_200_000);
        for i in 0..5 {
            stm.push(make_entry(&format!("{}", i), i as i64 * 1000, "x"));
        }
        assert_eq!(stm.len(), 3);
        assert_eq!(stm.entries[0].id, "2");
    }
}
