---
name: 前端组合式与状态机
description: 4 个 composable + decider/ 模块，分层明确；状态机是纯函数 reducer，副作用在 App.vue
type: project
---

# 前端组合式与状态机

**状态**：已上线
**上线时间**：2026-06-06
**所属业务**：交互层（01 spec）+ 02 spec (Chat/Memory) + 03/04/05 spec (Agent)

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
    ├── usePetSettings.ts               ← 配置持久化（localStorage）
    ├── usePetChat.ts                   ← 消息列表 + 剪贴板上下文
    ├── usePetEvents.ts                 ← 事件队列 + debounce + agent 调度
    └── (useContextMenu.ts 已删除)      ← 见 bugs/bug_webview2_contextmenu_black_flash.md
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

## 五、右键菜单实现（迁移到 Tauri native）

`useContextMenu` 已删除。菜单现由 Rust 端 `tauri::menu::Menu` + `WebviewWindow::popup_menu` 在 OS 层绘制，前端通过 `invoke("show_context_menu")` 触发 + `listen("context-menu-click", ...)` 接收点击事件。

**为什么迁**：in-webview `<div>` 菜单在 Windows WebView2 上有 native context menu 黑框闪烁 + setSize 异步裁剪问题。详见 [bugs/bug_webview2_contextmenu_black_flash.md](../bugs/bug_webview2_contextmenu_black_flash.md)。

**当前实现**：
- Rust（`src-tauri/src/lib.rs::show_context_menu`）：6 个 `MenuItemBuilder::with_id("ctx.xxx", "...")` + 1 个 `PredefinedMenuItem::separator` + `Menu::with_items` + `window.popup_menu`
- 前端（`App.vue`）：`@contextmenu.prevent="onContextMenu"` → `invoke("show_context_menu")`；`onMounted` 里 `listen<string>("context-menu-click", ...)` 路由到 `showOverlay` / `close`
- 菜单 id 前缀 `ctx.` 避免和未来其他菜单（tray 等）冲突

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
- 2026-06-07：05 spec 前端。`types/pet.ts` Decision 新增 set_reminder 变体、PetEvent 新增 reminder_triggered 变体、user_interaction.interaction 新增 "chat"；`usePetEvents` 新增 consecutiveStays/lastAgentCallAt 追踪、shouldProactiveFlush 主动唤醒（5分钟静默+2分钟无交互）、退避逻辑（连续 stay>=4 不再 flush、>=2 延长 10s debounce）、reminder_triggered 立即 flush；`useAnimationStateMachine` 新增 PendingReminder/reminders 管理（MAX_REMINDERS=5）、set_reminder case（setTimeout 触发 pushEvent reminder_triggered）、isWaitingRef + interruptWait 方法暴露、wait 分支追踪 isWaiting、新增 onPushEvent 回调参数；`App.vue` 解构 interruptWait + 新增 onChatMessageSent（打断 wait + push chat 事件）+ 绑定 @chat-sent + 传入 onPushEvent；`PetChatPanel.vue` 新增 chatSent emit 并在 onSend 中触发
- 2026-06-07：05 spec 修 bug + 配置化。① 修主动 chat 失效 bug：`usePetEvents.flush()` 加 `isProactive` 参数，proactive flush 跳过 `isDecidableEvent` 检查（之前主动唤醒被 `flush` 内部检查拦截，agent 根本没被调用）。② `usePetSettings` 新增 `proactiveIntervalMs`(300000) / `minSilenceMs`(120000) 两个持久化配置项，`usePetEvents` 改为从 settings 读取（不再硬编码）。③ `PetSettingsPanel.vue` 新增"主动聊天间隔"和"静默阈值"两个 number input，删除"Ticker 间隔"输入框（保留字段但不在 UI 暴露）；`.panel-body` 加 `max-height: 320px; overflow-y: auto` 防止内容溢出导致保存键看不见
- 2026-06-07：动画扩展 + 右键菜单精简。① `types/pet.ts` 的 `AnimationId` union 从 3 项扩为 11 项（含 shush/thumbs_up/nervous/sleep/peek/knead/heartbeat/cloud）。② `App.vue` 的 `onContextMenu` 删掉 "手动切动画" 子菜单（用户要求）。子菜单渲染/CSS 代码保留（`item.type === 'submenu'` 分支仍在），不影响当前菜单展示，但以后想加别的子菜单可直接复用
- 2026-06-07：修右键退出无效。根因 Tauri v2 `core:window:default` 不含 `allow-close`，调用方又没 await/catch → 静默失败。修法：capabilities 加 `core:window:allow-close` + 退出 onClick 改 async + try/catch + `[PetExit]` 日志
- 2026-06-07：Windows 右键菜单彻底改造。删除 `useContextMenu` composable + Vue `<div class="context-menu">` + 6 个 CSS 类 + 3 个 MENU_EST_* 常量 + 2 个 computed 里的菜单尺寸逻辑 + menuPixelVal helper。改用 Rust `tauri::menu::Menu` + `WebviewWindow::popup_menu`（OS 层绘制，绕开 WebView2 Chromium context menu 闪烁），通过 `app.emit("context-menu-click", id)` → 前端 `listen<string>` 路由动作。`needWindowW/H` 不再因菜单变大。修法详见 [bugs/bug_webview2_contextmenu_black_flash.md](../bugs/bug_webview2_contextmenu_black_flash.md)
