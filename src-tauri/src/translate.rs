use crate::llm::{do_http_request, load_static_config, normalize_endpoint, timeout_ms};
use crate::types::AppError;

const DEFAULT_FROM_LANG: &str = "en";
const DEFAULT_TO_LANG: &str = "zh";

/// Translate text from source language to target language using LLM.
pub async fn translate_text(
    text: &str,
    from_lang: Option<&str>,
    to_lang: Option<&str>,
    api_key: Option<&str>,
    endpoint: Option<&str>,
    model: Option<&str>,
) -> Result<String, AppError> {
    if text.trim().is_empty() {
        return Err(AppError::invalid_context("empty text"));
    }

    // Filter out non-translatable content
    let trimmed = text.trim();
    if trimmed.starts_with("call:") || trimmed.starts_with("{\"") || trimmed.starts_with("function") || trimmed.starts_with("()") {
        return Err(AppError::invalid_context("not translatable content"));
    }

    // Skip if text contains too many Chinese characters (already Chinese)
    let chinese_chars: usize = trimmed.chars().filter(|c| {
        let code = *c as u32;
        (0x4E00..=0x9FFF).contains(&code) || (0x3400..=0x4DBF).contains(&code)
    }).count();
    if chinese_chars > trimmed.len() / 2 {
        return Err(AppError::invalid_context("already Chinese"));
    }

    let static_config = load_static_config();

    // Use runtime config if provided, otherwise fall back to static config
    let api_key = api_key.filter(|s| !s.is_empty()).unwrap_or(&static_config.api_key);
    let endpoint = endpoint.filter(|s| !s.is_empty()).unwrap_or(&static_config.endpoint);
    let model = model.filter(|s| !s.is_empty()).unwrap_or(&static_config.model);

    eprintln!("[PetTranslate] api_key_present={}, endpoint={}", !api_key.is_empty(), endpoint);

    if api_key.is_empty() {
        return Err(AppError::internal("No API key configured"));
    }

    let from = from_lang.unwrap_or(DEFAULT_FROM_LANG);
    let to = to_lang.unwrap_or(DEFAULT_TO_LANG);

    let system_prompt = format!(
        r#"你是一个专业的翻译引擎。你的任务是将文本从{}翻译成{}。

翻译规则：
1. 只返回翻译结果，不要添加任何解释、评论或额外内容
2. 保持原文的语气和风格
3. 对于专有名词或习语，在保持准确的前提下可以适当本地化
4. 如果文本是口语化的，翻译也应该口语化
5. 只返回翻译后的文字，不要返回任何其他内容

直接返回翻译结果，不要用引号包裹，不要加任何前缀。"#,
        language_name(from),
        language_name(to)
    );

    let user_prompt = format!("翻译以下文本：\n{}", text);

    let timeout = timeout_ms(10000);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout))
        .build()
        .map_err(|e| AppError::internal(e.to_string()))?;

    let messages = vec![
        serde_json::json!({ "role": "system", "content": system_prompt }),
        serde_json::json!({ "role": "user", "content": user_prompt }),
    ];

    let normalized_endpoint = normalize_endpoint(endpoint);

    let body = serde_json::json!({
        "model": model,
        "temperature": 0.3,
        "max_tokens": 1024,
        "messages": messages,
    });

    eprintln!(
        "[PetTranslate] translating {} chars from {} to {} using {}",
        text.len(),
        from,
        to,
        model
    );

    let resp_body = do_http_request(&client, &normalized_endpoint, &api_key, &body)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let root: serde_json::Value =
        serde_json::from_str(&resp_body).map_err(|e| AppError::internal(e.to_string()))?;

    let translation = root["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();

    eprintln!("[PetTranslate] result: {}", translation);
    Ok(translation)
}

/// Convert language code to human-readable name.
fn language_name(code: &str) -> &'static str {
    match code.to_lowercase().as_str() {
        "en" | "english" => "英文 (English)",
        "zh" | "chinese" => "中文 (Chinese)",
        "ja" | "japanese" => "日文 (Japanese)",
        "ko" | "korean" => "韩文 (Korean)",
        "fr" | "french" => "法文 (French)",
        "de" | "german" => "德文 (German)",
        "es" | "spanish" => "西班牙文 (Spanish)",
        "ru" | "russian" => "俄文 (Russian)",
        "ar" | "arabic" => "阿拉伯文 (Arabic)",
        "pt" | "portuguese" => "葡萄牙文 (Portuguese)",
        "it" | "italian" => "意大利文 (Italian)",
        _ => Box::leak(code.to_string().into_boxed_str()),
    }
}