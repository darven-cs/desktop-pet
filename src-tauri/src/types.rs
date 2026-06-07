// Pet data structures, shared between Rust and TS via serde (R7).
// TS counterpart: src/types/pet.ts (manually mirrored, see AC-R7).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum LoopMode {
    Infinite,
    Once,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AnimationEntry {
    pub id: String,
    pub sheet_path: String,
    pub frame_count: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    pub fps: u32,
    pub loop_mode: LoopMode,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DecisionContext {
    pub current_state: AnimationState,
    pub last_interaction_at: u64,
    pub ticker_interval_ms: u32,
    // 02: optional fields for LLM context (spec 02 §3.2)
    pub time_of_day: Option<String>,
    pub recent_history: Option<Vec<String>>,
    // 02: runtime-overridable pet settings (spec 02 §3.2)
    pub llm_enabled: Option<bool>,
    pub pet_personality: Option<String>,
    pub pet_name: Option<String>,
    // 02: runtime-overridable API config (overrides .env)
    pub llm_api_endpoint: Option<String>,
    pub llm_api_key: Option<String>,
    pub llm_model: Option<String>,
    // 04: frontend-provided compacted events summary (overrides format_events_summary)
    pub events_summary: Option<String>,
    // memory context injected by decider (not sent from frontend)
    #[serde(skip_deserializing, default)]
    pub memory_context: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Decision {
    Stay,
    Switch {
        to: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        reason: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Speak {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        animation: Option<String>,
    },
    EnterIdle,
    ExitIdle,
    Wait {
        duration_ms: u32,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        reason: Option<String>,
    },
    SetReminder {
        message: String,
        delay_seconds: u32,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AnimationState {
    pub phase: Phase,
    pub current: String,
    pub iteration: u32,
    pub transition: Option<Transition>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Playing,
    Idle,
    Transitioning,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub progress: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppErrorCode {
    EAnimNotFound,
    EFramesMissing,
    EInvalidContext,
    EInternal,
}

impl std::fmt::Display for AppErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::EAnimNotFound => "E_ANIM_NOT_FOUND",
            Self::EFramesMissing => "E_FRAMES_MISSING",
            Self::EInvalidContext => "E_INVALID_CONTEXT",
            Self::EInternal => "E_INTERNAL",
        };
        f.write_str(s)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppError {
    pub code: AppErrorCode,
    pub message: String,
}

impl AppError {
    #[allow(dead_code)]
    pub fn anim_not_found(msg: impl Into<String>) -> Self {
        Self { code: AppErrorCode::EAnimNotFound, message: msg.into() }
    }
    pub fn frames_missing(msg: impl Into<String>) -> Self {
        Self { code: AppErrorCode::EFramesMissing, message: msg.into() }
    }
    pub fn invalid_context(msg: impl Into<String>) -> Self {
        Self { code: AppErrorCode::EInvalidContext, message: msg.into() }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self { code: AppErrorCode::EInternal, message: msg.into() }
    }
}

// --- 02 LLM types (spec 02 §4.1) ---

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LlmInfo {
    pub endpoint: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfigUpdate {
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub enabled: Option<bool>,
}
