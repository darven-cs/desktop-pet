---
name: Agent Loop + 事件驱动决策
status: FROZEN
created: 2026-06-07
owner: darven
related:
  - ../../docs/specs/02-llm-decision.md
---

# 03 · Agent Loop + 事件驱动决策

**一句话**：把 02 的定时器单次 LLM 调用升级为事件驱动的 Agent Loop——宠物能收集多种事件（定时心跳 / 用户交互 / 动画完成 / 窗口焦点），在 budget 控制下多轮 observe→think→act，让决策从"30s 盲猜"变成"根据环境持续推理"。

> 本 spec 依赖 02 的所有基础设施（LLM 客户端 / 工具注册表 / DecisionContext / 记忆系统）。核心改动在前端事件采集 + 后端 agent loop 循环，02 的 `decide_next_state` 保留但退化为 fallback。

---

## 1. 功能清单

| 编号 | 行为 |
|---|---|
| **F1** | 前端事件采集 composable：收集 `TimerTick` / `UserInteraction` / `AnimationCompleted` / `WindowFocusChanged` 四种事件到内存队列 |
| **F2** | 事件触发策略：TimerTick 每次都触发决策；其他事件 debounce 2s 后触发（同一窗口内的事件合并为一次请求） |
| **F3** | 后端 Agent Loop：收到事件批次后，在 budget 内循环 `observe → LLM think → tool act → observe result`，直到 terminal 工具被调用或 budget 耗尽 |
| **F4** | Budget 控制器：单次 loop 最多 5 步（LLM 调用）；每分钟最多 6 次 API 调用；两次调用间隔 ≥ 5s |
| **F5** | 新工具 `wait`：LLM 可调用 `wait(duration_seconds)` 表示"我想安静 N 秒"，返回 `Decision::Wait`，前端收到后暂停 ticker 并设置 setTimeout 恢复 |
| **F6** | `Decision` 枚举新增 `Wait` 变体：`Wait { duration_ms: u32 }`，前端收到后停止 ticker，在指定时间后恢复 |
| **F7** | 事件摘要注入 prompt：将事件批次格式化为自然语言摘要，注入 user prompt 的"最近事件"段落，让 LLM 知道发生了什么 |
| **F8** | 向后兼容：`decide_next_state` 保留不变（02 的接口），新增 `agent_decide` 为新入口。旧入口在 budget 耗尽时作为降级通道 |

---

## 2. 业务规则

| 编号 | 规则 |
|---|---|
| **R1** | 事件采集在前端完成，后端不维护事件队列。每次 IPC 调用传入完整的事件批次，后端无状态 |
| **R2** | Agent Loop 的每次迭代（step）= 一次 LLM API 调用。非 terminal 工具的执行不计为 step，但 terminal 工具的调用计入 |
| **R3** | Budget 按滑动窗口计算：以当前时刻为基准往前看 60s，统计 API 调用次数 |
| **R4** | Budget 耗尽时，agent loop 立即退出，返回 `Decision::Stay`。日志 `[PetAgent] budget exhausted` |
| **R5** | `AnimationCompleted` 事件仅在 `loop: once` 的动画播完时触发。`loop: infinite` 的动画不产生此事件 |
| **R6** | 同一事件不能连续触发两次 agent loop。`inFlight` 标志位（02 已有）继续生效，防止并发 |
| **R7** | `UserInteraction` 事件类型细分：`click`（点击宠物）、`drag_end`（拖拽结束）、`double_click`（双击）。每次交互只产生一条事件 |
| **R8** | `WindowFocusChanged` 事件仅在焦点状态**变化**时产生（blur→focus 或 focus→blur），不重复上报 |
| **R9** | `Wait` 决策的最短时长 = 10s，最长 = 600s（10 分钟）。超出范围 clamp 到边界值 |
| **R10** | Agent Loop 中，如果 LLM 连续 2 次返回既不是 tool_call 也不是合法 JSON 的响应，立即退出并 fallback Stay |
| **R11** | 系统提示词更新：在 02 的基础上新增"你可以调用 wait 工具来让自己安静一会儿"和"你会收到最近发生的事件列表，基于这些事件做决策" |
| **R12** | `UserInteraction` 事件在 `DecisionContext.lastInteractionAt` 之外**额外**传入，两者不冲突：前者是事件描述（"用户点击了我"），后者是时间戳用于计算距离上次互动多久 |

---

## 3. 接口契约

### 3.1 新增 Tauri Command：`agent_decide`

#### `agent_decide(events: Vec<PetEvent>, context: DecisionContext) -> Result<AgentResult, AppError>`

- **permission**：需在 `capabilities/default.json` 新增 `"allow-agent-decide"`
- **入参**：

  | 字段 | 类型 | 必填 | 校验 |
  |---|---|---|---|
  | `events` | `Vec<PetEvent>` | 是 | 长度 ≥ 1 |
  | `context` | `DecisionContext` | 是 | 复用 02 的结构 |

- **出参 ok**：`AgentResult`
  ```rust
  struct AgentResult {
      decision: Decision,         // 最终决策
      steps_used: u32,            // 本次 loop 消耗的步数
      tool_calls_made: Vec<String>, // 调用过的工具名列表（用于日志/调试）
  }
  ```
- **出参 err**：`AppError`（`E_INTERNAL`）
- **副作用**：
  - 记录决策到 MemoryManager
  - 更新 budget 计数器

### 3.2 前端 Composable：`usePetEvents`

#### `usePetEvents()` — 新增

- **职责**：采集前端事件，debounce 后触发 agent 决策
- **暴露**：
  ```ts
  {
    // 事件入队
    pushEvent: (event: PetEvent) => void,

    // 手动触发 flush（测试用）
    flush: () => Promise<void>,

    // 当前队列长度（只读）
    queueLength: Readonly<Ref<number>>,
  }
  ```
- **行为**：
  - `pushEvent()` 把事件推入内部数组
  - 如果事件类型是 `TimerTick` → 立即 flush
  - 如果事件类型是非 TimerTick → 启动 2s debounce timer，到期后 flush
  - `flush()` = 清空队列 + 构建 DecisionContext + `invoke("agent_decide", { events, context })`
  - flush 期间 `inFlight=true`，新事件入队但不触发新 flush

### 3.3 前端事件绑定点

| 事件类型 | 触发位置 | 代码文件 |
|---|---|---|
| `TimerTick` | `useAnimationStateMachine.ts` 的 `setInterval` 回调 | `useAnimationStateMachine.ts` |
| `UserInteraction` (`click`) | `App.vue` 中 pet 容器的 `@click` | `App.vue` |
| `UserInteraction` (`drag_end`) | `App.vue` 中 pet 容器的 `@mouseup`（拖拽结束时） | `App.vue` |
| `AnimationCompleted` | CSS `animationend` 事件 + 检查 `loop: once` | `App.vue` 或 sprite 组件 |
| `WindowFocusChanged` | `window.addEventListener('focus'/'blur')` | `App.vue` |

### 3.4 Rust 内部模块：`agent`

```
// src-tauri/src/agent/mod.rs

struct AgentBudget {
    max_steps: u32,              // 单次 loop 上限，默认 5
    max_calls_per_minute: u32,   // 每分钟上限，默认 6
    min_call_interval_ms: u64,   // 两次调用最小间隔，默认 5000
}

struct AgentLoopResult {
    decision: Decision,
    steps_used: u32,
    tool_calls_made: Vec<String>,
}

async fn run_agent_loop(
    config: &LlmStaticConfig,
    ctx: &DecisionContext,
    events: &[PetEvent],
    tools: &ToolRegistry,
    budget: &mut AgentBudget,
) -> Result<AgentLoopResult, LlmError>
```

### 3.5 工具注册表变更

`tools.rs` 新增一个工具：

#### `wait` 工具

| 属性 | 值 |
|---|---|
| name | `wait` |
| description | "让自己安静一段时间，不做任何动作。用于觉得该休息了、深夜不想打扰用户等场景" |
| parameters | `{ duration_seconds: { type: number, description: "想安静的秒数(10-600)", minimum: 10, maximum: 600 }, reason: { type: string, description: "安静的原因(简短)" } }` |
| required | `["duration_seconds"]` |
| is_terminal | `true` |
| handler | 返回 `Decision::Wait { duration_ms: clamp(duration_seconds * 1000, 10000, 600000), reason }` |

### 3.6 Decision 枚举扩展

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Decision {
    Stay,
    Switch { to: String, #[serde(skip_serializing_if = "Option::is_none")] reason: Option<String> },
    #[serde(rename_all = "camelCase")]
    Speak { message: String, #[serde(skip_serializing_if = "Option::is_none")] animation: Option<String> },
    EnterIdle,
    ExitIdle,
    Wait { duration_ms: u32, #[serde(skip_serializing_if = "Option::is_none")] reason: Option<String> },  // ← 新增
}
```

TS 端同步扩展：
```ts
export type Decision =
  | { action: "stay" }
  | { action: "switch"; to: string; reason?: string }
  | { action: "speak"; message: string; animation?: string }
  | { action: "enter_idle" }
  | { action: "exit_idle" }
  | { action: "wait"; durationMs: number; reason?: string };  // ← 新增
```

---

## 4. 数据结构

### 4.1 PetEvent（新增）

```rust
// src-tauri/src/agent/events.rs

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PetEvent {
    TimerTick {
        timestamp: u64,  // epoch ms
    },
    UserInteraction {
        interaction: String,  // "click" | "drag_end" | "double_click"
        timestamp: u64,
    },
    AnimationCompleted {
        animation_id: String,
        timestamp: u64,
    },
    WindowFocusChanged {
        focused: bool,
        timestamp: u64,
    },
}
```

TS 端镜像：
```ts
export type PetEvent =
  | { type: "timer_tick"; timestamp: number }
  | { type: "user_interaction"; interaction: "click" | "drag_end" | "double_click"; timestamp: number }
  | { type: "animation_completed"; animationId: string; timestamp: number }
  | { type: "window_focus_changed"; focused: boolean; timestamp: number };
```

### 4.2 AgentResult（新增）

```rust
// src-tauri/src/agent/mod.rs

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AgentResult {
    pub decision: Decision,
    pub steps_used: u32,
    pub tool_calls_made: Vec<String>,
}
```

### 4.3 AgentBudget（内部，不跨 IPC）

```rust
// src-tauri/src/agent/budget.rs

pub struct AgentBudget {
    pub max_steps: u32,              // 单次 loop，默认 5
    pub max_calls_per_minute: u32,   // 滑动窗口，默认 6
    pub min_call_interval_ms: u64,   // 两次间隔，默认 5000
    pub calls_timestamps: Vec<u64>,  // 最近 60s 内的调用时间戳
}
```

### 4.4 Decision 扩展（修改 02 的类型）

见 §3.6。

---

## 5. 异常与边界

| 场景 | 期望行为 | 错误码/恢复方式 |
|---|---|---|
| 事件队列为空时不小心调了 `agent_decide` | 返回 `AppError(E_INVALID_CONTEXT, "events must not be empty")` | `E_INVALID_CONTEXT` |
| 单次 loop 中 LLM 连续 2 次返回非法响应 | 立即退出 loop，返回 `Stay`，日志 `[PetAgent] consecutive parse failures` | fallback Stay |
| Budget 耗尽（6 次/分钟已达上限） | 直接返回 `Stay`，日志 `[PetAgent] budget exhausted (calls_per_minute)` | fallback Stay |
| Budget 耗尽（单次 5 步已满） | 返回最后一次合法 Decision（如果有），否则 `Stay` | 最后有效结果或 fallback |
| 两次调用间隔 < 5s | `AgentBudget::check()` 返回 false，跳过本次请求，日志 `[PetAgent] rate limited` | 不调 API，返回 Stay |
| `wait` 工具传入 duration < 10 或 > 600 | handler 内 clamp 到 [10000, 600000] ms | 自动修正 |
| 前端 flush 期间 `inFlight=true`，又有新事件入队 | 事件入队但不触发新 flush，等当前 flush 完成后由下次 TimerTick 处理 | 队列积压，下次 tick 处理 |
| 窗口 focus/blur 在 2s debounce 内来回切换 | 只产生最后一条 `WindowFocusChanged` 事件，之前的状态变化丢弃 | debounce 合并 |
| `AnimationCompleted` 但动画 ID 不在 registry 中 | 仍然作为事件传入后端，但 agent 的 prompt 里只显示 ID，不校验 | 不校验 |
| LLM 在 agent loop 中调用了 `wait` 但同时调了其他工具 | terminal 工具优先：`wait` 是 terminal，直接返回，不处理后续工具调用 | terminal 优先 |
| Agent loop 期间 ticker 又触发了 TimerTick | `inFlight` 阻断，跳过本次 tick，日志 `[PetTicker] skipped, inFlight` | 02 已有的行为不变 |

---

## 6. 系统提示词变更

在 02 的 `DEFAULT_SYSTEM_PROMPT` 基础上：

### 6.1 新增段落

```
## 事件

你会收到最近发生的事件列表（放在"最近事件"段落中），基于这些事件做决策：
- timer_tick: 定时心跳，表示时间流逝
- user_interaction: 用户和你互动了（点击、拖拽、双击）
- animation_completed: 你的某个动画播完了
- window_focus_changed: 用户切换到了别的窗口或回来了

## wait 工具

如果你想安静一会儿（比如深夜、用户在忙、你自己想休息），调用 wait 工具而不是 switch_animation。
这样你会暂停一段时间，不打扰用户。
```

### 6.2 决策规则更新

在"决策规则"段落末尾追加：
```
9. 如果最近事件显示用户刚回来（window_focus_changed: focused=true），考虑主动说话或做欢迎动作
10. 如果最近事件显示用户离开了（window_focus_changed: focused=false），考虑 wait 或安静的动作
11. 深夜且用户不在时，优先使用 wait 工具
```

### 6.3 User Prompt 变更

`build_user_prompt()` 在现有内容之后追加事件摘要：

```
最近事件：
- [14:35:22] timer_tick
- [14:35:23] user_interaction(click)
- [14:35:25] animation_completed(think)

决定下一步动作。
```

如果事件批次为空（不应发生，见 §5），显示"（无新事件）"。

---

## 7. 前端 Wait 处理

`applyDecision()` 新增 `wait` 分支：

```ts
case "wait": {
  const durationMs = decision.durationMs;
  lastTickerReason.value = decision.reason ?? `等待 ${Math.round(durationMs / 1000)}s`;
  petSettings?.onDecision?.(decision.reason ?? null);
  stopTicker();  // 暂停定时器
  // durationMs 后恢复
  setTimeout(() => {
    startTicker();
    pushEvent({ type: "timer_tick", timestamp: Date.now() }); // 立即触发一次决策
    console.log(`[PetAgent] wait ended after ${durationMs}ms, resuming ticker`);
  }, durationMs);
  break;
}
```

---

## 8. 文件变更清单

### 新增文件

| 文件 | 职责 |
|---|---|
| `src-tauri/src/agent/mod.rs` | Agent loop 核心逻辑：`run_agent_loop()` |
| `src-tauri/src/agent/events.rs` | `PetEvent` 枚举定义 |
| `src-tauri/src/agent/budget.rs` | `AgentBudget` 结构 + 检查/消耗逻辑 |
| `src/composables/usePetEvents.ts` | 前端事件采集 composable |

### 修改文件

| 文件 | 变更 |
|---|---|
| `src-tauri/src/lib.rs` | 新增 `agent_decide` command 注册；`mod agent;` |
| `src-tauri/src/tools.rs` | 新增 `wait` 工具注册 |
| `src-tauri/src/types.rs` | `Decision` 枚举新增 `Wait` 变体 |
| `src-tauri/src/llm.rs` | `send_chat_request()` 重构：移除硬编码 2 rounds，改为接受 step limit 参数（由 agent loop 控制） |
| `src/composables/useAnimationStateMachine.ts` | ticker 改为 pushEvent(TimerTick) 而非直接调 decider；`applyDecision` 新增 wait 分支；`startTicker`/`stopTicker` 暴露 |
| `src/types/pet.ts` | 新增 `PetEvent` 类型、`Decision` 新增 wait 变体 |
| `src/App.vue` | 绑定 click/drag/animationend/focus/blur 事件到 pushEvent |

### 不变的文件

| 文件 | 原因 |
|---|---|
| `src-tauri/src/chat.rs` | 聊天流程暂不走 agent loop（独立演进） |
| `src-tauri/src/decider.rs` | 保留为 fallback 通道，02 的接口不变 |
| `src-tauri/src/memory/*` | 记忆系统接口不变 |

---

## 9. 验收用例

### F1 事件采集

- [ ] **AC-F1.1** [代码评审] `src/composables/usePetEvents.ts` 存在，导出 `pushEvent` / `flush` / `queueLength`
- [ ] **AC-F1.2** [代码评审] `PetEvent` 类型覆盖 `timer_tick` / `user_interaction` / `animation_completed` / `window_focus_changed` 四种
- [ ] **AC-F1.3** [代码评审] Rust 端 `PetEvent` 枚举与 TS 端字段一一对应（serde `rename_all` 一致）

### F2 触发策略

- [ ] **AC-F2.1** [日志] TimerTick 触发时，`inFlight=false`，日志 `[PetCmd] agent_decide called with N events`（N ≥ 1）
- [ ] **AC-F2.2** [日志] 非定时器事件（如 click）触发后，2s 内没有第二次 `agent_decide`（debounce 生效）
- [ ] **AC-F2.3** [日志] 2s 内连续 3 次 click，只产生 1 次 `agent_decide` 调用
- [ ] **AC-F2.4** [日志] `inFlight=true` 时新事件入队但不触发 `agent_decide`

### F3 Agent Loop

- [ ] **AC-F3.1** [日志] Agent Loop 收到 `get_current_time` 工具调用后，feed result 回 messages，继续下一轮（日志 `[PetAgent] step N: tool_call get_current_time`）
- [ ] **AC-F3.2** [日志] Agent Loop 收到 `switch_animation` terminal 工具调用后，立即返回（不再调 LLM），日志 `[PetAgent] terminal: switch_animation`
- [ ] **AC-F3.3** [日志] Agent Loop 收到 `speak_to_user` terminal 工具调用后，同 AC-F3.2
- [ ] **AC-F3.4** [日志] 一次 loop 中最多 5 条 `[PetAgent] step N` 日志

### F4 Budget 控制

- [ ] **AC-F4.1** [日志] 连续触发 6 次 agent loop（1 分钟内），第 7 次日志 `[PetAgent] budget exhausted (calls_per_minute)`
- [ ] **AC-F4.2** [日志] 两次调用间隔 < 5s 时，日志 `[PetAgent] rate limited`
- [ ] **AC-F4.3** [代码评审] `AgentBudget` 有 `check()` 和 `consume()` 方法，`check()` 在 `consume()` 之前调用

### F5 Wait 工具

- [ ] **AC-F5.1** [代码评审] `tools.rs` 注册了 `wait` 工具，`is_terminal: true`
- [ ] **AC-F5.2** [日志] LLM 调用 `wait(60)` → 日志 `[PetAgent] terminal: wait(60s)` → 返回 `Decision::Wait { duration_ms: 60000 }`
- [ ] **AC-F5.3** [手动] Wait 决策后，宠物不产生新动作（ticker 停止），在指定时间后自动恢复并产生新决策
- [ ] **AC-F5.4** [代码评审] `wait` handler 对 duration_seconds 做 clamp [10, 600]

### F6 Decision::Wait

- [ ] **AC-F6.1** [代码评审] Rust `Decision` 枚举含 `Wait { duration_ms: u32, reason: Option<String> }`
- [ ] **AC-F6.2** [代码评审] TS `Decision` type 含 `| { action: "wait"; durationMs: number; reason?: string }`
- [ ] **AC-F6.3** [手动] 收到 Wait 后，状态面板显示"等待中 (60s)"
- [ ] **AC-F6.4** [手动] Wait 期间用户点击宠物 → 事件入队但不触发（inFlight/wait 优先级待讨论，暂定为：wait 期间 click 立即恢复 ticker）

### F7 事件摘要注入

- [ ] **AC-F7.1** [代码评审] `build_user_prompt()` 追加"最近事件"段落，格式化为 `- [HH:MM:SS] event_type(args)`
- [ ] **AC-F7.2** [日志] `[PetLLM] request` 的 user message 中可见 `最近事件：` 段落

### F8 向后兼容

- [ ] **AC-F8.1** [代码评审] `decide_next_state` command 仍在 `invoke_handler` 中注册
- [ ] **AC-F8.2** [手动] `agent_decide` 不可用时（如 budget 耗尽），`decide_next_state` 仍可调用
- [ ] **AC-F8.3** [代码评审] `llm.rs` 的 `send_chat_request()` 接受 `max_steps` 参数（默认 2），02 的调用方传入 2 保持不变

### R 级验收

- [ ] **AC-R1** [代码评审] 事件队列在前端内存中（`usePetEvents` 的局部变量），不在 localStorage
- [ ] **AC-R2** [代码评审] 后端 `agent_decide` 每次调用传入完整事件批次，后端无持久化队列
- [ ] **AC-R6** [日志] `inFlight` 日志在 agent loop 期间可见：`[PetTicker] skipped, inFlight`
- [ ] **AC-R9** [代码评审] `wait` handler 使用 `clamp(duration_seconds * 1000, 10000, 600000)`
- [ ] **AC-R10** [日志] 模拟 LLM 返回 2 次非法响应 → 日志 `[PetAgent] consecutive parse failures` → fallback Stay
- [ ] **AC-R11** [代码评审] 系统提示词包含"事件"和"wait 工具"段落

### 异常验收

- [ ] **AC-E1** [日志] 传空 events 到 `agent_decide` → 返回 `AppError(E_INVALID_CONTEXT)`
- [ ] **AC-E2** [日志] Budget 耗尽时 agent loop 不发出 HTTP 请求（`[PetLLM] request` 不出现）
- [ ] **AC-E3** [手动] 窗口在 2s 内 focus→blur→focus，最终事件只含一条 `window_focus_changed(focused=true)`
- [ ] **AC-E4** [手动] Agent loop 期间 LLM 超时 → 日志 `[PetLLM] timeout` → fallback Stay → 宠物不崩溃
- [ ] **AC-E5** [手动] 连续 10 次 budget 耗尽 → 每次都返回 Stay，应用不崩，60s 后 budget 恢复

---

## 10. Agent Loop 伪代码

```
fn run_agent_loop(config, ctx, events, tools, budget):
    if events.is_empty(): return Err(E_INVALID_CONTEXT)
    if !budget.check(): return Ok(AgentResult { decision: Stay, reason: "rate_limited" })

    system_prompt = build_system_prompt(config, ctx.personality, ctx.pet_name)
    user_prompt = build_user_prompt(ctx) + format_events(events)

    messages = [
        { role: "system", content: system_prompt },
        { role: "user",   content: user_prompt },
    ]

    steps = 0
    consecutive_failures = 0
    tool_calls_made = []

    while steps < budget.max_steps:
        budget.consume()
        steps += 1

        response = do_http_request(messages, tools.schema())

        if response.has_tool_calls:
            consecutive_failures = 0
            for tc in response.tool_calls:
                tool_calls_made.push(tc.name)
                result = tools.execute(tc.name, tc.args)

                if tools.is_terminal(tc.name):
                    decision = parse_decision(result)
                    return Ok(AgentResult { decision, steps_used: steps, tool_calls_made })

                // non-terminal: feed result back
                messages.push({ role: "tool", content: result })
        else:
            // no tool calls, try parse as direct Decision
            match parse_decision_from_message(response.message):
                Ok(decision) => return Ok(AgentResult { decision, steps_used: steps, tool_calls_made })
                Err(_) => {
                    consecutive_failures += 1
                    if consecutive_failures >= 2:
                        return Ok(AgentResult { decision: Stay, steps_used: steps, tool_calls_made })
                }

    // budget exhausted
    return Ok(AgentResult { decision: Stay, steps_used: steps, tool_calls_made })
```

---

## 11. 变更历史

- 2026-06-07 v1：建 spec，status: DRAFT。核心：事件采集 + agent loop + budget + wait 工具
