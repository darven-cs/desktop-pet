use std::time::{SystemTime, UNIX_EPOCH};

pub struct AgentBudget {
    pub max_steps: u32,
    pub max_calls_per_minute: u32,
    pub min_call_interval_ms: u64,
    pub calls_timestamps: Vec<u64>,
}

impl AgentBudget {
    pub fn new() -> Self {
        Self {
            max_steps: 5,
            max_calls_per_minute: 3,
            min_call_interval_ms: 15000,
            calls_timestamps: Vec::new(),
        }
    }

    /// Check whether a new call is allowed (budget not exhausted + interval satisfied).
    pub fn check(&self) -> bool {
        let now = now_millis();

        // Count calls within the last 60 s.
        let recent = self
            .calls_timestamps
            .iter()
            .filter(|&&ts| now.saturating_sub(ts) < 60_000)
            .count() as u32;

        if recent >= self.max_calls_per_minute {
            return false;
        }

        // Enforce minimum interval since last call.
        if let Some(&last) = self.calls_timestamps.last() {
            if now.saturating_sub(last) < self.min_call_interval_ms {
                return false;
            }
        }

        true
    }

    /// Consume one call quota and record the current timestamp.
    pub fn consume(&mut self) {
        let now = now_millis();
        self.calls_timestamps.push(now);
        self.prune();
    }

    /// Remove timestamps older than 60 s.
    fn prune(&mut self) {
        let now = now_millis();
        self.calls_timestamps
            .retain(|&ts| now.saturating_sub(ts) < 60_000);
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
