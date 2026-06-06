use std::path::PathBuf;

use super::types::MemoryEntry;

pub struct LongTermMemory {
    file_path: PathBuf,
    entries: Vec<MemoryEntry>,
}

impl LongTermMemory {
    /// Load from disk, or create empty if file doesn't exist.
    pub fn load(file_path: PathBuf) -> Self {
        let entries = match std::fs::read_to_string(&file_path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        Self { file_path, entries }
    }

    pub fn save(&self) {
        if let Some(parent) = self.file_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.entries) {
            let _ = std::fs::write(&self.file_path, json);
        }
    }

    pub fn insert(&mut self, entry: MemoryEntry) {
        self.entries.push(entry);
        self.save();
    }

    /// Simple keyword-based retrieval, sorted by importance desc.
    pub fn retrieve(&self, query: &str, limit: usize) -> Vec<&MemoryEntry> {
        let now = chrono::Utc::now().timestamp_millis();
        let week_ms: i64 = 7 * 24 * 3600 * 1000;
        let mut scored: Vec<(&MemoryEntry, f32)> = self
            .entries
            .iter()
            .filter(|e| e.importance > 0.0)
            .map(|e| {
                let mut score = e.importance;
                // Keyword match bonus.
                let query_lower = query.to_lowercase();
                if query_lower.len() > 1
                    && e.content.to_lowercase().contains(&query_lower)
                {
                    score += 0.3;
                }
                // Recency bonus (within 7 days).
                let age_ms = now - e.timestamp;
                if age_ms < week_ms {
                    let recency = 1.0 - (age_ms as f32 / week_ms as f32);
                    score += recency * 0.2;
                }
                (e, score)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(limit)
            .map(|(e, _)| e)
            .collect()
    }

    pub fn prune(&mut self) {
        let now = chrono::Utc::now().timestamp_millis();
        let month_ms: i64 = 30 * 24 * 3600 * 1000;
        let cutoff = now - month_ms;
        self.entries.retain(|e| {
            !(e.importance < 0.2 && e.timestamp < cutoff)
        });
        self.save();
    }

    pub fn all(&self) -> &[MemoryEntry] {
        &self.entries
    }
}
