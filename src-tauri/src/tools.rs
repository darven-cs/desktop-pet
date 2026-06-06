use std::collections::HashMap;

use crate::types::Decision;

/// A function that executes a tool and returns its string output.
pub type ToolFn = fn(args: &serde_json::Value) -> Result<String, String>;

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: serde_json::Value,
    pub handler: ToolFn,
    /// If true, the tool's result is a JSON-serialized Decision — the caller
    /// should return it immediately instead of feeding the result back to the LLM.
    pub is_terminal: bool,
}

pub struct ToolRegistry {
    tools: HashMap<String, ToolDef>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self { tools: HashMap::new() };
        registry.register(get_current_time_tool());
        registry.register(switch_animation_tool());
        registry.register(speak_to_user_tool());
        registry
    }

    pub fn register(&mut self, tool: ToolDef) {
        self.tools.insert(tool.name.to_string(), tool);
    }

    /// Build the schema for all tools (decision flow).
    pub fn schema(&self) -> Vec<serde_json::Value> {
        self.build_schema(false)
    }

    /// Build the schema for info-only tools (chat flow — no terminal tools).
    pub fn info_schema(&self) -> Vec<serde_json::Value> {
        self.build_schema(true)
    }

    fn build_schema(&self, info_only: bool) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .filter(|t| !info_only || !t.is_terminal)
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect()
    }

    /// Check if a tool is terminal (its result is a Decision, not info for the LLM).
    pub fn is_terminal(&self, name: &str) -> bool {
        self.tools.get(name).map(|t| t.is_terminal).unwrap_or(false)
    }

    /// Execute a tool by name, returning its string output.
    pub fn execute(&self, name: &str, arguments: &serde_json::Value) -> Result<String, String> {
        match self.tools.get(name) {
            Some(tool) => (tool.handler)(arguments),
            None => Err(format!("unknown tool: {}", name)),
        }
    }
}

// ---- built-in tools ----

fn get_current_time_tool() -> ToolDef {
    ToolDef {
        name: "get_current_time",
        description: "获取当前精确的日期和时间（包括星期几），用于了解现在是什么时候",
        parameters: serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        is_terminal: false,
        handler: |_args| {
            let now = chrono::Local::now();
            Ok(format!(
                "{} (星期{})",
                now.format("%Y年%m月%d日 %H:%M:%S"),
                match now.format("%u").to_string().as_str() {
                    "1" => "一", "2" => "二", "3" => "三",
                    "4" => "四", "5" => "五", "6" => "六",
                    "7" => "日", _ => "?",
                }
            ))
        },
    }
}

fn switch_animation_tool() -> ToolDef {
    ToolDef {
        name: "switch_animation",
        description: "切换宠物的动画。先查询时间再根据情境选择合适的动画，深夜选安静的动作（think/touch_nose），白天可以活泼一些",
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "to": {
                    "type": "string",
                    "enum": ["touch_nose", "think", "poop"],
                    "description": "要切换到的动画 ID"
                },
                "reason": {
                    "type": "string",
                    "description": "切换的原因（简短说明）"
                }
            },
            "required": ["to", "reason"]
        }),
        is_terminal: true,
        handler: |args| {
            let to = args["to"].as_str().unwrap_or("touch_nose");
            let reason = args["reason"].as_str().unwrap_or("");
            let decision = Decision::Switch {
                to: to.to_string(),
                reason: if reason.is_empty() { None } else { Some(reason.to_string()) },
            };
            serde_json::to_string(&decision).map_err(|e| e.to_string())
        },
    }
}

fn speak_to_user_tool() -> ToolDef {
    ToolDef {
        name: "speak_to_user",
        description: "主动给用户发一条消息（弹出对话框显示）。用于问用户在干嘛、提醒休息、表达心情、深夜问候等。先查询时间再决定说话内容",
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "要对用户说的话（简短自然，1-2句话）"
                },
                "animation": {
                    "type": "string",
                    "enum": ["touch_nose", "think", "poop"],
                    "description": "可选的伴随动画 ID"
                }
            },
            "required": ["message"]
        }),
        is_terminal: true,
        handler: |args| {
            let message = args["message"].as_str().unwrap_or("").to_string();
            let animation = args["animation"].as_str().map(String::from);
            let decision = Decision::Speak { message, animation };
            serde_json::to_string(&decision).map_err(|e| e.to_string())
        },
    }
}
