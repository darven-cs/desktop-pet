mod chat;
mod decider;
mod llm;
mod memory;
mod registry;
mod tools;
mod types;

use std::sync::Mutex;

use memory::types::{ChatMessage, MemoryEntry};
use types::{AppError, Decision, DecisionContext};

/// Strip "用户: " or "宠物: " prefix from conversation content safely.
fn strip_conversation_prefix(content: &str) -> String {
    if let Some(rest) = content.strip_prefix("用户: ") {
        return rest.to_string();
    }
    if let Some(rest) = content.strip_prefix("宠物: ") {
        return rest.to_string();
    }
    content.to_string()
}

/// Resolve the memory file path (~/.local/share/desktop-pet/memory.json on Linux).
fn memory_file_path() -> std::path::PathBuf {
    let base = if cfg!(target_os = "linux") {
        std::env::var("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                std::path::PathBuf::from(home).join(".local/share")
            })
    } else if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join("Library/Application Support")
    } else {
        std::env::var("APPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
    };
    base.join("desktop-pet/memory.json")
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn list_animations() -> Result<Vec<types::AnimationEntry>, AppError> {
    eprintln!("[PetCmd] list_animations called with: ()");
    let sprites_dir = registry::locate_sprites_dir();
    eprintln!("[PetCmd] list_animations using sprites dir: {}", sprites_dir.display());
    match registry::list_animations(&sprites_dir) {
        Ok(v) => {
            eprintln!("[PetCmd] list_animations returned {} entries", v.len());
            Ok(v)
        }
        Err(e) => {
            eprintln!("[PetError] list_animations → {}: {}", e.code, e.message);
            Err(e)
        }
    }
}

#[tauri::command]
fn get_llm_info() -> Result<types::LlmInfo, AppError> {
    let config = llm::load_static_config();
    Ok(types::LlmInfo {
        endpoint: config.endpoint,
        model: config.model,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        enabled: config.enabled,
    })
}

#[tauri::command]
fn update_llm_config(_update: types::LlmConfigUpdate) -> Result<(), AppError> {
    eprintln!(
        "[PetCmd] update_llm_config called with: {}",
        serde_json::to_string(&_update).unwrap_or_default()
    );
    Ok(())
}

#[tauri::command]
async fn decide_next_state(
    state: tauri::State<'_, Mutex<memory::MemoryManager>>,
    context: DecisionContext,
) -> Result<Decision, AppError> {
    let log_ctx = {
        let mut c = context.clone();
        if c.llm_api_key.is_some() {
            c.llm_api_key = Some("***".into());
        }
        c
    };
    eprintln!(
        "[PetCmd] decide_next_state called with: {}",
        serde_json::to_string(&log_ctx).unwrap_or_default()
    );
    match decider::decide_next_state(&state, context).await {
        Ok(d) => Ok(d),
        Err(e) => {
            eprintln!("[PetError] decide_next_state → {}: {}", e.code, e.message);
            Err(e)
        }
    }
}

#[tauri::command]
async fn send_message(
    state: tauri::State<'_, Mutex<memory::MemoryManager>>,
    context: DecisionContext,
    text: String,
    context_text: Option<String>,
) -> Result<memory::types::ChatResponse, AppError> {
    let log_text = if text.len() > 100 { &text[..100] } else { &text };
    eprintln!(
        "[PetCmd] send_message called with: text='{}...', context_text='{}'",
        log_text,
        context_text.as_deref().unwrap_or("")
    );

    let static_cfg = llm::load_static_config();

    // Record user message.
    {
        let mut mgr = state.lock().map_err(|e| AppError::internal(e.to_string()))?;
        mgr.record(
            memory::types::MemoryKind::Conversation,
            format!("用户: {}", text),
            0.5,
        );
    }

    // Build memory context and chat history.
    let (memory_context, history) = {
        let mgr = state.lock().map_err(|e| AppError::internal(e.to_string()))?;
        let ctx = mgr.build_context();
        // Build chat history from STM conversation entries.
        let convos = mgr.get_memories(Some("conversation"), 20);
        let history: Vec<ChatMessage> = convos
            .into_iter()
            .map(|e| {
                let role = if e.content.starts_with("用户:") {
                    "user"
                } else {
                    "assistant"
                };
                let content = strip_conversation_prefix(&e.content);
                ChatMessage {
                    role: role.to_string(),
                    content,
                    timestamp: e.timestamp,
                    animation_triggered: None,
                }
            })
            .rev()
            .collect();
        (ctx, history)
    };

    let tools = tools::ToolRegistry::new();
    match chat::send_chat_message(
        &static_cfg,
        &context,
        &text,
        context_text.as_deref(),
        &history,
        &memory_context,
        &tools,
    )
    .await
    {
        Ok(response) => {
            // Record pet response.
            if let Ok(mut mgr) = state.lock() {
                mgr.record(
                    memory::types::MemoryKind::Conversation,
                    format!("宠物: {}", response.message),
                    0.5,
                );
            }
            Ok(response)
        }
        Err(e) => {
            eprintln!("[PetChat] error: {}: {}", e.code, e.message);
            Err(e)
        }
    }
}

#[tauri::command]
fn get_chat_history(
    state: tauri::State<'_, Mutex<memory::MemoryManager>>,
    limit: u32,
) -> Result<Vec<ChatMessage>, AppError> {
    let mgr = state.lock().map_err(|e| AppError::internal(e.to_string()))?;
    let convos = mgr.get_memories(Some("conversation"), limit as usize);
    let messages: Vec<ChatMessage> = convos
        .into_iter()
        .map(|e| {
            let role = if e.content.starts_with("用户:") {
                "user"
            } else {
                "assistant"
            };
            let content = strip_conversation_prefix(&e.content);
            ChatMessage {
                role: role.to_string(),
                content,
                timestamp: e.timestamp,
                animation_triggered: None,
            }
        })
        .rev()
        .collect();
    Ok(messages)
}

#[tauri::command]
fn get_memories(
    state: tauri::State<'_, Mutex<memory::MemoryManager>>,
    kind: Option<String>,
    limit: u32,
) -> Result<Vec<MemoryEntry>, AppError> {
    let mgr = state.lock().map_err(|e| AppError::internal(e.to_string()))?;
    Ok(mgr.get_memories(kind.as_deref(), limit as usize))
}

#[tauri::command]
fn record_interaction(
    state: tauri::State<'_, Mutex<memory::MemoryManager>>,
    kind: String,
    content: String,
) -> Result<(), AppError> {
    let memory_kind = match kind.as_str() {
        "observation" => memory::types::MemoryKind::Observation,
        "interaction" => memory::types::MemoryKind::Interaction,
        "decision" => memory::types::MemoryKind::Decision,
        "conversation" => memory::types::MemoryKind::Conversation,
        _ => memory::types::MemoryKind::Interaction,
    };
    let mut mgr = state.lock().map_err(|e| AppError::internal(e.to_string()))?;
    mgr.record(memory_kind, content, 0.5);
    Ok(())
}

#[tauri::command]
fn read_clipboard() -> Result<String, AppError> {
    let text = chat::read_clipboard_text();
    eprintln!("[PetCmd] read_clipboard returned {} chars", text.len());
    Ok(text)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = dotenvy::dotenv();

    let memory_path = memory_file_path();
    eprintln!(
        "[PetSys] memory file: {}",
        memory_path.display()
    );
    let memory_manager = Mutex::new(memory::MemoryManager::new(memory_path));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(memory_manager)
        .invoke_handler(tauri::generate_handler![
            greet,
            list_animations,
            decide_next_state,
            get_llm_info,
            update_llm_config,
            send_message,
            get_chat_history,
            get_memories,
            record_interaction,
            read_clipboard,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
