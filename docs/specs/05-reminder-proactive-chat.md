---
name: 定时提醒 + 空调用优化 + 自主聊天
status: DONE
created: 2026-06-07
owner: darven
related:
  - ../../docs/specs/03-agent-loop.md
  - ../../docs/specs/04-event-compaction.md
---

# 05 · 定时提醒 + 空调用优化 + 自主聊天

**一句话**：让宠物能设置定时提醒（"5分钟后叫我吃饭"）、减少 LLM 空响应浪费、以及在无用户交互时根据时间/场景主动聊天——从"被动等事件"升级为"有记忆有节奏的伙伴"。

> 本 spec 依赖 03（agent loop）和 04（事件合并）。核心改动：新增 `set_reminder` 工具 + `Decision::SetReminder` + 前端提醒管理 + 自主 flush 机制 + 空调用退避。

---

## 1. 功能清单

| 编号 | 行为 |
|---|---|
| **F1** | 新增 `set_reminder` 工具：LLM 可调用 `set_reminder({ message, delay_seconds })` 设置定时提醒，前端设 setTimeout 到期后 push `reminder_triggered` 事件触发 agent 执行提醒 |
| **F2** | Chat→Agent 桥接：用户在对话框发消息后，自动 push `user_interaction("chat")` 事件到队列，让 agent loop 能感知用户刚说过话（从而调用 set_reminder） |
| **F3** | 前端提醒管理：维护 reminders 列表，支持多条并行提醒；到期的提醒 push `reminder_triggered` 事件进入 agent loop |
| **F4** | 空调用退避：前端追踪连续 Stay/空响应次数，连续 ≥ 2 次时将 debounce 从 2s 增加到 10s，≥ 4 次时抑制 flush 直到下次 TimerTick；非空决策后重置计数 |
| **F5** | 自主聊天：TimerTick 在无用户事件时也能触发 agent——条件为距上次 agent 调用 > `proactiveIntervalMs`（默认 5 分钟）且距上次用户交互 > `minSilenceMs`（默认 2 分钟） |
| **F6** | 系统提示词更新：新增 reminder 相关工具说明 + 自主聊天决策规则 + 时间段主动行为建议 |

---

## 2. 业务规则

| 编号 | 规则 |
|---|---|
| **R1** | `set_reminder` 是 terminal 工具，调用后立即返回 `Decision::SetReminder`，不再继续 agent loop |
| **R2** | 提醒延迟范围：`delay_seconds` clamp 到 [10, 3600]（10 秒 ~ 1 小时） |
| **R3** | 前端同时维护最多 5 条活跃提醒。超出时最早的提醒被丢弃（FIFO），日志 `[PetReminder] dropped oldest reminder` |
| **R4** | `reminder_triggered` 事件是可决策事件（与 user_interaction 同级），会触发 agent flush |
| **R5** | 提醒触发时 agent 的 events summary 中包含 `reminder_triggered(message: "...")`，agent 可选择 speak_to_user 或 switch_animation 来执行提醒 |
| **R6** | Chat→Agent 桥接仅在 `send_message` 成功返回后 push 事件，失败不 push |
| **R7** | 空调用退避计数器 (`consecutiveStays`) 在任何非 Stay 决策后归零 |
| **R8** | 自主 chat 触发时，events 列表中只有 1 条 `timer_tick`。agent 的 system prompt 负责引导其判断是否该主动说话 |
| **R9** | `proactiveIntervalMs` 和 `minSilenceMs` 使用与 `tickerIntervalMs` 相同的 settings 机制，默认值分别为 300000（5 分钟）和 120000（2 分钟） |
| **R10** | 自主 chat 与 budget 不冲突：自主触发的 agent 调用同样消耗 budget quota，超出时返回 Stay |
| **R11** | 用户在 wait 期间发送聊天消息应立即恢复 ticker（打断 wait），让 agent 能处理用户输入 |

---

## 3. 接口契约

### 3.1 新增工具：`set_reminder`

| 属性 | 值 |
|---|---|
| name | `set_reminder` |
| description | "设置一条定时提醒。到时间后你会主动提醒用户。用于用户让你'N分钟后提醒我X'的场景" |
| parameters | `{ message: { type: string, description: "提醒的内容" }, delay_seconds: { type: number, description: "多少秒后提醒(10-3600)", minimum: 10, maximum: 3600 } }` |
| required | `["message", "delay_seconds"]` |
| is_terminal | `true` |
| handler | 返回 `Decision::SetReminder { message, delay_seconds: clamp(seconds, 10, 3600) }` |

### 3.2 Decision 枚举扩展

Rust 端：
```rust
SetReminder {
    message: String,
    delay_seconds: u32,
}
```

TS 端：
```ts
| { action: "set_reminder"; message: string; delaySeconds: number }
```

### 3.3 PetEvent 扩展

Rust 端新增变体：
```rust
ReminderTriggered {
    message: String,
    timestamp: u64,
}
```

TS 端新增：
```ts
| { type: "reminder_triggered"; message: string; timestamp: number }
```

`user_interaction.interaction` 类型新增 `"chat"`：
```ts
interaction: "click" | "drag_end" | "double_click" | "chat"
```

### 3.4 `usePetEvents` 行为变更

#### `pushEvent` 变更

```
pushEvent(event):
  queue.push(event)

  if event.type === "timer_tick":
    hasDecidable = queue.some(isDecidableEvent)
    if hasDecidable:
      flush()
    else if shouldProactiveFlush():
      flush()  // 自主 chat 路径
    else:
      drain timer_ticks from queue
  else if event.type === "reminder_triggered":
    flush()  // 提醒触发立即 flush
  else:
    // user_interaction / window_focus_changed
    if consecutiveStays >= 4:
      return  // 退避中，不 flush
    debounce = consecutiveStays >= 2 ? 10000 : 2000
    resetDebounce(debounce)
```

#### 新增 `shouldProactiveFlush()`

```ts
function shouldProactiveFlush(): boolean {
  const now = Date.now();
  const sinceLastAgentCall = now - lastAgentCallAt;
  const sinceLastInteraction = now - getLastInteractionAt();
  return sinceLastAgentCall >= proactiveIntervalMs
    && sinceLastInteraction >= minSilenceMs;
}
```

#### 新增 `isDecidableEvent` 扩展

```ts
function isDecidableEvent(ev: PetEvent): boolean {
  return ev.type === "user_interaction"
    || ev.type === "window_focus_changed"
    || ev.type === "reminder_triggered";
}
```

#### flush 中追踪空响应

```ts
// 在 flush() 的 onDecision 回调后：
if (result.decision.action === "stay") {
  consecutiveStays++;
} else {
  consecutiveStays = 0;
}
lastAgentCallAt = Date.now();
```

### 3.5 提醒管理（前端）

在 `usePetEvents` 或 `App.vue` 中维护：

```ts
interface PendingReminder {
  message: string;
  triggerAt: number;  // Date.now() + delay_seconds * 1000
  timerId: number;    // setTimeout ID
}

const reminders: PendingReminder[] = [];
const MAX_REMINDERS = 5;
```

`applyDecision` 新增分支：
```ts
case "set_reminder": {
  const { message, delaySeconds } = decision;
  const triggerAt = Date.now() + delaySeconds * 1000;
  const timerId = window.setTimeout(() => {
    // 移除已触发的提醒
    const idx = reminders.findIndex(r => r.timerId === timerId);
    if (idx >= 0) reminders.splice(idx, 1);
    // push 事件触发 agent
    pushEvent({ type: "reminder_triggered", message, timestamp: Date.now() });
    console.log(`[PetReminder] fired: "${message}"`);
  }, delaySeconds * 1000);

  // 超出上限时丢弃最早的
  while (reminders.length >= MAX_REMINDERS) {
    const oldest = reminders.shift()!;
    clearTimeout(oldest.timerId);
    console.log("[PetReminder] dropped oldest reminder:", oldest.message);
  }

  reminders.push({ message, triggerAt, timerId });
  console.log(`[PetReminder] set: "${message}" in ${delaySeconds}s`);
  break;
}
```

### 3.6 Chat→Agent 桥接

`PetChatPanel` 在 `send_message` 成功后 emit 事件，`App.vue` 监听并 push：

```ts
// App.vue
function onChatMessageSent() {
  pushEvent({
    type: "user_interaction",
    interaction: "chat",
    timestamp: Date.now(),
  });
}
```

### 3.7 Wait 打断

`App.vue` 中，当 wait 期间收到 `onChatMessageSent`，立即恢复 ticker：

```ts
function onChatMessageSent() {
  // 如果正在 wait，恢复 ticker
  if (isWaiting) {
    isWaiting = false;
    startTicker();
  }
  pushEvent({ type: "user_interaction", interaction: "chat", timestamp: Date.now() });
}
```

---

## 4. 数据结构

### 4.1 Decision 扩展

见 §3.2。

### 4.2 PetEvent 扩展

见 §3.3。

### 4.3 PendingReminder（前端内部）

见 §3.5。不跨 IPC。

---

## 5. 异常与边界

| 场景 | 期望行为 |
|---|---|
| 用户设置提醒后关闭应用 | 提醒丢失，不持久化（未来可扩展） |
| 提醒触发时 budget 已耗尽 | 返回 Stay，提醒不执行。日志 `[PetAgent] rate limited, reminder not delivered` |
| 同时设置 6 条提醒 | 第 6 条设置时丢弃最早的，日志 `[PetReminder] dropped oldest` |
| `delay_seconds` 传入 0 | clamp 到 10 秒 |
| `delay_seconds` 传入 99999 | clamp 到 3600 秒 |
| 自主 chat 触发但 LLM 返回空 | consecutiveStays++，下次自主 chat 间隔自动拉长 |
| 用户在 wait 期间发聊天消息 | 恢复 ticker，push chat 事件，agent 立即处理 |
| 空调用退避中收到 reminder_triggered | 仍然 flush（提醒不受退避影响） |
| `reminder_triggered` 的 message 为空 | 仍然 push 事件，agent 自行决定如何处理 |

---

## 6. 验收用例

### F1 提醒工具

- [ ] **AC-F1.1** [代码评审] `tools.rs` 注册了 `set_reminder` 工具，`is_terminal: true`
- [ ] **AC-F1.2** [代码评审] `Decision` 枚举含 `SetReminder { message, delay_seconds }`
- [ ] **AC-F1.3** [日志] LLM 调用 `set_reminder({ message: "吃饭", delay_seconds: 60 })` → 日志 `[PetReminder] set: "吃饭" in 60s`
- [ ] **AC-F1.4** [日志] 60s 后日志 `[PetReminder] fired: "吃饭"` → agent 调用 `speak_to_user` 说出提醒内容
- [ ] **AC-F1.5** [代码评审] handler 对 `delay_seconds` 做 clamp [10, 3600]

### F2 Chat→Agent 桥接

- [ ] **AC-F2.1** [代码评审] `PetChatPanel` 在 send 成功后 emit `chat-sent` 事件
- [ ] **AC-F2.2** [代码评审] `App.vue` 监听 `chat-sent`，调用 `pushEvent({ type: "user_interaction", interaction: "chat" })`
- [ ] **AC-F2.3** [日志] 用户发送聊天消息后，日志出现 `agent_decide called with N events`（N ≥ 1，包含 chat 事件）

### F3 前端提醒管理

- [ ] **AC-F3.1** [代码评审] reminders 数组维护在 `useAnimationStateMachine.ts` 或 `App.vue`
- [ ] **AC-F3.2** [日志] 设置 6 条提醒后，日志 `[PetReminder] dropped oldest reminder`
- [ ] **AC-F3.3** [代码评审] `reminder_triggered` 是可决策事件，会触发 flush

### F4 空调用退避

- [ ] **AC-F4.1** [代码评审] `usePetEvents` 追踪 `consecutiveStays` 计数器
- [ ] **AC-F4.2** [日志] 连续 2 次 Stay 后，debounce 从 2s 变为 10s（可观测：事件触发到 agent_decide 的间隔 ≥ 10s）
- [ ] **AC-F4.3** [日志] 连续 4 次 Stay 后，非 TimerTick 事件不触发 flush
- [ ] **AC-F4.4** [代码评审] `reminder_triggered` 不受退避影响（始终 flush）

### F5 自主聊天

- [ ] **AC-F5.1** [日志] 宠物启动后无任何交互，5 分钟后日志出现 `agent_decide called`（自主 chat 触发）
- [ ] **AC-F5.2** [日志] 自主 chat 触发后，agent 调用 `speak_to_user` 或 `switch_animation`（不是 Stay）
- [ ] **AC-F5.3** [日志] 用户交互后 1 分钟内不触发自主 chat（`minSilenceMs` 生效）
- [ ] **AC-F5.4** [代码评审] `shouldProactiveFlush()` 检查 `proactiveIntervalMs` 和 `minSilenceMs`

### F6 系统提示词

- [ ] **AC-F6.1** [代码评审] system prompt 包含 `set_reminder` 工具说明
- [ ] **AC-F6.2** [代码评审] system prompt 包含自主聊天引导（"在长时间无互动时主动说话"）
- [ ] **AC-F6.3** [代码评审] system prompt 包含时间段主动行为建议（吃饭时间、深夜等）

### R 级验收

- [ ] **AC-R2** [代码评审] `delay_seconds` clamp [10, 3600]
- [ ] **AC-R3** [代码评审] reminders 上限 5 条，FIFO 淘汰
- [ ] **AC-R7** [代码评审] `consecutiveStays` 在非 Stay 决策后归零
- [ ] **AC-R10** [日志] 自主 chat 触发 3 次/分钟后被 budget 限制
- [ ] **AC-R11** [手动] Wait 期间发聊天消息 → ticker 恢复 → agent 处理 chat 事件

### 异常验收

- [ ] **AC-E1** [手动] 用户说"5分钟后提醒我吃饭" → 5分钟后宠物弹出提醒消息
- [ ] **AC-E2** [日志] 提醒触发时 budget 耗尽 → 日志 `[PetAgent] rate limited` → Stay
- [ ] **AC-E3** [日志] `delay_seconds: 0` → clamp 到 10，提醒在 10s 后触发

---

## 7. 系统提示词变更

### 7.1 工具说明追加

```
5. **set_reminder** — 设置一条定时提醒。当用户让你"N分钟后提醒我X"时使用。参数：message（提醒内容）、delay_seconds（多少秒后提醒）
```

### 7.2 事件列表追加

```
- **reminder_triggered** — 你设置的提醒到时间了！请立即用 speak_to_user 把提醒内容告诉用户
- **user_interaction(chat)** — 用户在对话框给你发了消息
```

### 7.3 自主行为规则追加

```
## 自主行为

当长时间没有用户互动时，你可以主动发起对话。适合主动说话的时机：
1. 早中晚饭时间附近（7-8点、11-13点、17-19点）
2. 用户超过10分钟没理你
3. 你之前通过 set_reminder 设置了提醒（收到 reminder_triggered 时必须说话）

不适合主动说话的时机：
1. 深夜（23:00-07:00），除非有提醒要执行
2. 连续主动说话间隔 < 3分钟
3. 用户刚和你互动完（< 2分钟）

收到 reminder_triggered 事件时，必须调用 speak_to_user 把提醒内容告诉用户。
```

---

## 8. 文件变更清单

### 修改文件

| 文件 | 变更 |
|---|---|
| `src-tauri/src/tools.rs` | 新增 `set_reminder` 工具注册 |
| `src-tauri/src/types.rs` | `Decision` 新增 `SetReminder` 变体 |
| `src-tauri/src/agent/events.rs` | `PetEvent` 新增 `ReminderTriggered` 变体；`format_events_summary` 处理新类型 |
| `src-tauri/src/llm.rs` | system prompt 追加工具说明 + 事件说明 + 自主行为规则 |
| `src/types/pet.ts` | `Decision` 新增 `set_reminder`；`PetEvent` 新增 `reminder_triggered`；`interaction` 新增 `"chat"` |
| `src/composables/usePetEvents.ts` | `isDecidableEvent` 新增 `reminder_triggered`；`pushEvent` 增加 `reminder_triggered` 立即 flush + 自主 flush 逻辑 + 空调用退避 + `shouldProactiveFlush()` |
| `src/composables/useAnimationStateMachine.ts` | `applyDecision` 新增 `set_reminder` 分支（reminders 管理 + setTimeout）；wait 打断逻辑；暴露 `isWaiting` 状态 |
| `src/App.vue` | 监听 `PetChatPanel` 的 `chat-sent` 事件 → pushEvent + wait 打断 |

### 不变的文件

| 文件 | 原因 |
|---|---|
| `src-tauri/src/agent/mod.rs` | agent loop 本身不变 |
| `src-tauri/src/agent/budget.rs` | budget 参数不变（04 已收紧） |

---

## 9. 变更历史

- 2026-06-07 v1：建 spec，status: DRAFT。核心：set_reminder + 空调用退避 + 自主 chat
