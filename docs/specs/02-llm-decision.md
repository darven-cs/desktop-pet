---
name: LLM 状态决策（OpenAI 兼容 API + 宠物管理中心 + 可配置人格）
status: DRAFT
created: 2026-06-07
owner: darven
related:
  - ../../docs/specs/01-pet-interaction-layer.md
  - ../../memory/modules/project_ai_pet_vision.md
---

# 02 · LLM 状态决策

**一句话**：把 01 的占位 decider 替换为 OpenAI 兼容 LLM API 调用，让宠物根据人格设定 + 上下文自主决策动画；同时把右键菜单从「动画选择器」转型为「宠物管理中心」。

> 本 spec 依赖 01 的所有基础设施（状态机 + ticker + decider 接口 + Rust IPC）。LLM 调用在 Rust 端完成，前端无感知。

---

## 1. 功能清单

| 编号 | 行为 |
|---|---|
| **F1** | Rust 端 OpenAI 兼容 HTTP 客户端：从 `.env` 加载配置，POST 到 `/v1/chat/completions`，解析 JSON 响应为 `Decision` |
| **F2** | LLM 决策替换占位：`decide_next_state` 从永远 `Stay` → 调 LLM 决定下一步动画 |
| **F3** | 可配置宠物人格：`.env` 提供默认系统提示词，可通过 `LLM_SYSTEM_PROMPT` 覆盖；提示词描述宠物性格 + 可用动画 + 决策规则 |
| **F4** | 可配置 LLM 参数：endpoint / api_key / model / temperature / max_tokens / enabled，均从 `.env` 加载 |
| **F5** | 右键菜单重构：从「动画 ID 列表 + 退出」→「宠物状态 + 宠物设定 + 宠物记忆 + 手动切动画（子菜单）+ 退出」 |
| **F6** | 宠物状态面板（overlay）：显示当前动画、阶段、上次 LLM 决策理由、LLM 连接状态、ticker 间隔 |
| **F7** | 宠物设定面板（overlay）：LLM 开关 toggle、宠物人格文本编辑、ticker 间隔输入、模型选择；保存后持久化到 localStorage |
| **F8** | 宠物记忆面板（overlay 占位）：显示「即将推出」，为 06 留入口 |
| **F9** | 错误容错：任何 LLM 错误（网络、解析、超时）→ fallback `Decision::Stay`，日志 `[PetLLM]`，宠物不崩溃 |
| **F10** | `DecisionContext` 扩展：新增 `time_of_day`、`recent_history` 可选字段，Rust + TS 同步 |

---

## 2. 业务规则

| 编号 | 规则 |
|---|---|
| **R1** | LLM 调用是**异步非阻塞**的：ticker 内部 `await decider(ctx)` 期间 `inFlight=true`，不会并发调用 |
| **R2** | LLM 超时 = max(10s, ticker_interval_ms × 0.8)，防止请求堆积。超时视为错误，fallback `Stay` |
| **R3** | API key 必须存在于 `.env`，不可硬编码在源码中。`LLM_API_KEY` 为空 → `LLM_ENABLED` 自动视为 `false` |
| **R4** | `LLM_ENABLED=false` 时行为与 01 完全一致（占位 `Stay`），向后兼容 |
| **R5** | 右键菜单不再列出所有动画 ID 作为平铺项；动画 ID 收进「手动切动画」子菜单 |
| **R6** | overlay 面板定位规则同 01 菜单：在宠物窗口右下弹出，自动防出屏（留 ≥ 8px 边距） |
| **R7** | 同一时刻最多一个 overlay 打开。打开新 overlay → 先关闭当前 overlay。右键菜单与 overlay 互斥（打开 overlay 时关闭菜单） |
| **R8** | 宠物设定变更立即生效（ticker 间隔变更 → 重启 ticker；人格变更 → 下次 LLM 请求用新提示词；LLM 开关 off → 下一次 tick 起走 Stay stub） |
| **R9** | `recent_history` 最多保存 5 条动画 ID（FIFO），在状态机内部维护，不在 `AnimationState` 结构里 |
| **R10** | LLM 返回的 `to` 必须是当前 registry 中存在的动画 ID；不存在则忽略（日志 warn），fallback `Stay` |

---

## 3. 接口契约

### 3.1 Rust 端（Tauri command，续用 01 的 + 新增）

#### `decide_next_state(context: DecisionContext) -> Result<Decision, AppError>`（改造）

- **permission**：已有（01 已声明）
- **行为变化**：从同步函数 → async，内部调 `llm::send_chat_request()`
- **稳定保证**：接口签名不变，TS 端调用方无改动

#### `get_llm_config() -> Result<LlmConfig, AppError>`（新增）

- **permission**：需在 `capabilities/default.json` 声明
- **入参**：无
- **出参 ok**：`LlmConfig`（当前生效的 LLM 配置，不含 api_key）
- **出参 err**：`AppError`(`InternalError`)
- **用途**：前端设定面板读取当前配置来展示默认值

#### `update_llm_config(config: LlmConfigUpdate) -> Result<(), AppError>`（新增）

- **permission**：需在 `capabilities/default.json` 声明
- **入参**：`LlmConfigUpdate`（部分字段可选，只更新提供的字段）
- **出参 ok**：`()`
- **出参 err**：`AppError`(`InvalidContext` | `InternalError`)
- **副作用**：写入环境变量（运行时生效，不持久化到 .env 文件——Tauri 运行时环境变量只读，实际由前端 localStorage 持久化，Rust 端每次 tick 从 DecisionContext 或内部状态读取）

> **注意**：由于 Tauri 运行时无法动态写 `.env`，设定的实际持久化在前端 `localStorage`。Rust 端不从 `.env` 之外的地方读设定——所以 `LLM_ENABLED` / `LLM_SYSTEM_PROMPT` 这类**可运行时修改**的设定，通过 `DecisionContext` 或一个独立的 settings 字段传给 Rust。见 §3.2。

### 3.2 DecisionContext 扩展传给 LLM 可运行时修改的设定

`DecisionContext` 在 02 中新增两个用途：
1. 携带 LLM 决策所需上下文（时间、历史）——F10
2. 携带**可运行时修改**的宠物设定（人格文本、开关）——这些设定不由 `.env` 静态控制，而是由前端 localStorage + Rust 端每次 tick 读取

```rust
pub struct DecisionContext {
    // 01 原有（不变）
    pub current_state: AnimationState,
    pub last_interaction_at: u64,
    pub ticker_interval_ms: u32,
    // 02 新增上下文
    pub time_of_day: Option<String>,
    pub recent_history: Option<Vec<String>>,
    // 02 可运行时修改的设定（从前端传过来，每次 tick 都可能变）
    pub llm_enabled: Option<bool>,
    pub pet_personality: Option<String>,
}
```

TS 端 `tickerTick()` 填充：
- `timeOfDay`: `new Date().toLocaleTimeString('zh-CN', {hour:'2-digit', minute:'2-digit'})`
- `recentHistory`: 动画 ID ring buffer（FIFO，最多 5 条）
- `llmEnabled` / `petPersonality`: 从 `usePetSettings` composable 读取

### 3.3 前端 composable

#### `usePetSettings()`（新增）

- **职责**：管理可运行时修改的宠物设定，持久化到 localStorage
- **暴露**：
  ```ts
  {
    llmEnabled: Ref<boolean>,           // 默认 true
    petPersonality: Ref<string>,        // 默认 ""
    tickerIntervalMs: Ref<number>,      // 默认 30000
    model: Ref<string>,                 // 默认 "gpt-4o-mini"
    lastDecisionReason: Ref<string | null>,
    updateSettings(partial): void,
  }
  ```
- **行为**：
  - 启动时从 `localStorage['pet-settings']` 恢复，无则用默认值
  - `updateSettings()` 合并写入 ref + 写回 localStorage
  - ticker 间隔变更 → 通知 `useAnimationStateMachine` 更新 ticker

### 3.4 Rust 内部模块：`llm`

```
// src-tauri/src/llm.rs（不是 Tauri command，是内部模块）

fn load_static_config() -> LlmStaticConfig  // .env 中不可运行时修改的项
fn build_system_prompt(personality: &str) -> String
fn build_user_prompt(ctx: &DecisionContext) -> String
async fn send_chat_request(static_cfg, ctx) -> Result<Decision, LlmError>
fn parse_response(body: &str) -> Result<Decision, LlmError>
```

`LlmStaticConfig`（从 `.env` 读，运行时不变）：
```rust
struct LlmStaticConfig {
    endpoint: String,    // LLM_API_ENDPOINT
    api_key: String,     // LLM_API_KEY
    model: String,       // LLM_MODEL
    temperature: f32,    // LLM_TEMPERATURE
    max_tokens: u32,     // LLM_MAX_TOKENS
}
```

### 3.5 错误码扩展

01 的 `AppErrorCode` 不变。新增 `LlmError`（Rust 内部错误类型，不暴露给 TS）：

```rust
enum LlmError {
    Http(u16, String),     // HTTP 非 200
    Network(String),       // 请求发不出去
    Parse(String),         // 响应 JSON 解析失败
    Timeout,               // 超时
    Disabled,              // LLM 未启用
}
```

---

## 4. 数据结构

> Rust 端定义并 serde。TS 端手动镜像（R7）。

### 4.1 新增类型

```rust
// src-tauri/src/types.rs

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    pub endpoint: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfigUpdate {
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}
```

### 4.2 DecisionContext（扩展后）

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DecisionContext {
    pub current_state: AnimationState,
    pub last_interaction_at: u64,
    pub ticker_interval_ms: u32,
    pub time_of_day: Option<String>,
    pub recent_history: Option<Vec<String>>,
    pub llm_enabled: Option<bool>,
    pub pet_personality: Option<String>,
}
```

TS 端同步扩展：

```ts
export interface DecisionContext {
  currentState: AnimationState;
  lastInteractionAt: number;
  tickerIntervalMs: number;
  timeOfDay?: string;
  recentHistory?: string[];
  llmEnabled?: boolean;
  petPersonality?: string;
}
```

---

## 5. 提示词设计

### 5.1 默认系统提示词

```
你是一只可爱的桌面宠物。你住在用户的桌面上，通过切换不同的动画来表达自己。

你的性格：好奇心旺盛、偶尔偷懒、喜欢吸引用户注意。

可用的动画：
- touch_nose: 摸鼻子（可爱动作）
- think: 思考（发呆/想事情）
- poop: 拉粑粑（恶搞/搞笑）

决策规则：
1. 不要连续3次以上播放同一动画
2. 如果用户很久没互动（>5分钟），做点有趣的动作吸引注意
3. 早上和下午多活跃，深夜减少动作
4. 偶尔（~20%概率）在动画之间插入 think 发呆
5. 你的决策应该让用户感到这只宠物有性格、不可预测但又可爱

你必须只返回一个 JSON 对象，格式如下：
{"action":"switch","to":"<动画ID>","reason":"<简短理由>"}
或者 {"action":"stay"}

不要返回任何其他文字。
```

### 5.2 用户提示词（每 tick 构建）

```
当前状态：
- 动画：{current}、阶段：{phase}、已循环：{iteration} 次
- 最近动画：{recent_history}
- 距上次互动：{seconds} 秒
- 当前时间：{time_of_day}

决定下一步动作。
```

### 5.3 自定义人格

用户可通过宠物设定面板修改人格文本（保存在 localStorage 的 `pet-personality` 字段）。当 `petPersonality` 非空时，**替换**默认系统提示词中的「你的性格：…」段落。

---

## 6. 日志约定

延续 01 的四类前缀，02 新增：

| 前缀 | 用途 | 来源 |
|---|---|---|
| `[PetState]` | 状态变更（01） | 前端 |
| `[PetCmd]` | Rust command 调用（01） | Rust |
| `[PetError]` | Rust command 错误（01） | Rust |
| `[PetTicker]` | ticker 跳过/间隔变化（01） | 前端 |
| **`[PetLLM]`** | **LLM 请求/响应/错误（02 新增）** | **Rust** |

`[PetLLM]` 详细格式：

| 事件 | 格式 |
|---|---|
| 请求发送 | `[PetLLM] request to {endpoint}, model={model}, tokens={max_tokens}` |
| 响应成功 | `[PetLLM] response: {decision_json}` |
| 响应解析失败 | `[PetLLM] parse error: {reason}, raw={truncated_raw}` |
| HTTP 错误 | `[PetLLM] http error {status}: {body_truncated}` |
| 网络错误 | `[PetLLM] network error: {err}` |
| 超时 | `[PetLLM] timeout after {ms}ms` |
| 禁用 | `[PetLLM] disabled, returning Stay` |
| 动画 ID 无效 | `[PetLLM] invalid animation id: {id}, falling back to Stay` |
| 回退 | `[PetLLM] falling back to Stay` |

---

## 7. 验收用例

### F1 OpenAI 客户端

- [ ] **AC-F1.1** [日志] `.env` 配置正确的 API key + endpoint，启动后 ticker 触发生成 `[PetLLM] request` 日志
- [ ] **AC-F1.2** [日志] LLM 返回有效 JSON 后，grep `[PetLLM] response:` 可见 Decision JSON
- [ ] **AC-F1.3** [代码评审] `src-tauri/src/llm.rs` 独立模块，不依赖 tauri 运行时（可单独测试）

### F2 LLM 决策替换占位

- [ ] **AC-F2.1** [日志] `LLM_ENABLED=true` 时，grep `[PetLLM] request` 存在
- [ ] **AC-F2.2** [日志] `LLM_ENABLED=false` 时，grep `[PetLLM] disabled` 存在，且 `[PetLLM] request` 不存在
- [ ] **AC-F2.3** [手动] LLM 返回 `{"action":"switch","to":"think","reason":"test"}` → 宠物 500ms 内切到 think 动画

### F3 可配置宠物人格

- [ ] **AC-F3.1** [代码评审] 默认系统提示词硬编码在 `llm.rs`，描述宠物性格 + 动画 + 规则
- [ ] **AC-F3.2** [手动] `.env` 设 `LLM_SYSTEM_PROMPT=一只愤怒的猫`，启动后 ticker 触发 → `[PetLLM] request` 的请求体 system message 为「一只愤怒的猫」

### F4 可配置 LLM 参数

- [ ] **AC-F4.1** [代码评审] `LLM_API_ENDPOINT` / `LLM_API_KEY` / `LLM_MODEL` / `LLM_TEMPERATURE` / `LLM_MAX_TOKENS` / `LLM_ENABLED` 均从 `std::env::var` 读
- [ ] **AC-F4.2** [日志] 各 env 未设置时使用硬编码默认值，启动不报错

### F5 右键菜单重构

- [ ] **AC-F5.1** [手动] 右键宠物 → 菜单显示「宠物状态」「宠物设定」「宠物记忆」「手动切动画 ▶」「退出」
- [ ] **AC-F5.2** [手动] 菜单中不显示平铺的动画 ID 列表（touch_nose / think / poop）
- [ ] **AC-F5.3** [手动] 悬停「手动切动画」→ 展开子菜单列出可用动画 ID
- [ ] **AC-F5.4** [手动] 点击子菜单中的动画 → 宠物切换到该动画，菜单关闭
- [ ] **AC-F5.5** [手动] 点击「退出」→ 应用退出

### F6 宠物状态面板

- [ ] **AC-F6.1** [手动] 右键 → 宠物状态 → 弹出 overlay，显示当前动画名 + 阶段
- [ ] **AC-F6.2** [手动] LLM 上次返回 switch 带 reason 时，状态面板显示该 reason
- [ ] **AC-F6.3** [手动] 点击 overlay 外部 / Esc → 关闭
- [ ] **AC-F6.4** [手动] 面板不出屏（贴边时反向偏移）

### F7 宠物设定面板

- [ ] **AC-F7.1** [手动] 右键 → 宠物设定 → overlay 显示 LLM 开关 toggle（默认开）
- [ ] **AC-F7.2** [手动] 关闭 LLM 开关 → 保存后，下一次 ticker 日志显示 `[PetLLM] disabled`
- [ ] **AC-F7.3** [手动] 修改宠物人格文本 → 保存后，下一次 `[PetLLM] request` 请求体 system message 包含新人格
- [ ] **AC-F7.4** [手动] 修改 ticker 间隔（从 30000 → 5000）→ 日志 `[PetTicker] interval: 30000ms → 5000ms` 出现
- [ ] **AC-F7.5** [手动] 关闭应用重开 → 设定从 localStorage 恢复，LLM 开关 / 人格 / ticker 与关闭前一致

### F8 宠物记忆面板（占位）

- [ ] **AC-F8.1** [手动] 右键 → 宠物记忆 → overlay 显示「即将推出」
- [ ] **AC-F8.2** [手动] 关闭 overlay（点击外部/Esc）正常

### F9 错误容错

- [ ] **AC-F9.1** [手动] 断开网络 → LLM 请求失败 → 日志 `[PetLLM] network error:` → 然后 `[PetLLM] falling back to Stay` → 宠物继续播放当前动画不崩溃
- [ ] **AC-F9.2** [手动] 用无效 API key → HTTP 401 → `[PetLLM] http error 401:` → fallback Stay
- [ ] **AC-F9.3** [手动] mock 服务返回非法 JSON → `[PetLLM] parse error:` → fallback Stay
- [ ] **AC-F9.4** [手动] mock 服务返回 `{"action":"switch","to":"nonexistent"}` → `[PetLLM] invalid animation id:` → fallback Stay

### F10 DecisionContext 扩展

- [ ] **AC-F10.1** [代码评审] Rust `DecisionContext` 含 `time_of_day` / `recent_history` / `llm_enabled` / `pet_personality` 可选字段
- [ ] **AC-F10.2** [代码评审] TS `DecisionContext` 含对应字段
- [ ] **AC-F10.3** [日志] ticker 触发的 `[PetCmd] decide_next_state called with:` 日志中包含 `timeOfDay` 字段
- [ ] **AC-F10.4** [代码评审] `recentHistory` 在状态机内部为 FIFO ring buffer，最多 5 条

### R-级验收

- [ ] **AC-R1** [代码评审] `decider.rs` 的 `decide_next_state` 是 `async fn`
- [ ] **AC-R2** [代码评审] 超时常量 = `max(10000, ticker_interval_ms * 0.8)`，硬编码在 `llm.rs`
- [ ] **AC-R3** [代码评审] API key 通过 `std::env::var("LLM_API_KEY")` 读取，源码中无可硬编码 key
- [ ] **AC-R4** [手动] `LLM_ENABLED=false` + 启动 → 右键菜单可以手动切动画，行为同 01
- [ ] **AC-R5** [手动] overlay 与右键菜单互斥（打开 overlay 时右键菜单不显示，或先关闭 overlay）
- [ ] **AC-R8** [手动] 设定面板改 ticker 间隔后立即生效，无需重启

### 异常验收

- [ ] **AC-E1** [手动] mock 服务延迟 15 秒 → 超时 → `[PetLLM] timeout` → fallback Stay（期间 ticker 的 inFlight 阻断并发请求）
- [ ] **AC-E2** [手动] 连续 3 次 LLM 失败 → 每次 tick 仍尝试调用，每次都 fallback Stay，应用不崩

---

## 8. 变更历史

- 2026-06-07 v1：建 spec，status: DRAFT。基于 F1-F10 + R1-R10，覆盖 OpenAI 兼容 LLM 调用 + 右键菜单重构为宠物管理中心 + overlay 面板 + 持久化设定
