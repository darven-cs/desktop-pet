use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PetEvent {
    TimerTick { timestamp: u64 },
    UserInteraction { interaction: String, timestamp: u64 },
    AnimationCompleted { animation_id: String, timestamp: u64 },
    WindowFocusChanged { focused: bool, timestamp: u64 },
    ReminderTriggered { message: String, timestamp: u64 },
}

/// Format a slice of PetEvents into a natural-language summary suitable for the LLM prompt.
pub fn format_events_summary(events: &[PetEvent]) -> String {
    if events.is_empty() {
        return String::new();
    }

    let mut lines = vec!["最近事件：".to_string()];
    for ev in events {
        let ts = format_timestamp(ev.timestamp());
        lines.push(format!("- [{}] {}", ts, ev.label()));
    }
    lines.join("\n")
}

impl PetEvent {
    fn timestamp(&self) -> u64 {
        match self {
            PetEvent::TimerTick { timestamp, .. } => *timestamp,
            PetEvent::UserInteraction { timestamp, .. } => *timestamp,
            PetEvent::AnimationCompleted { timestamp, .. } => *timestamp,
            PetEvent::WindowFocusChanged { timestamp, .. } => *timestamp,
            PetEvent::ReminderTriggered { timestamp, .. } => *timestamp,
        }
    }

    fn label(&self) -> String {
        match self {
            PetEvent::TimerTick { .. } => "timer_tick".to_string(),
            PetEvent::UserInteraction { interaction, .. } => format!("user_interaction({})", interaction),
            PetEvent::AnimationCompleted { animation_id, .. } => {
                format!("animation_completed({})", animation_id)
            }
            PetEvent::WindowFocusChanged { focused, .. } => {
                format!("window_focus_changed({})", if *focused { "focused" } else { "unfocused" })
            }
            PetEvent::ReminderTriggered { message, .. } => {
                format!("reminder_triggered({})", message)
            }
        }
    }
}

/// Convert a unix-millis timestamp to a readable HH:MM:SS string.
fn format_timestamp(millis: u64) -> String {
    let secs = (millis / 1000) as u32;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}
