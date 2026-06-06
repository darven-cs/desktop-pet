---
name: 宠物交互层（多动画+右键菜单+状态机+Rust IPC+窗口自适应）
status: FROZEN
created: 2026-06-06
owner: darven
related:
  - ../../memory/modules/project_window_setup.md
  - ../../memory/modules/project_sprite_pipeline.md
  - ../../memory/modules/project_window_drag.md
  - ../../memory/modules/project_ai_pet_vision.md
---

# 01 · 宠物交互层

**一句话**：让桌宠能切动画、弹右键菜单、按状态机驱动动画切换（决策占位），并把窗口尺寸自适应到当前动画单帧——为 02 接 LLM 决策铺好接口。

> 本 spec 只做**基础设施**。LLM 调用、截屏、记忆、Q&A 都不在本 spec,见 [`project_ai_pet_vision.md`](../../memory/modules/project_ai_pet_vision.md) 路线图。

---

## 1. 功能清单

| 编号 | 行为 |
|---|---|
| **F1** | 切动画：动态替换 sprite sheet + 重置 `background-position` + 调整窗口尺寸 |
| **F2** | Rust IPC 拉动画列表:启动时前端调 `list_animations()` 拿到 `AnimationEntry[]`,前端只缓存不硬编码;**首次启动默认动画 = `touch_nose`** |
| **F3** | 右键宠物任意位置弹出菜单(绝对定位 `<div>`),菜单项=可用动画+退出;**菜单自动防出屏** |
| **F4** | 可插拔决策的状态机 + 可配置 ticker(默认 30s);**`transitioning` 状态下 ticker 跳过本次** |
| **F5** | Rust 端真 command:`list_animations` + 占位 `decide_next_state` |
| **F6** | 窗口尺寸自适应单帧尺寸(用 Tauri 内置 `setSize`,`capabilities` 加 permission) |

**LLM 钩子预留(给 02 spec 用的接口,01 不实现)**:
- F4 的 decider 是**可注入函数**,接口签名固定;02 只需替换实现
- F5 的 `decide_next_state` 接口签名固定;02 替换 impl 为 LLM 调用
- `DecisionContext` 字段当前最小,02 需扩展时(用户历史/时间等)不破坏 01 的契约

---

## 2. 业务规则

| 编号 | 规则 |
|---|---|
| **R1** | 窗口尺寸 = 当前动画单帧尺寸 |
| **R2** | 切动画无黑帧(preload 新 sheet 后再切 `background-image`) |
| **R3** | 右键菜单 `contextmenu` 必须 `preventDefault`,不触发系统菜单 |
| **R4** | 错误码是契约,前后端共用 enum(改一个同步改另一个) |
| **R5** | 状态机变更只能通过 `dispatch()`,**禁止外部直接改 state**——这是 02 接 LLM 的前提 |
| **R6** | ticker 间隔通过 `.env` 的 `VITE_PET_TICKER_INTERVAL_MS` 配置(Vite 前缀才能注入 client),默认 30000,不写死常量 |
| **R7** | `AnimationEntry` / `AnimationState` / `DecisionContext` / `Decision` 等结构**在 Rust 端定义**(serde),TS 端类型由 Rust 定义派生——避免后期重复定义 |

---

## 3. 接口契约

### 3.1 前端 → Rust(Tauri command)

#### `list_animations() -> Result<Vec<AnimationEntry>, AppError>`

- **permission**:需在 `capabilities/default.json` 声明(自定义 command)
- **入参**:无
- **出参 ok**:`AnimationEntry[]`,按 `id` 字典序
- **出参 err**:`AppError`(`FramesMissing` | `InternalError`)
- **副作用**:扫描 `public/sprites/*_sheet.png`,过滤掉 PNG 读不到或缺注册信息的条目

#### `decide_next_state(context: DecisionContext) -> Result<Decision, AppError>`

- **permission**:需在 `capabilities/default.json` 声明
- **入参**:`DecisionContext`
- **出参 ok**:`Decision`
- **出参 err**:`AppError`(`InvalidContext` | `InternalError`)
- **01 实现**:**永远返回 `Decision::Stay`**(占位,02 替换为 LLM 调用)
- **稳定保证**:接口签名一旦冻结,02 只能改 impl 不能改签名

### 3.2 Tauri 内置 API(已有 permission,直接用)

| API | 用途 | 当前 capabilities 状态 |
|---|---|---|
| `getCurrentWindow().setSize({width, height})` | F6 改窗口尺寸 | 已有 `core:window:allow-set-size` |
| `getCurrentWindow().startDragging()` | 窗口拖拽(已有) | 已有 `core:window:allow-start-dragging` |

### 3.3 前端 composable

#### `useAnimationRegistry()`

- **职责**:启动时调 `list_animations`,缓存为响应式 `ref<AnimationEntry[]>`
- **暴露**:`{ registry: Ref<AnimationEntry[]>, isLoading: Ref<boolean>, error: Ref<AppError | null> }`
- **错误处理**:`isLoading=false` 后若有 error,UI 显示"宠物暂时打瞌睡了"占位
- **初始状态**:`registry` 解析完前 `currentAnimationId` 临时取 `touch_nose` 作为占位(防止 F1 切动画时引用空 registry)

#### `useAnimationStateMachine()`

- **职责**:封装状态机 + ticker,提供 `dispatch` 入口
- **暴露**:
  ```ts
  {
    state: Ref<AnimationState>,        // 当前状态(只读)
    dispatch: (event: StateEvent) => void,  // 唯一外部入口
    tickerInterval: Ref<number>        // 当前 ticker 间隔(只读)
  }
  ```
- **内部**:
  - 维护 `AnimationState`(§4 定义),`phase` 初始 = `'playing'`,`current` 初始 = `'touch_nose'`
  - 启动 `setInterval` ticker,每 N ms **先检查** `state.phase === 'transitioning'`:是则本次跳过,否则调 `decide_next_state(ctx)` 并应用返回的 `Decision`
  - 监听 `tickerInterval` 变化,自动重启 ticker
  - `tickerInterval` 初始值从 `import.meta.env.VITE_PET_TICKER_INTERVAL_MS` 读,默认 30000

#### `useContextMenu(items: MenuItem[], anchor: {x, y})`

- **职责**:渲染绝对定位的 `<div>` 菜单
- **暴露**:`{ isOpen, open(anchor), close(), menuStyle }`
- **行为**:
  - `open(anchor)`:把菜单定位到鼠标位置,**自动算边界防出屏**(贴右/贴下时反向偏移,留 ≥ 8px 边距)
  - 点击菜单项:触发回调 + `close()`
  - 点击菜单外 / Esc: `close()`
  - 同一时刻只允许一个菜单打开

#### `decider/`

- **位置**:`src/decider/index.ts`(独立目录,方便 02 替换)
- **接口**:
  ```ts
  export type Decider = (ctx: DecisionContext) => Promise<Decision>
  export function getDefaultDecider(): Decider
  ```
- **01 默认实现**:`(ctx) => Promise.resolve({ action: 'stay' })`(占位)
- **02 替换方式**:`getDefaultDecider()` 内部从 import 改成 LLM 客户端,接口不变

### 3.4 错误码契约

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AppError {
    AnimNotFound,         // code: E_ANIM_NOT_FOUND
    FramesMissing,        // code: E_FRAMES_MISSING
    InvalidContext,       // code: E_INVALID_CONTEXT
    InternalError,        // code: E_INTERNAL
}
```

| code | 含义 | 触发 |
|---|---|---|
| `E_ANIM_NOT_FOUND` | 动画 ID 不在 registry | 切到未知 ID |
| `E_FRAMES_MISSING` | sheet 文件读不到 | `list_animations` 扫到无效 PNG |
| `E_INVALID_CONTEXT` | 决策上下文非法 | `decide_next_state` 入参校验失败(如 ticker 间隔 ≤ 0) |
| `E_INTERNAL` | 内部错误 | Rust panic catch / registry 不变量违反 |

TS 端用 `type AppErrorCode = 'E_ANIM_NOT_FOUND' | ...`,跟 Rust enum 一一对应(R7)。

---

## 4. 数据结构

> 所有结构**在 Rust 端定义并 serde**,TS 端类型由 Rust 定义派生(R7)。

```rust
// src-tauri/src/types.rs

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnimationEntry {
    pub id: String,                 // 'touch_nose' | 'think' | 'poop'
    pub sheet_path: String,         // '/sprites/touch_nose_sheet.png'
    pub frame_count: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    pub fps: u32,
    pub loop_mode: LoopMode,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum LoopMode { Infinite, Once }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecisionContext {
    pub current_state: AnimationState,
    pub last_interaction_at: u64,    // epoch ms
    pub ticker_interval_ms: u32,
    // 02 扩展位(本 spec 不加):user_history, time_of_day, screen_snapshot_ref ...
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "action")]
pub enum Decision {
    Stay,
    Switch { to: String, reason: Option<String> },
    EnterIdle,
    ExitIdle,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AnimationState {
    pub phase: Phase,
    pub current: String,
    pub iteration: u32,
    pub transition: Option<Transition>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Phase { Playing, Idle, Transitioning }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub progress: f32,   // 0.0..1.0
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppError { /* 见 §3.4 */ }
```

TS 端(由 Rust 定义派生,本 spec 内手写,后续可改 ts-rs 自动生成):

```ts
// src/types/pet.ts (从 Rust 派生,改 Rust 时同步改这里,02 引入 ts-rs 后自动生成)

export type AnimationId = 'touch_nose' | 'think' | 'poop';
export interface AnimationEntry {
  id: AnimationId;
  sheetPath: string;
  frameCount: number;
  frameWidth: number;
  frameHeight: number;
  fps: number;
  loopMode: 'infinite' | 'once';
}
export type Phase = 'playing' | 'idle' | 'transitioning';
export interface AnimationState {
  phase: Phase;
  current: AnimationId;
  iteration: number;
  transition?: { from: AnimationId; to: AnimationId; progress: number };
}
export type Decision =
  | { action: 'stay' }
  | { action: 'switch'; to: AnimationId; reason?: string }
  | { action: 'enter_idle' }
  | { action: 'exit_idle' };
export interface DecisionContext {
  currentState: AnimationState;
  lastInteractionAt: number;
  tickerIntervalMs: number;
}
export type AppErrorCode =
  | 'E_ANIM_NOT_FOUND' | 'E_FRAMES_MISSING'
  | 'E_INVALID_CONTEXT' | 'E_INTERNAL';
export interface AppError { code: AppErrorCode; message: string }
```

---

## 5. 异常与边界

| 场景 | 期望行为 | 错误码/恢复 |
|---|---|---|
| 首次启动,无任何 sheet 文件 | `list_animations` 返回 `[]`,菜单只显示"退出",state 保持 `current=touch_nose` 试图播放(若 sheet 缺失则 sprite 区空白) | — |
| 单个 sheet 文件被删 | `list_animations` 过滤掉,不报错,日志 warn | — |
| 多个 sheet 文件被删光 | 同"首次启动" | — |
| 切到当前正在播的动画 | 忽略调用,**`background-image` / `animation` / `background-position` 三者都不变**,CSS animation 继续从当前位置播放(不重置) | — |
| 切到不存在的 ID | 前端拦截(已知 registry),若绕过则后端返 `E_ANIM_NOT_FOUND` | `E_ANIM_NOT_FOUND` |
| ticker 期间用户操作改变状态 | 下一个 tick 收到的 `currentState` 是新状态,不是旧状态 | — |
| ticker 期间右键菜单打开 | 决策应用不影响菜单状态(菜单独立 UI 层) | — |
| ticker 期间 phase=transitioning | ticker **跳过本次**(在 ticker 内部判断,见 §3.3),等切完再决策 | — |
| Rust 端 `decide_next_state` 抛错 | 前端 fallback 到 `Decision::Stay`,日志 error | `E_INTERNAL` |
| ticker 间隔 = 0 或负数 | 启动时报 `E_INVALID_CONTEXT` 退出 | `E_INVALID_CONTEXT` |
| registry 内 id 重复 | 启动时报 `E_INTERNAL` 退出(不变量违反) | `E_INTERNAL` |
| 右键菜单点击菜单外 | 菜单关闭 | — |
| 右键菜单打开时再右键 | 先关再开,位置更新 | — |
| 菜单贴屏幕右/下边缘 | 菜单反向偏移,留 ≥ 8px 边距不出屏 | — |

---

## 5.5 日志约定

> 所有可观察行为必须可 grep。验收时按此格式 grep 验证。

| 事件 | 格式 | 触发 | 来源 |
|---|---|---|---|
| 状态变更 | `[PetState] <from.phase> → <to.phase>, current: <id>, by: <dispatch\|ticker\|init>` | `dispatch` / `ticker` / 启动 | 前端 `useAnimationStateMachine` |
| Rust command 调用 | `[PetCmd] <command_name> called with: <args>` | 每次 Rust command 被调 | Rust `lib.rs` |
| Rust command 错误 | `[PetError] <command_name> → <code>: <message>` | command 返回 `Err` | Rust `lib.rs` |
| 跳过 ticker | `[PetTicker] skipped, phase=transitioning` | ticker 命中跳过条件 | 前端 `useAnimationStateMachine` |
| Ticker 间隔变化 | `[PetTicker] interval: <old>ms → <new>ms` | `tickerInterval` ref 变化 | 前端 |

**前缀**:`[PetState] [PetCmd] [PetError] [PetTicker]` 四类,02+ 可扩展 `[PetLLM] [PetMemory]` 等。

---

## 6. 验收用例

> 每条都是 ✓/✗ 布尔判断。"怎么测"用 [手动]/[代码评审]/[日志]/[env] 标注。

### F1 切动画

- [ ] **AC-F1.1** [手动] 选 think 后,250ms 内 sprite 显示 think 第一帧
- [ ] **AC-F1.2** [手动] 切动画期间,人眼 5 次连续切换每次 < 50ms 黑帧(R2)
- [ ] **AC-F1.3** [手动] 点击当前动画,菜单项标记"当前"且 sprite 不重启(CSS animation 继续)
- [ ] **AC-F1.4** [手动] 切到 think 后窗口尺寸 = 155×155(think 的单帧尺寸)

### F2 Rust IPC 拉动画列表

- [ ] **AC-F2.1** [日志] 应用启动后 500ms 内 `list_animations` 被调用一次(可 grep `[PetCmd] list_animations called`)
- [ ] **AC-F2.2** [代码评审] 返回的 `AnimationEntry[]` 长度 = `public/sprites/*_sheet.png` 文件数
- [ ] **AC-F2.3** [手动] 删掉一个 sheet 文件后启动,日志无 error,菜单少一项
- [ ] **AC-F2.4** [手动] 首次启动(无任何用户操作)默认播放 `touch_nose`,窗口尺寸 = 240×240

### F3 右键菜单

- [ ] **AC-F3.1** [手动] 右键宠物任意位置,菜单在鼠标位置弹出
- [ ] **AC-F3.2** [手动] 菜单显示所有动画名 + 退出项
- [ ] **AC-F3.3** [手动] 点击动画名后菜单关闭,对应动画被播放
- [ ] **AC-F3.4** [手动] 点击退出后应用退出
- [ ] **AC-F3.5** [手动] 菜单打开时按 Esc,菜单关闭
- [ ] **AC-F3.6** [手动] 菜单打开时点击菜单外,菜单关闭
- [ ] **AC-F3.7** [手动] 菜单打开时再右键另一位置,菜单移过去(不重叠)
- [ ] **AC-F3.8** [手动] 在屏幕右下角(贴边 5px 内)右键宠物,菜单向左上反向偏移,留 ≥ 8px 边距,不出屏

### F4 状态机 + ticker + decider

- [ ] **AC-F4.1** [代码评审] `state` 是只读的 `Ref<AnimationState>`,只能通过 `dispatch` 改
- [ ] **AC-F4.2** [env] 项目根 `.env` 写 `VITE_PET_TICKER_INTERVAL_MS=2000` 后 `npm run tauri dev`,devtools 看 ticker 每 2s 调一次 `decide_next_state`(可 grep `[PetTicker]` 验证)
- [ ] **AC-F4.3** [代码评审] ticker 间隔从 `import.meta.env.VITE_PET_TICKER_INTERVAL_MS` 读,默认 30000
- [ ] **AC-F4.4** [日志] decider 返回 `Stay` 时,grep `[PetState]` 无新增状态变更行
- [ ] **AC-F4.5** [代码评审] `src/decider/index.ts` 是独立目录,`getDefaultDecider` 是唯一导出
- [ ] **AC-F4.6** [代码评审] ticker 内部检查 `state.phase === 'transitioning'` 时跳过 decider 调用(grep `[PetTicker] skipped` 可验)
- [ ] **AC-F4.7** [代码评审] 状态机 reducer 是**纯函数**(`(state, event) → state`),无副作用

### F5 Rust command

- [ ] **AC-F5.1** [代码评审] `list_animations` 在 `capabilities/default.json` 声明
- [ ] **AC-F5.2** [代码评审] `decide_next_state` 在 `capabilities/default.json` 声明
- [ ] **AC-F5.3** [日志] 前端调 `decide_next_state`,Rust 日志打印 `[PetCmd] decide_next_state called with: <json>`
- [ ] **AC-F5.4** [日志] 01 实现下,`decide_next_state` 永远返回 `Stay`,无 `[PetState]` 状态变更日志伴随

### F6 窗口尺寸自适应

- [ ] **AC-F6.1** [手动] 切到 think 后 100ms 内窗口尺寸变 155×155
- [ ] **AC-F6.2** [代码评审] 切动画逻辑内调用 `getCurrentWindow().setSize({width, height})`
- [ ] **AC-F6.3** [代码评审] `capabilities/default.json` 已有 `core:window:allow-set-size`

### R-级验收

- [ ] **AC-R1** [手动] 任意时刻窗口尺寸 = 当前动画的单帧尺寸
- [ ] **AC-R3** [手动] 右键宠物后系统右键菜单**不**弹出
- [ ] **AC-R4** [代码评审] TS 端 `AppErrorCode` 与 Rust 端 `AppError` 一一对应
- [ ] **AC-R5** [代码评审] 全局 grep `state.value =` 在状态机文件外**无**结果
- [ ] **AC-R6** [env] 启动时 `.env` 写 `VITE_PET_TICKER_INTERVAL_MS=500` 启动可生效(grep `[PetTicker]` 验证间隔 ≈ 500ms)
- [ ] **AC-R7** [代码评审] `src-tauri/src/types.rs` 是结构定义唯一源,TS 端文件有"由 Rust 派生"注释

### 异常验收

- [ ] **AC-E1** [手动] 删光所有 sheet 后启动,UI 不崩,菜单只显示"退出"
- [ ] **AC-E2** [日志] ticker 期间用户点击右键菜单,ticker 收到的 `currentState` 是新状态
- [ ] **AC-E3** [日志] 手动 mock 后端抛 `E_INTERNAL`,前端 fallback 到 `Stay` 且日志打印 `[PetError]`
- [ ] **AC-E4** [手动] `.env` 写 `VITE_PET_TICKER_INTERVAL_MS=0` 启动,应用退出并显示 `E_INVALID_CONTEXT` 错误

---

## 变更历史

- 2026-06-06 v1:建 spec, status: DRAFT。基于 F1-F6 + R1-R7 清单,首次落 spec
- 2026-06-06 v2:基于评审⚠修复 6 条(VITE_ 前缀统一 / ticker skip 位置明确 / 默认动画明确 / 切同动画定义明确 / 日志约定新增 / 菜单防出屏补 AC),重新提交评审
- 2026-06-06 v3:评审通过 0✗0⚠, status: FROZEN。**锁定契约**——后续修改必须走 CHANGED → DRAFT 流程,不允许直接改 FROZEN 内容
