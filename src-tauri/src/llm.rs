// OpenAI-compatible LLM client. Reads static config from env vars (spec 02 §3.4).

use crate::types::{Decision, DecisionContext};
use std::time::Duration;

const DEFAULT_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const DEFAULT_TEMPERATURE: f32 = 0.7;
const DEFAULT_MAX_TOKENS: u32 = 256;
const DEFAULT_ENABLED: bool = true;

const DEFAULT_SYSTEM_PROMPT: &str = r#"你是一只可爱的桌面宠物。你住在用户的桌面上，通过切换不同的动画和主动说话来表达自己。

你的性格：好奇心旺盛、偶尔偷懒、喜欢吸引用户注意。

## 你的工具箱（必须使用工具来执行动作）

你有以下工具可用，每次决策必须调用其中一个：

1. **get_current_time** — 查询当前精确时间（信息工具，查完后可以继续调用其他工具）
2. **switch_animation** — 切换动画到 touch_nose/think/poop，需要给出原因
3. **speak_to_user** — 主动弹出对话框对用户说话，可以附带动画

## 决策规则

1. 不要连续3次以上播放同一动画
2. 如果用户很久没互动（>5分钟），主动说话或做有趣动作
3. 根据实际时间判断活跃度（先调用 get_current_time 查询，再决定）
4. 偶尔（~20%概率）在动画之间插入 think 发呆
5. 每隔3-5次 tick，应该主动 speak_to_user，而不是一直沉默
6. 让用户感到这只宠物有性格、不可预测但又可爱
7. 深夜时如果要说话，语气温柔关心；白天可以活泼一些
8. **重要**：如果在记忆上下文中看到最近的对话或事件，请基于这些记忆做决策。比如用户说过什么、你之前做了什么——这让你看起来有记忆、有个性

## 典型决策流程

- 想切动画 → 先调 get_current_time 查时间 → 再调 switch_animation
- 想主动说话 → 先调 get_current_time 查时间 → 再调 speak_to_user
- 想安静不动 → 直接调 switch_animation(to="touch_nose", reason="保持安静")

不要返回文字，直接调用工具。"#;

pub struct LlmStaticConfig {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: String,
    pub enabled: bool,
}

pub enum LlmError {
    Http(u16, String),
    Network(String),
    Parse(String),
    Timeout(u64),
    Disabled,
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(status, body) => write!(f, "HTTP {}: {}", status, body),
            Self::Network(e) => write!(f, "network error: {}", e),
            Self::Parse(e) => write!(f, "parse error: {}", e),
            Self::Timeout(ms) => write!(f, "timeout after {}ms", ms),
            Self::Disabled => write!(f, "LLM disabled"),
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_float(key: &str, default: f32) -> f32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(default)
}

/// Load static config from env vars. Call once per tick (cheap).
pub fn load_static_config() -> LlmStaticConfig {
    let api_key = env_or("LLM_API_KEY", "");
    let system_prompt_override = env_or("LLM_SYSTEM_PROMPT", "");
    let system_prompt = if system_prompt_override.is_empty() {
        DEFAULT_SYSTEM_PROMPT.to_string()
    } else {
        system_prompt_override
    };

    // R3: empty api_key → auto-disable
    let enabled = env_bool("LLM_ENABLED", DEFAULT_ENABLED) && !api_key.is_empty();

    LlmStaticConfig {
        endpoint: env_or("LLM_API_ENDPOINT", DEFAULT_ENDPOINT),
        api_key,
        model: env_or("LLM_MODEL", DEFAULT_MODEL),
        temperature: env_float("LLM_TEMPERATURE", DEFAULT_TEMPERATURE),
        max_tokens: env_u32("LLM_MAX_TOKENS", DEFAULT_MAX_TOKENS),
        system_prompt,
        enabled,
    }
}

/// Build the system message, overriding the personality section if user provided one.
pub fn build_system_prompt(
    config: &LlmStaticConfig,
    pet_personality: Option<&str>,
    pet_name: Option<&str>,
) -> String {
    let base = config.system_prompt.clone();

    // Inject pet name into the opening line if provided.
    let prompt = match pet_name.filter(|n| !n.is_empty()) {
        Some(name) => base.replace(
            "你是一只可爱的桌面宠物。你住在用户的桌面上",
            &format!("你叫「{}」，是一只可爱的桌面宠物。你住在用户的桌面上", name),
        ),
        None => base,
    };

    match pet_personality {
        Some(p) if !p.is_empty() => {
            let mut prompt = prompt.replace(
                "你的性格：好奇心旺盛、偶尔偷懒、喜欢吸引用户注意。",
                &format!("你的性格：{}。", p),
            );
            if p.contains("问") || p.contains("说话") || p.contains("聊") || p.contains("对话") {
                prompt.push_str("\n\n⚠️ 重要提示：主人给你设定的性格表明你喜欢主动和主人交流。请务必在每次决策时认真考虑是否应该用 speak 主动给主人发消息，不要一直沉默。");
            }
            prompt
        }
        _ => prompt,
    }
}

/// Build the user message from DecisionContext.
pub fn build_user_prompt(ctx: &DecisionContext) -> String {
    let seconds_since = if ctx.last_interaction_at > 0 {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        format!("{}", (now_ms.saturating_sub(ctx.last_interaction_at)) / 1000)
    } else {
        "N/A".to_string()
    };

    let history = ctx
        .recent_history
        .as_ref()
        .map(|h| h.join(" → "))
        .unwrap_or_else(|| "无".to_string());

    let time = ctx
        .time_of_day
        .as_deref()
        .unwrap_or("未知");

    let mut parts = vec![format!(
        "当前状态：\n\
         - 动画：{}、阶段：{}、已循环：{} 次\n\
         - 最近动画：{}\n\
         - 距上次互动：{} 秒\n\
         - 当前时间：{}",
        ctx.current_state.current,
        serde_phase(&ctx.current_state.phase),
        ctx.current_state.iteration,
        history,
        seconds_since,
        time,
    )];

    // Inject memory context if available.
    if let Some(ref mc) = ctx.memory_context {
        let trimmed = mc.trim();
        if !trimmed.is_empty() && trimmed != "(暂无记忆上下文)" {
            parts.push(format!("\n{}", trimmed));
        }
    }

    parts.push("\n决定下一步动作。".to_string());
    parts.join("\n")
}

fn serde_phase(phase: &crate::types::Phase) -> &str {
    match phase {
        crate::types::Phase::Playing => "playing",
        crate::types::Phase::Idle => "idle",
        crate::types::Phase::Transitioning => "transitioning",
    }
}

/// Auto-append /v1/chat/completions if the user only provided a base URL.
pub fn normalize_endpoint(url: &str) -> String {
    if url.contains("/chat/completions") {
        return url.to_string();
    }
    // Looks like a base URL — append standard OpenAI path.
    let trimmed = url.trim_end_matches('/');
    format!("{}/v1/chat/completions", trimmed)
}

/// Timeout for LLM requests (R2): max(10s, ticker_interval * 0.8).
pub fn timeout_ms(ticker_interval_ms: u32) -> u64 {
    let min = 10_000u64;
    let ratio = (ticker_interval_ms as f64 * 0.8) as u64;
    min.max(ratio)
}

/// Send chat request with tool support. The LLM may call tools; Rust executes
/// them and feeds results back for a final decision (max 2 round-trips).
/// Runtime overrides (endpoint, api_key, model) from DecisionContext take
/// precedence over static env config.
pub async fn send_chat_request(
    config: &LlmStaticConfig,
    ctx: &DecisionContext,
    tools: &crate::tools::ToolRegistry,
) -> Result<Decision, LlmError> {
    let llm_enabled = ctx.llm_enabled.unwrap_or(true);
    if !llm_enabled {
        return Err(LlmError::Disabled);
    }

    let endpoint = normalize_endpoint(
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
        return Err(LlmError::Disabled);
    }

    let pet_personality = ctx.pet_personality.as_deref();
    let pet_name = ctx.pet_name.as_deref();
    let system_prompt = build_system_prompt(config, pet_personality, pet_name);
    let user_prompt = build_user_prompt(ctx);

    let timeout = timeout_ms(ctx.ticker_interval_ms);

    let masked_key = if api_key.len() > 8 {
        format!("{}...{}", &api_key[..4], &api_key[api_key.len() - 4..])
    } else {
        "***".to_string()
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout))
        .build()
        .map_err(|e| LlmError::Network(e.to_string()))?;

    // Round 1: send with tools, allowing the LLM to call tools.
    let tools_schema = tools.schema();
    let mut messages: Vec<serde_json::Value> = vec![
        serde_json::json!({ "role": "system", "content": system_prompt }),
        serde_json::json!({ "role": "user", "content": user_prompt }),
    ];

    eprintln!(
        "[PetLLM] request to {}, model={}, key={}, tokens={}, tools={}",
        endpoint, model, masked_key, config.max_tokens, tools_schema.len()
    );

    let mut body = serde_json::json!({
        "model": model,
        "temperature": config.temperature,
        "max_tokens": config.max_tokens,
        "messages": messages,
        "tools": tools_schema,
    });

    let resp_body = do_http_request(&client, &endpoint, &api_key, &body).await?;
    let root: serde_json::Value =
        serde_json::from_str(&resp_body).map_err(|e| LlmError::Parse(e.to_string()))?;

    let choice = &root["choices"][0];
    let msg = &choice["message"];

    // Process tool calls.
    if let Some(tool_calls) = msg["tool_calls"].as_array() {
        if tool_calls.is_empty() {
            return parse_decision_from_message(msg);
        }

        eprintln!("[PetLLM] tool calls: {}", tool_calls.len());

        // Append assistant message with tool_calls.
        messages.push(msg.clone());

        let mut terminal_decision: Option<Decision> = None;

        for tc in tool_calls {
            let call_id = tc["id"].as_str().unwrap_or("?");
            let fn_name = tc["function"]["name"].as_str().unwrap_or("?");
            let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
            let args: serde_json::Value =
                serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null);

            eprintln!("[PetLLM] tool call: {}({})", fn_name, args_str);

            let result = tools.execute(fn_name, &args).unwrap_or_else(|e| e);
            eprintln!("[PetLLM] tool result: {}", result);

            if tools.is_terminal(fn_name) {
                // Terminal tool → parse as Decision and return immediately.
                let decision: Decision = serde_json::from_str(&result)
                    .map_err(|e| LlmError::Parse(format!("terminal tool parse: {} | raw='{}'", e, result)))?;
                terminal_decision = Some(decision);
                break; // Don't process further tools after terminal.
            }

            messages.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": call_id,
                "content": result,
            }));
        }

        // If a terminal tool produced a decision, return it.
        if let Some(decision) = terminal_decision {
            eprintln!(
                "[PetLLM] response: {}",
                serde_json::to_string(&decision).unwrap_or_default()
            );
            return Ok(decision);
        }

        // Round 2: non-terminal tools completed, ask LLM for final decision.
        body = serde_json::json!({
            "model": model,
            "temperature": config.temperature,
            "max_tokens": config.max_tokens,
            "messages": messages,
            "tools": tools_schema,
        });

        let resp_body2 = do_http_request(&client, &endpoint, &api_key, &body).await?;
        let root2: serde_json::Value =
            serde_json::from_str(&resp_body2).map_err(|e| LlmError::Parse(e.to_string()))?;
        let msg2 = &root2["choices"][0]["message"];

        // Round 2 may also have tool calls (e.g. get_current_time → switch_animation).
        if let Some(tool_calls2) = msg2["tool_calls"].as_array() {
            if !tool_calls2.is_empty() {
                for tc in tool_calls2 {
                    let fn_name = tc["function"]["name"].as_str().unwrap_or("?");
                    let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                    let args: serde_json::Value =
                        serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null);
                    eprintln!("[PetLLM] tool call (r2): {}({})", fn_name, args_str);
                    let result = tools.execute(fn_name, &args).unwrap_or_else(|e| e);
                    eprintln!("[PetLLM] tool result (r2): {}", result);
                    if tools.is_terminal(fn_name) {
                        let decision: Decision = serde_json::from_str(&result)
                            .map_err(|e| LlmError::Parse(format!("r2 terminal parse: {} | raw='{}'", e, result)))?;
                        eprintln!(
                            "[PetLLM] response: {}",
                            serde_json::to_string(&decision).unwrap_or_default()
                        );
                        return Ok(decision);
                    }
                }
            }
        }

        let decision = parse_decision_from_message(msg2)?;
        eprintln!(
            "[PetLLM] response: {}",
            serde_json::to_string(&decision).unwrap_or_default()
        );
        return Ok(decision);
    }

    // No tool calls — shouldn't happen with the new prompt, but handle gracefully.
    eprintln!("[PetLLM] no tool call, falling back to Stay");
    Ok(Decision::Stay)
}

pub(crate) async fn do_http_request(
    client: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    body: &serde_json::Value,
) -> Result<String, LlmError> {
    let resp = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout(0) // timeout is computed earlier
            } else {
                LlmError::Network(e.to_string())
            }
        })?;

    let status = resp.status().as_u16();
    let resp_body = resp.text().await.unwrap_or_default();

    if status != 200 {
        let truncated: String = resp_body.chars().take(200).collect();
        eprintln!("[PetLLM] http error {}: {}", status, truncated);
        return Err(LlmError::Http(status, truncated));
    }
    Ok(resp_body)
}

/// Parse a Decision from a message object (handles both direct content and after-tool-call messages).
fn parse_decision_from_message(msg: &serde_json::Value) -> Result<Decision, LlmError> {
    let content = msg["content"].as_str().unwrap_or("");
    let trimmed = content.trim();

    let decision: Decision = serde_json::from_str(trimmed).map_err(|e| {
        let raw: String = trimmed.chars().take(150).collect();
        LlmError::Parse(format!("{} | raw='{}'", e, raw))
    })?;
    Ok(decision)
}

