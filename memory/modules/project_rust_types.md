---
name: Rust 数据类型
description: src-tauri/src/types.rs 是前后端共享结构的唯一源，TS 端手写镜像
type: project
---

# Rust 数据类型

**状态**：已上线
**上线时间**：2026-06-06
**所属业务**：核心基础设施

`src-tauri/src/types.rs` 定义 01 spec 涉及的所有 serde 结构。TS 端在 `src/types/pet.ts` 手写镜像（02 spec 计划切 ts-rs 自动生成）。

---

## 一、模块结构

```
src-tauri/src/
├── types.rs       ← 共享结构（AnimationEntry / Decision / Phase ...）
├── registry.rs    ← list_animations 实现（扫 public/sprites/）
├── decider.rs     ← decide_next_state 占位（永远返回 Stay）
└── lib.rs         ← 2 个 #[tauri::command]
```

## 二、契约要点

- **R7**：Rust 是唯一源；TS 文件头部有 `// Mirrored from src-tauri/src/types.rs` 注释
- **字段名**：Rust 用 snake_case，`#[serde(rename_all = "camelCase")]` 暴露给 TS
- **枚举变体**：
  - `LoopMode` → `"infinite" | "once"`（lowercase）
  - `Phase` → `"playing" | "idle" | "transitioning"`（lowercase）
  - `Decision` → `tag = "action"` + `rename_all = "snake_case"` → `{"action": "stay"}` / `{"action": "switch", "to": ...}` / `{"action": "enter_idle"}` / `{"action": "exit_idle"}`
  - `AppErrorCode` → `rename_all = "SCREAMING_SNAKE_CASE"` → `"E_ANIM_NOT_FOUND"` 等

## 三、AppError 设计

`AppError` 是 **struct** 而非 enum：
```rust
pub struct AppError { pub code: AppErrorCode, pub message: String }
```
带 4 个构造函数：`anim_not_found` / `frames_missing` / `invalid_context` / `internal`。这样序列化直接得到 `{"code": "E_XXX", "message": "..."}`，TS 端 1:1 对应。

## 四、当前 metadata 表（01 hardcoded）

```rust
fn known_meta() -> HashMap<&'static str, AnimationMeta> {
    // touch_nose: fps=25, loop=Infinite
    // think:     fps=25, loop=Infinite
    // poop:      fps=25, loop=Once
}
```
后续加动画：要么在此 map 补一行，要么 02+ 改 sidecar TOML。

## 五、相关记忆

- [SDD 工作流](project_sdd_workflow.md) — 改 spec 必须走 CHANGED 流程
- [精灵图管线](project_sprite_pipeline.md) — frame_count/width/height 来自 PNG IHDR

---

## 变更历史

- 2026-06-06：建模块。types.rs / registry.rs / decider.rs / 2 个 command 上线
- 2026-06-07：02 spec。`DecisionContext` 新增 4 个可选字段（time_of_day / recent_history / llm_enabled / pet_personality）；新增 `LlmInfo` + `LlmConfigUpdate` 类型；`decider.rs` 从 stub 改为 LLM 调用
- 2026-06-07：Chat + Memory。新增 `memory/` 模块（types / short_term / long_term / retrieval / digest / mod.rs）+ `chat.rs`；新增 `MemoryEntry` `MemoryKind` `ChatMessage` `ChatResponse` 类型；`lib.rs` 新增 5 个 command（send_message / get_chat_history / get_memories / record_interaction / read_clipboard）；`decider.rs` 注入记忆上下文 + Tauri State；+ chrono + uuid 依赖
- 2026-06-07（同日二轮）：`DecisionContext` 新增 `pet_name: Option<String>` + `memory_context: Option<String>`（#[serde(skip_deserializing)]）；`Decision::Speak` 变体已存在于上一轮（message + animation）；新增工具系统 `tools.rs`（ToolRegistry / ToolDef + is_terminal + info_schema）
