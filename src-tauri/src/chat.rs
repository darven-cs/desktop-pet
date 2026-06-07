use std::time::Duration;

use crate::memory::types::{ChatMessage, ChatResponse};
use crate::types::{AppError, DecisionContext};

const MAX_PARSE_RETRIES: u32 = 3;

/// Build the system prompt for chat.
fn build_chat_system_prompt(
    personality: Option<&str>,
    pet_name: Option<&str>,
    memory_context: &str,
) -> String {
    let personality_text = personality
        .filter(|p| !p.is_empty())
        .unwrap_or("好奇心旺盛、偶尔偷懒、喜欢吸引用户注意");

    let identity = match pet_name.filter(|n| !n.is_empty()) {
        Some(name) => format!("你叫「{}」，是一只可爱的桌面宠物。用户正在和你对话。", name),
        None => "你是一只可爱的桌面宠物。用户正在和你对话。".to_string(),
    };

    format!(
        r#"{}

你的性格：{}。

{}

你可以使用 get_current_time 工具查询当前精确时间（如果用户的问题涉及时间，必须先查询再回答）。

请自然地回复用户。你可以：
- 用可爱的语气聊天（简短，3句话以内）
- 分享你观察到的事情（如果有记忆上下文的话）
- 偶尔建议切换动画来表达情绪（touch_nose/think/poop）
- 回答时间类问题时，先调用 get_current_time 工具，再基于准确时间回复

最终你必须返回一个 JSON 对象，格式如下：
{{"message": "回复内容", "animation": null}}

animation 字段为 null 或动画 ID（touch_nose/think/poop）。不要返回任何其他文字。"#,
        identity, personality_text, memory_context
    )
}

/// Build the full user prompt for a chat message, optionally with context text.
fn build_chat_user_prompt(
    message: &str,
    context_text: Option<&str>,
    history: &[ChatMessage],
) -> String {
    let mut parts: Vec<String> = Vec::new();

    if !history.is_empty() {
        parts.push("最近对话历史：".to_string());
        for msg in history.iter().rev().take(5).rev() {
            let role = if msg.role == "user" { "用户" } else { "宠物" };
            parts.push(format!("- {}: {}", role, msg.content));
        }
        parts.push("---".to_string());
    }

    if let Some(ctx) = context_text {
        if !ctx.is_empty() {
            parts.push(format!("用户选中了这段文字：「{}」", ctx));
            parts.push("请基于这段文字和用户的消息进行回复。".to_string());
        }
    }

    parts.push(format!("用户说：{}", message));

    parts.join("\n")
}

/// Try to parse LLM message content as ChatResponse JSON.
/// Returns Ok(ChatResponse) on success, Err(raw_text) on failure.
fn try_parse_chat_response(msg: &serde_json::Value) -> Result<ChatResponse, String> {
    let content = msg["content"].as_str().unwrap_or("");
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("(empty response)".to_string());
    }
    serde_json::from_str::<ChatResponse>(trimmed)
        .map_err(|_| trimmed.to_string())
}

/// Send a chat request and parse the response, with exponential backoff on JSON parse failure.
/// On each retry, feeds the malformed response back to the LLM with a correction instruction.
async fn request_and_parse(
    client: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    model: &str,
    messages: &mut Vec<serde_json::Value>,
    has_tools: bool,
) -> Result<ChatResponse, AppError> {
    let mut backoff_ms = 600u64;

    for attempt in 0..=MAX_PARSE_RETRIES {
        let use_json_format = has_tools || attempt > 0;

        let mut body = serde_json::json!({
            "model": model,
            "temperature": 0.8,
            "max_tokens": 512,
            "messages": messages.clone(),
        });
        if use_json_format {
            body["response_format"] = serde_json::json!({ "type": "json_object" });
        }

        let resp_body = crate::llm::do_http_request(client, endpoint, api_key, &body)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        let root: serde_json::Value =
            serde_json::from_str(&resp_body).map_err(|e| AppError::internal(e.to_string()))?;

        let choice = &root["choices"][0];
        let msg = &choice["message"];

        // If there are tool calls, return a marker so the caller can process them.
        if let Some(tool_calls) = msg["tool_calls"].as_array() {
            if !tool_calls.is_empty() {
                // Package tool_calls into a "pseudo" ChatResponse for the caller.
                return Ok(ChatResponse {
                    message: String::new(),
                    animation: Some("__tool_calls__".to_string()),
                });
            }
        }

        match try_parse_chat_response(msg) {
            Ok(response) => {
                eprintln!(
                    "[PetChat] response: {}",
                    serde_json::to_string(&response).unwrap_or_default()
                );
                return Ok(response);
            }
            Err(raw_text) => {
                if attempt < MAX_PARSE_RETRIES {
                    let preview: String = raw_text.chars().take(100).collect();
                    eprintln!(
                        "[PetChat] JSON parse failed (attempt {}/{}), retrying in {}ms: {}…",
                        attempt + 1,
                        MAX_PARSE_RETRIES,
                        backoff_ms,
                        preview
                    );
                    // Feed the malformed response back with a correction instruction.
                    messages.push(msg.clone());
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": "你上次返回的不是合法 JSON。请严格按照 {\"message\":\"回复内容\",\"animation\":null} 格式返回，不要返回任何其他文字。"
                    }));
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = backoff_ms.saturating_mul(2);
                } else {
                    // Max retries: use raw text as fallback.
                    eprintln!(
                        "[PetChat] max retries exceeded, using raw text as fallback ({} chars)",
                        raw_text.len()
                    );
                    return Ok(ChatResponse {
                        message: raw_text,
                        animation: None,
                    });
                }
            }
        }
    }

    Err(AppError::internal("unreachable"))
}

/// Send a chat message and get a pet response, with tool support.
pub async fn send_chat_message(
    config: &crate::llm::LlmStaticConfig,
    ctx: &DecisionContext,
    message: &str,
    context_text: Option<&str>,
    history: &[ChatMessage],
    memory_context: &str,
    tools: &crate::tools::ToolRegistry,
) -> Result<ChatResponse, AppError> {
    let llm_enabled = ctx.llm_enabled.unwrap_or(true);
    if !llm_enabled {
        return Err(AppError::internal("LLM disabled"));
    }

    let endpoint = crate::llm::normalize_endpoint(
        ctx.llm_api_endpoint
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&config.endpoint),
    );
    let api_key = ctx
        .llm_api_key
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&config.api_key);
    let model = ctx
        .llm_model
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&config.model);

    if api_key.is_empty() {
        return Err(AppError::internal("No API key configured"));
    }

    let pet_personality = ctx.pet_personality.as_deref();
    let pet_name = ctx.pet_name.as_deref();
    let system_prompt = build_chat_system_prompt(pet_personality, pet_name, memory_context);
    let user_prompt = build_chat_user_prompt(message, context_text, history);

    let timeout = crate::llm::timeout_ms(ctx.ticker_interval_ms);

    let masked_key = if api_key.len() > 8 {
        format!("{}...{}", &api_key[..4], &api_key[api_key.len() - 4..])
    } else {
        "***".to_string()
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout))
        .build()
        .map_err(|e| AppError::internal(e.to_string()))?;

    let tools_schema = tools.info_schema();
    let mut messages: Vec<serde_json::Value> = vec![
        serde_json::json!({ "role": "system", "content": system_prompt }),
        serde_json::json!({ "role": "user", "content": user_prompt }),
    ];

    eprintln!(
        "[PetChat] request to {}, model={}, key={}, tools={}",
        endpoint, model, masked_key, tools_schema.len()
    );

    // Round 1: with tools. request_and_parse handles JSON parse retries internally.
    let mut response = request_and_parse(
        &client,
        &endpoint,
        &api_key,
        &model,
        &mut messages,
        !tools_schema.is_empty(),
    )
    .await?;

    // Check if the response has pending tool calls.
    if response.animation.as_deref() == Some("__tool_calls__") {
        // Re-fetch the last assistant message (which contains tool_calls).
        // We need to re-parse the last HTTP response to get the actual tool_calls.
        // request_and_parse already pushed the assistant message to `messages`
        // when retries happened, but for the tool_calls case we need the raw msg.
        //
        // Re-do round 1 without retry wrapper to get the raw response.
        let body = serde_json::json!({
            "model": model,
            "temperature": 0.8,
            "max_tokens": 512,
            "messages": messages,
            "tools": tools_schema,
        });

        let resp_body = crate::llm::do_http_request(&client, &endpoint, &api_key, &body)
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        let root: serde_json::Value =
            serde_json::from_str(&resp_body).map_err(|e| AppError::internal(e.to_string()))?;

        let msg = &root["choices"][0]["message"];
        let tool_calls = msg["tool_calls"].as_array().ok_or_else(|| {
            AppError::internal("expected tool_calls but none found")
        })?;

        eprintln!("[PetChat] tool calls: {}", tool_calls.len());

        messages.push(msg.clone());

        for tc in tool_calls {
            let call_id = tc["id"].as_str().unwrap_or("?");
            let fn_name = tc["function"]["name"].as_str().unwrap_or("?");
            let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
            let args: serde_json::Value =
                serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null);

            eprintln!("[PetChat] tool call: {}({})", fn_name, args_str);

            let result = tools.execute(fn_name, &args).unwrap_or_else(|e| e);
            eprintln!("[PetChat] tool result: {}", result);

            messages.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": call_id,
                "content": result,
            }));
        }

        // Round 2: with tool results, force JSON output. Retry on parse failure.
        response = request_and_parse(
            &client,
            &endpoint,
            &api_key,
            &model,
            &mut messages,
            false, // no tools in round 2
        )
        .await?;
    }

    Ok(response)
}

/// Read system clipboard text. Returns empty string on failure.
pub fn read_clipboard_text() -> String {
    #[cfg(target_os = "linux")]
    {
        for cmd in &["xclip -o -selection clipboard 2>/dev/null", "xsel -b -o 2>/dev/null"] {
            if let Ok(output) = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
            {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !text.is_empty() {
                        return text;
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("pbpaste").output() {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("powershell")
            .args(["-Command", "Get-Clipboard"])
            .output()
        {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }
    }
    String::new()
}
