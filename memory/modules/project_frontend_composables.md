---
name: 前端组合式与状态机
description: 4 个 composable + decider/ 模块，分层明确；状态机是纯函数 reducer，副作用在 App.vue
type: project
---

# 前端组合式与状态机

**状态**：已上线
**上线时间**：2026-06-06
**所属业务**：交互层（01 spec）

4 个 Vue 3 composable + 1 个独立 decider 目录。状态机封装在 `useAnimationStateMachine` 内，**纯函数 reducer**（AC-F4.7），副作用（preload / setSize / 切 CSS）由 App.vue 的 watcher 触发。

---

## 一、模块结构

```
src/
├── types/pet.ts                        ← TS 镜像（由 Rust 派生）
├── decider/index.ts                    ← Decider 接口 + getDefaultDecider（独立目录方便 02 替换）
└── composables/
    ├── useAnimationRegistry.ts         ← F2：启动调 list_animations，缓存为响应式 ref
    ├── useAnimationStateMachine.ts     ← F4：state + dispatch + ticker，reducer 纯函数
    └── useContextMenu.ts               ← F3：绝对定位 <div>，全局单例 + 8px 防出屏
```

## 二、状态机接口

```ts
const { state, dispatch, tickerInterval, lastInteractionAt } = useAnimationStateMachine();
// state 只读（readonly(state)），只能通过 dispatch 改
```

**事件类型**（`StateEvent`）：
- `init` — 启动初始化
- `switch_to` — 切动画（source: dispatch | ticker | init）
- `transition_complete` — preload 完成后调
- `enter_idle` / `exit_idle`

**决策应用**（`applyDecision`）：ticker 调 decider 拿 `Decision` → 翻译为 `StateEvent` 调 dispatch。

## 三、Ticker

- 间隔：`import.meta.env.VITE_PET_TICKER_INTERVAL_MS`，默认 30000
- 启动时校验 > 0，否则抛 `E_INVALID_CONTEXT`（AC-E4）
- 内部 `inFlight` 标志防止慢调阻塞
- 间隔变化时 `console.log("[PetTicker] interval: ${old}ms → ${new}ms")` 然后重启
- 跳过条件：`state.phase === "transitioning"` 时 `console.log("[PetTicker] skipped, phase=transitioning")` 并 return

## 四、Decider 替换接口

```ts
// src/decider/index.ts
export type Decider = (ctx: DecisionContext) => Promise<Decision>;
export function getDefaultDecider(): Decider
```

01 默认实现 = Rust `decide_next_state` 薄包装（永远返 Stay）。02 spec 替换时改 `getDefaultDecider` 内部即可，状态机无感。

## 五、useContextMenu 要点

- **全局单例**：module-level `activeCloser` 防止多个菜单同时开
- **防出屏**：拿 `getCurrentWindow().outerPosition()` + `window.screen.{width,height}` 做翻转 + 8px clamp（AC-F3.8）
- **估算尺寸**：`ESTIMATED_MENU_WIDTH=140`、`ESTIMATED_ITEM_HEIGHT=30`（首次 open 还没 render 所以只能估）
- **关闭触发**：item 点击 / 外部 mousedown（`closest('.context-menu')` 排除）/ Esc

## 六、副作用分层

| 副作用 | 位置 | 触发 |
|---|---|---|
| preload sheet | App.vue | watch state.phase==='transitioning' |
| 切 CSS | App.vue（:style 响应式） | state.current 变 |
| 改窗口尺寸 | App.vue | watch phase 从 transitioning → playing |
| ticker | useAnimationStateMachine 内部 | setInterval |
| 菜单定位/翻转 | useContextMenu | open() |

**铁律**：状态机内部不碰 DOM、不调 Tauri API、不读 env（除 tickerInterval 一次）。这样 02 spec 替换 decider 时状态机不动。

## 七、相关记忆

- [Rust 数据类型](project_rust_types.md) — 状态机消费的 type
- [窗口配置](project_window_setup.md) — setSize 的 Tauri API
- [SDD 工作流](project_sdd_workflow.md) — 改这层要先 CHANGED spec

---

## 变更历史

- 2026-06-06：建模块。3 个 composable + decider/ 上线
- 2026-06-07：02 spec。`useAnimationStateMachine` 新增 recentHistory ring buffer + timeOfDay/llmEnabled/petPersonality 上下文填充 + lastTickerReason 暴露；`useContextMenu` 新增 separator/submenu 类型 + activeSubmenu/toggleSubmenu；新增 `usePetSettings` composable（localStorage 持久化）
- 2026-06-07：Chat + Memory。新增 `usePetChat` composable（消息列表 + sendMessage + 剪贴板上下文）；新增 `PetChatPanel.vue`（对话 overlay）；`PetMemoryPanel.vue` 从占位改为真实记忆列表（按类型筛选 + 时间排序 + importance 星标）；`types/pet.ts` 新增 MemoryEntry / MemoryKind / ChatMessage / ChatResponse 镜像类型
- 2026-06-07（同日二轮）：`PetSettings` 新增 `petName` 字段；`useAnimationStateMachine` 新增 `petName` 参数并传入 DecisionContext；`usePetChat.buildContext()` 传入 petName；`PetSettingsPanel.vue` 新增「宠物名字」输入框；`PetChatPanel.vue` 新增 watch(petSpeakMessage) 支持多次主动对话追加；`types/pet.ts` DecisionContext 新增 petName/memoryContext
- 2026-06-07：Agent Loop 前端。`types/pet.ts` 新增 Decision wait 变体、PetEvent 类型、AgentResult 接口；新增 `usePetEvents` composable（事件队列 + timer_tick 立即 flush / 其他 2s debounce + invoke agent_decide）；`useAnimationStateMachine` 新增 wait 分支（stopTicker + setTimeout 恢复）、applyAgentResult 方法、暴露 startTicker/stopTicker、stopTicker 时清除 waitTimeoutId；`App.vue` 绑定 click/drag_end/animation_completed/window_focus_changed 四类事件到 pushEvent
- 2026-06-07：04 spec 事件合并。`usePetEvents` 新增 compactEvents() 按同类型连续事件合并（count 字段）、filter animation_completed 不触发 agent、TimerTick 条件 flush（只有 user_interaction / window_focus_changed 才 flush）、前端构建 formatCompactedSummary 传 eventsSummary 到后端；`types/pet.ts` DecisionContext 新增 eventsSummary；Rust DecisionContext 新增 events_summary，agent/mod.rs 优先使用前端摘要
