pub mod budget;
pub mod events;

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::llm::{self, LlmStaticConfig};
use crate::tools::ToolRegistry;
use crate::types::{AppError, Decision, DecisionContext};

use events::PetEvent;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AgentResult {
    pub decision: Decision,
    pub steps_used: u32,
    pub tool_calls_made: Vec<String>,
}

/// Run the agent loop: iteratively call the LLM with tool support until a
/// terminal decision is reached or the budget is exhausted.
pub async fn run_agent_loop(
    config: &LlmStaticConfig,
    ctx: &DecisionContext,
    events: &[PetEvent],
    tools: &ToolRegistry,
    budget: &mut budget::AgentBudget,
) -> Result<AgentResult, AppError> {
    if events.is_empty() {
        return Err(AppError::invalid_context(
            "agent loop requires at least one event",
        ));
    }

    if !budget.check() {
        eprintln!("[PetAgent] rate limited, returning Stay");
        return Ok(AgentResult {
            decision: Decision::Stay,
            steps_used: 0,
            tool_calls_made: vec![],
        });
    }

    let pet_personality = ctx.pet_personality.as_deref();
    let pet_name = ctx.pet_name.as_deref();
    let system_prompt = llm::build_system_prompt(config, pet_personality, pet_name);
    let mut user_prompt = llm::build_user_prompt(ctx);
    let events_summary = ctx.events_summary.as_deref().map(|s| s.to_string())
        .unwrap_or_else(|| events::format_events_summary(events));
    if !events_summary.is_empty() {
        user_prompt.push_str("\n\n");
        user_prompt.push_str(&events_summary);
    }

    let endpoint = llm::normalize_endpoint(
        ctx.llm_api_endpoint
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&config.endpoint),
    );
    let api_key = ctx
        .llm_api_key
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&config.api_key)
        .to_string();
    let model = ctx
        .llm_model
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&config.model)
        .to_string();

    if api_key.is_empty() {
        return Err(AppError::internal("No API key configured"));
    }

    let timeout = llm::timeout_ms(ctx.ticker_interval_ms);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout))
        .build()
        .map_err(|e| AppError::internal(e.to_string()))?;

    let tools_schema = tools.schema();
    let mut messages: Vec<serde_json::Value> = vec![
        serde_json::json!({ "role": "system", "content": system_prompt }),
        serde_json::json!({ "role": "user", "content": user_prompt }),
    ];

    let mut steps = 0u32;
    let mut consecutive_failures = 0u32;
    let mut tool_calls_made: Vec<String> = Vec::new();

    eprintln!(
        "[PetAgent] starting loop, max_steps={}, endpoint={}, model={}",
        budget.max_steps, endpoint, model
    );

    while steps < budget.max_steps {
        steps += 1;
        budget.consume();

        let body = serde_json::json!({
            "model": model,
            "temperature": config.temperature,
            "max_tokens": config.max_tokens,
            "messages": messages,
            "tools": tools_schema,
        });

        eprintln!("[PetAgent] step {}/{}", steps, budget.max_steps);

        let resp_body = match llm::do_http_request(&client, &endpoint, &api_key, &body).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[PetAgent] HTTP error on step {}: {}", steps, e);
                consecutive_failures += 1;
                if consecutive_failures >= 2 {
                    eprintln!("[PetAgent] 2 consecutive failures, returning Stay");
                    return Ok(AgentResult {
                        decision: Decision::Stay,
                        steps_used: steps,
                        tool_calls_made,
                    });
                }
                continue;
            }
        };

        let root: serde_json::Value = match serde_json::from_str(&resp_body) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[PetAgent] JSON parse error on step {}: {}", steps, e);
                consecutive_failures += 1;
                if consecutive_failures >= 2 {
                    return Ok(AgentResult {
                        decision: Decision::Stay,
                        steps_used: steps,
                        tool_calls_made,
                    });
                }
                continue;
            }
        };

        let msg = &root["choices"][0]["message"];

        // Handle tool calls.
        if let Some(tool_calls) = msg["tool_calls"].as_array() {
            if tool_calls.is_empty() {
                // No tool calls — try to parse a decision directly.
                match parse_decision_from_message(msg) {
                    Ok(decision) => {
                        return Ok(AgentResult {
                            decision,
                            steps_used: steps,
                            tool_calls_made,
                        });
                    }
                    Err(_) => {
                        consecutive_failures += 1;
                        if consecutive_failures >= 2 {
                            return Ok(AgentResult {
                                decision: Decision::Stay,
                                steps_used: steps,
                                tool_calls_made,
                            });
                        }
                        continue;
                    }
                }
            }

            consecutive_failures = 0;
            messages.push(msg.clone());

            for tc in tool_calls {
                let call_id = tc["id"].as_str().unwrap_or("?");
                let fn_name = tc["function"]["name"].as_str().unwrap_or("?");
                let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                let args: serde_json::Value =
                    serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null);

                eprintln!("[PetAgent] tool call: {}({})", fn_name, args_str);
                tool_calls_made.push(fn_name.to_string());

                let result = tools.execute(fn_name, &args).unwrap_or_else(|e| e);
                eprintln!("[PetAgent] tool result: {}", result);

                if tools.is_terminal(fn_name) {
                    let decision: Decision = match serde_json::from_str(&result) {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!(
                                "[PetAgent] terminal tool parse error: {} | raw='{}'",
                                e, result
                            );
                            return Ok(AgentResult {
                                decision: Decision::Stay,
                                steps_used: steps,
                                tool_calls_made,
                            });
                        }
                    };
                    eprintln!(
                        "[PetAgent] terminal decision: {}",
                        serde_json::to_string(&decision).unwrap_or_default()
                    );
                    return Ok(AgentResult {
                        decision,
                        steps_used: steps,
                        tool_calls_made,
                    });
                }

                // Non-terminal: feed result back.
                messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": result,
                }));
            }
            // Continue loop to let LLM make the next move.
        } else {
            // No tool calls — try to parse decision from text.
            match parse_decision_from_message(msg) {
                Ok(decision) => {
                    return Ok(AgentResult {
                        decision,
                        steps_used: steps,
                        tool_calls_made,
                    });
                }
                Err(e) => {
                    eprintln!("[PetAgent] parse decision failed: {}", e);
                    consecutive_failures += 1;
                    if consecutive_failures >= 2 {
                        return Ok(AgentResult {
                            decision: Decision::Stay,
                            steps_used: steps,
                            tool_calls_made,
                        });
                    }
                    // Feed the failed response back so the LLM can try again.
                    messages.push(msg.clone());
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": "请调用工具来做出决策，不要直接返回文字。"
                    }));
                }
            }
        }
    }

    // Max steps exhausted.
    eprintln!("[PetAgent] max steps ({}) exhausted, returning Stay", budget.max_steps);
    Ok(AgentResult {
        decision: Decision::Stay,
        steps_used: steps,
        tool_calls_made,
    })
}

/// Parse a Decision from an LLM message object's content field.
fn parse_decision_from_message(msg: &serde_json::Value) -> Result<Decision, String> {
    let content = msg["content"].as_str().unwrap_or("");
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("empty response content".to_string());
    }
    serde_json::from_str::<Decision>(trimmed)
        .map_err(|e| format!("{} | raw='{}'", e, trimmed.chars().take(150).collect::<String>()))
}
