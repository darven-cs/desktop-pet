# Spec 模板

复制本文件到 `docs/specs/<NN>-<功能名>.md`，按章节填充。

---

## 模板正文

````markdown
---
name: <一句话功能名, 如"多动画状态机">
status: DRAFT
created: <YYYY-MM-DD>
owner: darven
related: []
---

# <功能名>

**一句话**：<用户能看见的最终行为, 一句话说清>

---

## 1. 功能清单

按用户视角列出 1-2 级行为点。

- [ ] **F1**：<动词+对象，如"右键宠物弹出菜单">
- [ ] **F2**：<如"菜单项点击切换动画">
- [ ] **F3**：<如"切动画时把 sprite sheet 替换">

> 每个 F 编号都要在 §3 接口契约 / §6 验收用例里被引用。

---

## 2. 业务规则

不变量 + 约束。比功能清单更"隐形"。

- **R1**：<如"窗口尺寸 = 当前动画单帧尺寸">
- **R2**：<如"切换动画时不能闪黑屏">
- **R3**：<如"鼠标右键必须 preventDefault,否则会触发系统菜单">

---

## 3. 接口契约

按"调用方"分小节。每条接口必含 path / method / 入参 / 出参 / 错误码。

### 3.1 前端 → Rust（Tauri command）

#### `play_animation(animation_id: string)`

- **path / method**：`invoke('play_animation', { animationId })`
- **入参**：
  | 字段 | 类型 | 必填 | 校验 |
  |---|---|---|---|
  | `animationId` | string | 是 | 必须是 `AnimationRegistry` 内的已知 key |
- **出参**：`{ ok: true, frame: number } | { ok: false, code: string, message: string }`
- **错误码**：
  | code | 含义 | 触发条件 |
  |---|---|---|
  | `E_ANIM_NOT_FOUND` | 动画不存在 | animationId 不在 registry |
  | `E_ALREADY_PLAYING` | 已在播同一个 | 重复调用同 id |
  | `E_FRAMES_MISSING` | 帧资源缺失 | sheet 文件读不到 |

### 3.2 Rust → 前端（Tauri event）

#### `animation_finished`

- **触发条件**：当前动画自然播完一次（`iteration` 完）
- **payload**：`{ animationId: string, totalFrames: number }`

### 3.3 前端内部（method / composable）

#### `useAnimation().switch(id: string)`

- **入参**：`id: AnimationId`
- **出参**：`Promise<void>`
- **副作用**：替换 `background-image`,重置 `animation`,更新内部 ref

### 3.4 窗口操作（如适用）

#### `setWindowSize(width: number, height: number)`

- **permission**：`core:window:allow-set-size`
- **前置**：尺寸必须 ≥ 单帧尺寸
- **后置**：CSS `.pet-sprite` 的 `width/height` 同步更新

---

## 4. 数据结构

### 4.1 `AnimationRegistry`

```ts
type AnimationId = 'touch_nose' | 'think' | 'poop';

interface AnimationEntry {
  id: AnimationId;
  sheetPath: string;         // 例 '/sprites/touch_nose_sheet.png'
  frameCount: number;        // 例 28
  frameWidth: number;        // 例 240
  frameHeight: number;       // 例 240
  fps: number;               // 例 25
  loop: 'infinite' | 'once';
}
```

### 4.2 `AnimationState`（运行时）

```ts
type AnimationState =
  | { phase: 'playing'; current: AnimationId; iteration: number }
  | { phase: 'transitioning'; from: AnimationId; to: AnimationId; progress: 0..1 }
  | { phase: 'idle'; current: AnimationId };
```

---

## 5. 异常与边界

| 场景 | 期望行为 | 错误码/恢复方式 |
|---|---|---|
| 首次启动, 无 sheet 文件 | 退到静态占位图, 状态 = 'idle', log 警告 | 不报错 |
| sheet 文件被用户手动删了 | 自动降级到默认动画, log error | `E_FRAMES_MISSING` |
| 切到当前正在播的动画 | 忽略调用, 不重置进度 | `E_ALREADY_PLAYING` |
| 单帧尺寸 ≠ 窗口尺寸 | 拒绝切换, 提示先改窗口 | `E_SIZE_MISMATCH` |
| 连续 10 次快速切换 | 防抖 200ms, 期间只接最后一次 | — |
| 窗口被最小化 | 暂停动画计时, 恢复时继续 | — |

---

## 6. 验收用例

每条都是 `✓/✗` 的可勾选项，跟 §1 的 F 编号一一对应。

- [ ] **AC-F1**：右键宠物任意位置, 弹出菜单
- [ ] **AC-F1**：菜单内显示当前所有动画名（动态从 registry 读）
- [ ] **AC-F2**：点击菜单项后菜单关闭, sprite 在 200ms 内切到新动画
- [ ] **AC-F2**：切动画期间不闪黑屏
- [ ] **AC-F3**：刷新页面后, 最后一次选中的动画被恢复（持久化）
- [ ] **AC-R1**：窗口尺寸 = 选中动画的单帧尺寸, 改窗口后 sprite 仍居中
- [ ] **AC-E1**：删掉 sheet 文件, 启动后不崩, 显示占位图
- [ ] **AC-E2**：1 秒内连点 10 次菜单项, 只切最后一次
- [ ] **AC-E3**：窗口最小化再恢复, 动画进度连续

---

## 变更历史

- <YYYY-MM-DD>：建 spec, status: DRAFT
````

---

## 填写说明

- **每条 F 编号必须在 §6 至少出现一次**，否则就是"功能写了但没测"
- **每条 AC 必须能 ✓/✗ 客观判断**，主观描述（如"好看"、"流畅"）拆成可测的子项
- **错误码是契约的一部分**——前后端共用一个 enum,改一个就要同步
- **数据结构要先于接口定义**——接口签名引用数据结构, 反过来要回头改
- **异常与边界是 spec 最容易漏的章节**, 评审时必查
