---
name: 工具系统
description: ToolRegistry 统一管理 LLM 工具（get_current_time / switch_animation / speak_to_user），支持终端/非终端工具分流
type: project
---

# 工具系统

**状态**：已上线
**上线时间**：2026-06-07
**所属业务**：02 LLM 决策 + 05 对话

---

## 一、设计

所有 LLM 动作（切换动画、主动说话、查询信息）统一为 OpenAI function calling 工具。`ToolRegistry` 管理注册和执行：

```rust
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: serde_json::Value,  // JSON Schema
    pub handler: ToolFn,                // fn(&Value) -> Result<String, String>
    pub is_terminal: bool,              // true → 结果作为 Decision 直接返回
}
```

## 二、工具分类

| 工具 | 类型 | 用途 |
|------|------|------|
| `get_current_time` | 非终端（信息） | 查询当前精确时间，结果反馈给 LLM |
| `switch_animation` | 终端 | 切换动画，返回 `Decision::Switch` |
| `speak_to_user` | 终端 | 主动说话，返回 `Decision::Speak` |

## 三、schema() vs info_schema()

- `schema()` — 返回全部工具（决策流使用，可调用终端工具）
- `info_schema()` — 仅返回非终端工具（对话流使用，避免对话中误调 switch/speak）

## 四、工具调用流程

```
LLM 请求 (tools: schema)
  → LLM 返回 tool_calls
  → Rust 执行工具
  → 终端工具：解析 Decision，立即返回
  → 非终端工具：结果追加到 messages，发送第二轮请求
  → LLM 基于结果做最终决策
```

最多 2 轮 HTTP 往返。

## 五、扩展方式

新增工具只需调 `registry.register(ToolDef{...})`，不修改任何调用代码。

---

## 变更历史

- 2026-06-07：建模块。3 个内置工具 + ToolRegistry + 终端/非终端分离。
