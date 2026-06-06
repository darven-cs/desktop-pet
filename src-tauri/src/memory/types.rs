use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum MemoryKind {
    Observation,
    Interaction,
    Decision,
    Conversation,
    Reflection,
}

impl std::fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Observation => "observation",
            Self::Interaction => "interaction",
            Self::Decision => "decision",
            Self::Conversation => "conversation",
            Self::Reflection => "reflection",
        };
        f.write_str(s)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEntry {
    pub id: String,
    pub timestamp: i64,
    pub kind: MemoryKind,
    pub content: String,
    pub importance: f32,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub animation_triggered: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChatResponse {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub animation: Option<String>,
}
