pub mod digest;
pub mod long_term;
pub mod retrieval;
pub mod short_term;
pub mod types;

use long_term::LongTermMemory;
use short_term::ShortTermMemory;
use types::{MemoryEntry, MemoryKind};

const DEFAULT_MAX_ENTRIES: usize = 100;
const DEFAULT_MAX_AGE_MS: i64 = 7_200_000; // 2 hours
#[allow(dead_code)]
const DIGEST_MIN_ENTRIES: usize = 20;
#[allow(dead_code)]
const DIGEST_MIN_INTERVAL_MS: i64 = 1_800_000; // 30 minutes

pub struct MemoryManager {
    short_term: ShortTermMemory,
    long_term: LongTermMemory,
    #[allow(dead_code)]
    last_digest_at: i64,
    #[allow(dead_code)]
    entries_since_digest: usize,
}

impl MemoryManager {
    pub fn new(file_path: std::path::PathBuf) -> Self {
        Self {
            short_term: ShortTermMemory::new(DEFAULT_MAX_ENTRIES, DEFAULT_MAX_AGE_MS),
            long_term: LongTermMemory::load(file_path),
            last_digest_at: 0,
            entries_since_digest: 0,
        }
    }

    /// Record an event into STM. Auto-promotes important entries to LTM.
    pub fn record(&mut self, kind: MemoryKind, content: String, importance: f32) {
        let entry = MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            kind,
            content,
            importance,
            metadata: serde_json::Value::Null,
        };
        self.short_term.push(entry.clone());
        self.entries_since_digest += 1;

        // Auto-promote conversations and important decisions to LTM.
        match &entry.kind {
            MemoryKind::Conversation if importance >= 0.3 => {
                self.long_term.insert(entry);
            }
            MemoryKind::Decision if importance >= 0.5 => {
                self.long_term.insert(entry);
            }
            _ => {}
        }
    }

    #[allow(dead_code)] // kept for future digestion pipeline
    pub fn record_entry(&mut self, entry: MemoryEntry) {
        self.short_term.push(entry);
        self.entries_since_digest += 1;
    }

    /// Build memory context for prompt injection.
    pub fn build_context(&self) -> String {
        retrieval::build_memory_context(&self.short_term, &self.long_term)
    }

    #[allow(dead_code)] // kept for future digestion pipeline
    pub fn should_digest(&self) -> bool {
        digest::should_digest(
            self.entries_since_digest,
            self.last_digest_at,
            DIGEST_MIN_ENTRIES,
            DIGEST_MIN_INTERVAL_MS,
        )
    }

    #[allow(dead_code)] // kept for future digestion pipeline
    pub fn entries_since_last_digest(&self) -> Vec<&MemoryEntry> {
        self.short_term.since(self.last_digest_at)
    }

    #[allow(dead_code)] // kept for future digestion pipeline
    pub fn commit_digest(&mut self, entry: MemoryEntry) {
        self.long_term.insert(entry);
        self.last_digest_at = chrono::Utc::now().timestamp_millis();
        self.entries_since_digest = 0;
    }

    /// Get all memories (STM + LTM) for display, filtered by optional kind.
    pub fn get_memories(&self, kind: Option<&str>, limit: usize) -> Vec<MemoryEntry> {
        let mut all: Vec<MemoryEntry> = self
            .short_term
            .all()
            .chain(self.long_term.all().iter())
            .filter(|e| {
                if let Some(k) = kind {
                    e.kind.to_string() == k
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        all.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all.truncate(limit);
        all
    }

    pub fn prune(&mut self) {
        self.long_term.prune();
    }
}
