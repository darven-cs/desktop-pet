use std::sync::Mutex;

use crate::memory;
use crate::types::{AppError, Decision, DecisionContext};

pub async fn decide_next_state(
    state: &tauri::State<'_, Mutex<memory::MemoryManager>>,
    mut ctx: DecisionContext,
) -> Result<Decision, AppError> {
    if ctx.ticker_interval_ms == 0 {
        return Err(AppError::invalid_context("ticker_interval_ms must be > 0"));
    }

    let static_cfg = crate::llm::load_static_config();

    // R8: runtime enabled override via settings panel.
    if ctx.llm_enabled.unwrap_or(true) == false {
        eprintln!("[PetLLM] disabled (runtime override), returning Stay");
        return Ok(Decision::Stay);
    }

    // R4: static config disabled → only block if no runtime API credentials.
    let has_runtime_creds = ctx
        .llm_api_key
        .as_deref()
        .filter(|s| !s.is_empty())
        .is_some();
    if !static_cfg.enabled && !has_runtime_creds {
        eprintln!("[PetLLM] disabled (no API key in .env or settings), returning Stay");
        return Ok(Decision::Stay);
    }

    // Build memory context and inject into DecisionContext.
    {
        let mgr = state.lock().map_err(|e| AppError::internal(e.to_string()))?;
        let mem_ctx = mgr.build_context();
        ctx.memory_context = Some(mem_ctx);
    }

    // Check and trigger digestion if needed (deferred — uses a quick prune instead).
    {
        let mut mgr = state.lock().map_err(|e| AppError::internal(e.to_string()))?;
        // Prune old low-importance LTM entries every ~50 ticks (cheap approximation).
        mgr.prune();
    }

    let tools = crate::tools::ToolRegistry::new();
    match crate::llm::send_chat_request(&static_cfg, &ctx, &tools).await {
        Ok(decision) => {
            // Record decision in memory.
            if let Ok(mut mgr) = state.lock() {
                let content = match &decision {
                    Decision::Stay => "stay".to_string(),
                    Decision::Switch { to, reason } => {
                        format!("switch to {} (reason: {})", to, reason.as_deref().unwrap_or("?"))
                    }
                    Decision::Speak { message, .. } => {
                        format!("主动对话: {}", message)
                    }
                    Decision::EnterIdle => "enter_idle".to_string(),
                    Decision::ExitIdle => "exit_idle".to_string(),
                    Decision::Wait { duration_ms, reason } => {
                        format!("wait {}ms (reason: {})", duration_ms, reason.as_deref().unwrap_or("?"))
                    }
                };
                mgr.record(memory::types::MemoryKind::Decision, content, 0.3);
            }

            // R10: validate animation id if switching.
            if let Decision::Switch { ref to, .. } = decision {
                if !crate::registry::is_known_animation(to) {
                    eprintln!("[PetLLM] invalid animation id: {}, falling back to Stay", to);
                    return Ok(Decision::Stay);
                }
            }
            Ok(decision)
        }
        Err(e) => {
            eprintln!("[PetLLM] {} — falling back to Stay", e);
            Ok(Decision::Stay)
        }
    }
}
