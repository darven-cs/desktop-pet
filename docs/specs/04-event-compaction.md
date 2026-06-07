---
name: 事件合并与 Agent 调用节流
status: DONE
created: 2026-06-07
owner: darven
related:
  - ../../docs/specs/03-agent-loop.md
---

# 04 · 事件合并与 Agent 调用节流

**一句话**：在事件 flush 到 agent 之前，对同类型事件做合并压缩、过滤噪音事件、收紧 budget 参数、让 TimerTick 只在有"值得决策"的事件时才触发 LLM 调用——解决"12 个 click 事件 → 41 个事件批量灌给 LLM"的高频噪音问题。

> 本 spec 是对 03-agent-loop 的**行为修正**。03 的 F1 事件采集不变，F2 触发策略和 F4 Budget 参数被替换。

---

## 1. 功能清单

| 编号 | 行为 |
|---|---|
| **F1** | flush 前事件合并：N 个连续同类型事件压缩成 1 条摘要事件（如 12 clicks → `user_interaction(click, count: 12)`） |
| **F2** | 事件过滤：`animation_completed` 事件不进入 agent 决策队列（前端仍在 `animationend` 时 dispatch 状态切换，但不触发 LLM） |
| **F3** | TimerTick 条件 flush：TimerTick 只在队列中存在"可决策事件"（user_interaction / window_focus_changed）时才 flush；队列为空或仅含 TimerTick 自身时跳过 |
| **F4** | Budget 收紧：`max_calls_per_minute` 从 6 降到 3；`min_call_interval_ms` 从 5000 提到 15000 |
| **F5** | 向后兼容：03 的其余行为（inFlight 防并发、debounce 2s、agent loop 本身）保持不变 |

---

## 2. 业务规则

| 编号 | 规则 |
|---|---|
| **R1** | 事件合并只发生在 `flush()` 内部，`pushEvent()` 接口不变——外部代码无感知 |
| **R2** | 合并粒度：按事件类型分组。`user_interaction` 进一步按 `interaction` 子类型分组（click 和 drag_end 分别计数） |
| **R3** | 合并后的 `timestamp` 取组内最早的时间戳，`count` 字段记录合并数量 |
| **R4** | `animation_completed` 在 `pushEvent` 阶段仍入队，但在 `flush` 前被过滤掉（不发送给 agent） |
| **R5** | "可决策事件"定义为：`user_interaction`（任意子类型）或 `window_focus_changed`。`timer_tick` 和 `animation_completed` 不算 |
| **R6** | TimerTick 条件 flush 只影响"是否调用 LLM"，不影响"是否清空队列"——如果队列里有 animation_completed 等噪音事件，它们在 flush 时一并被清掉 |
| **R7** | 合并后如果只有 1 条事件（count=1），格式与 03 一致，不带 count 字段——减少 prompt 变化 |
| **R8** | 合并后事件列表最多 10 条（防止极端情况 prompt 过长）。超出时保留最早和最晚的各 5 条 |

---

## 3. 接口契约

### 3.1 `PetEvent` 类型扩展（TS 端）

`PetEvent` 的序列化格式不变（`pushEvent` 入参不变），合并发生在 flush 内部，不影响类型定义。

新增内部合并结果类型（不跨 IPC）：

```ts
// 仅 flush 内部使用，不导出
interface CompactedEvent {
  type: PetEvent["type"];
  // user_interaction 专有
  interaction?: string;
  // window_focus_changed 专有
  focused?: boolean;
  // 合并元数据
  timestamp: number;   // 组内最早时间戳
  count: number;       // 合并了几条
}
```

### 3.2 `format_events_summary` 变更（Rust 端）

`PetEvent` 枚举**不变**。合并在前端完成，传给后端的仍是标准 `PetEvent` 数组，但数量大幅减少。

后端收到的事件里，如果 prompt 显示 `"3 次 click"`，这是因为前端在构建 user prompt 时做了合并格式化——不改变 Rust 的 `PetEvent` 结构。

### 3.3 `flush()` 行为变更

```
flush():
  if inFlight: return
  if queue is empty: return

  // Step 1: 过滤 animation_completed
  filtered = queue.filter(e => e.type !== "animation_completed")

  // Step 2: 如果没有可决策事件，清空队列并返回（不调 LLM）
  hasDecidable = filtered.some(e =>
    e.type === "user_interaction" || e.type === "window_focus_changed"
  )
  if !hasDecidable:
    queue.splice(0)  // 清空，含已过滤的 animation_completed
    return

  // Step 3: 合并同类型事件
  compacted = compactEvents(filtered)

  // Step 4: 构建 prompt 时格式化合并结果
  eventsForPrompt = compacted.map(formatCompacted)

  // Step 5: 调用 agent_decide（与 03 相同）
  ...
```

### 3.4 TimerTick 行为变更

```
pushEvent(event):
  queue.push(event)

  if event.type === "timer_tick":
    // 条件 flush：只在有可决策事件时触发
    hasDecidable = queue.some(e =>
      e.type === "user_interaction" || e.type === "window_focus_changed"
    )
    if hasDecidable:
      flush()
    else:
      // 没有 user_interaction / window_focus_changed
      // 清空队列里的 timer_tick，不调 LLM
      queue = queue.filter(e => e.type !== "timer_tick")
  else:
    // 非 timer_tick：debounce 2s（与 03 一致）
    resetDebounce(2000)
```

### 3.5 Budget 参数变更

| 参数 | 03 旧值 | 04 新值 |
|---|---|---|
| `max_calls_per_minute` | 6 | 3 |
| `min_call_interval_ms` | 5000 | 15000 |

`max_steps` 保持 5 不变。

---

## 4. 数据结构

无新增跨 IPC 数据结构。所有变更在 `usePetEvents.ts` 内部完成。

### 4.1 内部合并结果（前端 only）

见 §3.1 `CompactedEvent`。

### 4.2 事件摘要格式化

合并前后的事件摘要对比：

**合并前（03 行为）：**
```
最近事件：
- [11:57:50] user_interaction(click)
- [11:57:50] user_interaction(click)
- [11:57:51] user_interaction(click)
... (共 12 条)
```

**合并后（04 行为）：**
```
最近事件：
- [11:57:50] user_interaction(click) x12
- [11:57:53] timer_tick
```

如果 count=1，省略 `x1`，保持与 03 一致。

---

## 5. 异常与边界

| 场景 | 期望行为 |
|---|---|
| 队列里只有 timer_tick | TimerTick flush 时检测无可决策事件，清空队列，不调 LLM |
| 队列里只有 animation_completed | 同上，清空不调 LLM |
| 队列里 timer_tick + animation_completed | 同上，清空不调 LLM |
| 队列里 timer_tick + 1 个 user_interaction(click) | flush 触发，合并后 2 条事件发送给 agent |
| 队列里 50 个 user_interaction(click) | 合并为 1 条 `click x50`，只产生 1 次 LLM 调用 |
| 队列里 click + drag_end + click | 合并为 2 条：`click x2` + `drag_end x1` |
| flush 期间又有 click 入队 | inFlight 生效，事件入队等下次处理（03 已有行为） |
| 前端格式化合并事件的 prompt 段落 | 直接在 `usePetEvents.ts` 的 `flush()` 中构建事件摘要字符串，传给后端作为 events 参数的一部分 |

---

## 6. 验收用例

### F1 事件合并

- [ ] **AC-F1.1** [日志] 2s 内连续点击宠物 10 次，`agent_decide` 只收到 ≤ 3 条事件（合并后的 user_interaction + 可能的 timer_tick）
- [ ] **AC-F1.2** [日志] 1 次 click + 1 次 drag_end，收到 2 条事件（不同子类型不合并）
- [ ] **AC-F1.3** [代码评审] `flush()` 中有 `compactEvents()` 函数，按 type + interaction 分组合并
- [ ] **AC-F1.4** [日志] 合并后 user prompt 中可见 `x N` 格式的事件摘要（N > 1 时）
- [ ] **AC-F1.5** [日志] 合并后 count=1 时不带 `x1`，与 03 格式一致

### F2 事件过滤

- [ ] **AC-F2.1** [代码评审] `flush()` 中过滤掉 `animation_completed` 事件
- [ ] **AC-F2.2** [日志] 一次 `loop: once` 动画播完后，日志中不出现 `agent_decide called`（不触发 LLM）
- [ ] **AC-F2.3** [代码评审] `onSpriteAnimationEnd` 仍 dispatch 状态切换（本地行为不变），只是不触发 agent

### F3 TimerTick 条件 flush

- [ ] **AC-F3.1** [日志] 宠物启动后无任何交互，30s ticker 触发时日志无 `agent_decide called`（不调 LLM）
- [ ] **AC-F3.2** [日志] 用户点击宠物后，下一个 ticker tick 触发 `agent_decide`（有可决策事件时 flush）
- [ ] **AC-F3.3** [日志] 连续 3 个 ticker tick 无交互，0 次 `agent_decide` 调用

### F4 Budget 收紧

- [ ] **AC-F4.1** [代码评审] `AgentBudget::new()` 中 `max_calls_per_minute = 3`
- [ ] **AC-F4.2** [代码评审] `AgentBudget::new()` 中 `min_call_interval_ms = 15000`
- [ ] **AC-F4.3** [日志] 1 分钟内触发 3 次 agent loop 后，第 4 次日志 `[PetAgent] rate limited`

### F5 向后兼容

- [ ] **AC-F5.1** [代码评审] `pushEvent()` 签名不变
- [ ] **AC-F5.2** [代码评审] `usePetEvents` 返回值不变（pushEvent / flush / queueLength）
- [ ] **AC-F5.3** [代码评审] Rust 端 `PetEvent` 枚举不变
- [ ] **AC-F5.4** [代码评审] `agent/mod.rs` 的 `run_agent_loop` 无变更

### R 级验收

- [ ] **AC-R1** [代码评审] 合并逻辑仅在 `usePetEvents.ts` 的 `flush()` 中，不影响外部
- [ ] **AC-R3** [代码评审] 合并后 timestamp 取组内最早值
- [ ] **AC-R7** [代码评审] count=1 时不序列化 count 字段
- [ ] **AC-R8** [代码评审] 合并后事件列表上限 10 条

### 异常验收

- [ ] **AC-E1** [日志] 50 个 click 事件合并后只产生 1 条摘要，LLM 正常响应
- [ ] **AC-E2** [日志] 队列只有 timer_tick + animation_completed 时，0 次 LLM 调用，应用不卡

---

## 7. 文件变更清单

### 修改文件

| 文件 | 变更 |
|---|---|
| `src/composables/usePetEvents.ts` | 新增 `compactEvents()` 函数；`flush()` 增加过滤+合并逻辑；`pushEvent()` TimerTick 分支改为条件 flush |
| `src-tauri/src/agent/budget.rs` | `AgentBudget::new()` 参数调整：`max_calls_per_minute` 6→3，`min_call_interval_ms` 5000→15000 |

### 不变的文件

| 文件 | 原因 |
|---|---|
| `src-tauri/src/agent/mod.rs` | agent loop 本身不变 |
| `src-tauri/src/agent/events.rs` | `PetEvent` 枚举不变，`format_events_summary` 不变 |
| `src-tauri/src/agent/budget.rs` 以外 | 无变更 |
| `src/App.vue` | 事件绑定点不变 |
| `src/types/pet.ts` | 类型定义不变 |

---

## 8. 变更历史

- 2026-06-07 v1：建 spec，status: DRAFT。修正 03 的事件触发策略和 budget 参数
