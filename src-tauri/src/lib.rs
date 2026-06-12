mod agent;
mod chat;
mod decider;
mod llm;
mod memory;
mod registry;
mod tools;
mod translate;
mod types;

use std::sync::Mutex;

use memory::types::{ChatMessage, MemoryEntry};
use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, WebviewWindow};
use types::{AppError, Decision, DecisionContext};

const TRANSLATE_PANEL_EVENT: &str = "translate-panel-toggle";

const CTX_MENU_EVENT: &str = "context-menu-click";

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
async fn agent_decide(
    state: tauri::State<'_, Mutex<memory::MemoryManager>>,
    events: Vec<agent::events::PetEvent>,
    context: DecisionContext,
) -> Result<agent::AgentResult, AppError> {
    eprintln!(
        "[PetCmd] agent_decide called with {} events",
        events.len()
    );

    let static_cfg = llm::load_static_config();

    // Check LLM enabled.
    let llm_enabled = context.llm_enabled.unwrap_or(true);
    if !llm_enabled || (!static_cfg.enabled && context.llm_api_key.as_deref().filter(|s| !s.is_empty()).is_none()) {
        eprintln!("[PetAgent] LLM disabled, returning Stay");
        return Ok(agent::AgentResult {
            decision: Decision::Stay,
            steps_used: 0,
            tool_calls_made: vec![],
        });
    }

    // Build memory context and inject into DecisionContext.
    let mut ctx = context;
    {
        let mgr = state.lock().map_err(|e| AppError::internal(e.to_string()))?;
        let mem_ctx = mgr.build_context();
        ctx.memory_context = Some(mem_ctx);
    }

    let tools = tools::ToolRegistry::new();
    let mut budget = agent::budget::AgentBudget::new();

    let result = agent::run_agent_loop(&static_cfg, &ctx, &events, &tools, &mut budget)
        .await?;

    // Record decision in memory.
    if let Ok(mut mgr) = state.lock() {
        let content = match &result.decision {
            Decision::Stay => "stay (agent)".to_string(),
            Decision::Switch { to, reason } => {
                format!("switch to {} (reason: {})", to, reason.as_deref().unwrap_or("?"))
            }
            Decision::Speak { message, .. } => {
                format!("主动对话(agent): {}", message)
            }
            Decision::EnterIdle => "enter_idle (agent)".to_string(),
            Decision::ExitIdle => "exit_idle (agent)".to_string(),
            Decision::Wait { duration_ms, reason } => {
                format!("wait {}ms (reason: {})", duration_ms, reason.as_deref().unwrap_or("?"))
            }
            Decision::SetReminder { message, delay_seconds } => {
                format!("set_reminder(agent): {} ({}s后)", message, delay_seconds)
            }
        };
        mgr.record(memory::types::MemoryKind::Decision, content, 0.3);
    }

    // Validate animation id if switching.
    if let Decision::Switch { ref to, .. } = result.decision {
        if !registry::is_known_animation(to) {
            eprintln!("[PetAgent] invalid animation id: {}, falling back to Stay", to);
            return Ok(agent::AgentResult {
                decision: Decision::Stay,
                steps_used: result.steps_used,
                tool_calls_made: result.tool_calls_made,
            });
        }
    }

    eprintln!(
        "[PetAgent] result: decision={:?}, steps={}, tools={:?}",
        result.decision, result.steps_used, result.tool_calls_made
    );
    Ok(result)
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

#[tauri::command]
async fn translate_text(
    text: String,
    from_lang: Option<String>,
    to_lang: Option<String>,
    api_key: Option<String>,
    endpoint: Option<String>,
    model: Option<String>,
) -> Result<String, AppError> {
    eprintln!(
        "[PetCmd] translate_text called with: text='{}', from={:?}, to={:?}",
        if text.len() > 50 {
            format!("{}...", text.chars().take(50).collect::<String>())
        } else {
            text.clone()
        },
        from_lang,
        to_lang
    );
    translate::translate_text(
        &text,
        from_lang.as_deref(),
        to_lang.as_deref(),
        api_key.as_deref(),
        endpoint.as_deref(),
        model.as_deref(),
    )
    .await
}

/// Build the right-click context menu and show it as a native popup at the
/// cursor position. Using Tauri native menu (instead of an in-webview <div>)
/// avoids WebView2's Chromium-layer context menu flash on Windows.
#[tauri::command]
fn show_context_menu(app: AppHandle, window: WebviewWindow) -> Result<(), AppError> {
    let status = MenuItemBuilder::with_id("ctx.status", "宠物状态")
        .build(&app)
        .map_err(|e| AppError::internal(format!("build status: {e}")))?;
    let settings = MenuItemBuilder::with_id("ctx.settings", "宠物设定")
        .build(&app)
        .map_err(|e| AppError::internal(format!("build settings: {e}")))?;
    let chat = MenuItemBuilder::with_id("ctx.chat", "宠物对话")
        .build(&app)
        .map_err(|e| AppError::internal(format!("build chat: {e}")))?;
    let memory = MenuItemBuilder::with_id("ctx.memory", "宠物记忆")
        .build(&app)
        .map_err(|e| AppError::internal(format!("build memory: {e}")))?;
    let separator = PredefinedMenuItem::separator(&app)
        .map_err(|e| AppError::internal(format!("build separator: {e}")))?;
    let exit = MenuItemBuilder::with_id("ctx.exit", "退出")
        .build(&app)
        .map_err(|e| AppError::internal(format!("build exit: {e}")))?;

    let menu = Menu::with_items(&app, &[&status, &settings, &chat, &memory, &separator, &exit])
        .map_err(|e| AppError::internal(format!("build menu: {e}")))?;

    window
        .popup_menu(&menu)
        .map_err(|e| AppError::internal(format!("popup_menu: {e}")))?;
    Ok(())
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
                .plugin(tauri_plugin_clipboard_manager::init())
        .manage(memory_manager)
        .setup(|app| {
            use tauri_plugin_clipboard_manager::ClipboardExt;

            let app_handle = app.handle().clone();

            // Start clipboard monitoring task
            tauri::async_runtime::spawn(async move {
                let mut last_clipboard = String::new();
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                    if let Ok(text) = app_handle.clipboard().read_text() {
                        let text = text.trim().to_string();
                        if !text.is_empty() && text != last_clipboard {
                            last_clipboard = text.clone();
                            // Only trigger for reasonably short text (not huge clipboard content)
                            if text.len() < 500 {
                                eprintln!("[PetSys] clipboard changed, showing translate panel");
                                if let Err(e) = app_handle.emit(TRANSLATE_PANEL_EVENT, text) {
                                    eprintln!("[PetError] emit {TRANSLATE_PANEL_EVENT} failed: {e}");
                                }
                            }
                        }
                    }
                }
            });

            eprintln!("[PetSys] clipboard monitor started");
            Ok(())
        })
        .on_menu_event(|app, event| {
            let id = event.id().0.clone();
            eprintln!("[PetMenu] click: {id}");
            if let Err(e) = app.emit(CTX_MENU_EVENT, id) {
                eprintln!("[PetError] emit {CTX_MENU_EVENT} failed: {e}");
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            list_animations,
            decide_next_state,
            agent_decide,
            get_llm_info,
            update_llm_config,
            send_message,
            get_chat_history,
            get_memories,
            record_interaction,
            read_clipboard,
            show_context_menu,
            translate_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
