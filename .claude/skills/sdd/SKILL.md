---
name: sdd
description: Spec-Driven Development workflow for this project. Use this skill whenever the user wants to implement, add, build, design, or change a feature in the desktop pet app — even if they just say "加个 X", "实现 Y", "搞个 Z", "写个 XX 功能", or "我想让宠物能...". Triggers on new functionality, new behaviors, or any code change that introduces a new capability. Does NOT trigger for pure bug fixes, refactors, doc updates, or sprite asset generation (use gif-to-sheet for that). When triggered, the skill enforces a spec-first workflow: a Spec must reach `status: FROZEN` before any code for that feature is written, and all code/tests must trace back to the frozen Spec.
---

# SDD — Spec-Driven Development

把"先把契约写死、再写代码"固化成项目级流程。每次涉及新功能/新行为，**先写 Spec → 评审冻结 → 才动代码 → 对照 Spec 验收**。需求变更走"改 Spec → 重评 → 改代码"。

---

## 什么时候必须走 SDD

| 触发 | 例子 | 处理 |
|---|---|---|
| 用户说要做新功能 | "加个右键菜单"、"做个多动画切换"、"加个拖入物品" | **进入 SDD 流程** |
| 用户说改行为 | "让宠物被点击时有反应"、"窗口被双击时…" | **进入 SDD 流程**（旧 spec 进 CHANGED） |
| 用户说实现某个机制 | "做个状态机"、"做个事件总线" | **进入 SDD 流程** |
| 修 bug（不改外部行为） | "拖拽不灵"、"sprite 错位" | **不走 SDD**，直接修 |
| 重构/清理 | "拆个文件"、"重命名函数" | **不走 SDD**，但要保证不改外部行为 |
| 加 sprite 资源 | 用户丢一个新 GIF | 走 `gif-to-sheet` skill，不开 SDD |
| 写/改文档 | 改 CLAUDE.md、memory 维护 | **不走 SDD** |

判断不确定时倾向走 SDD——"宁严勿松"是本流程的底线。

---

## 状态机（Spec 生命周期）

```
DRAFT ──→ REVIEW ──→ FROZEN ──→ IMPLEMENTING ──→ TESTING ──→ DONE
  ↑          │           │             │              │
  │          │           │             │              │
  └──────────┘           └─────────────┴──────────────┘
       (评审不过)         (需求变更 → CHANGED → 回 DRAFT)
```

| 状态 | 含义 | 谁能往下推 | 推的条件 |
|---|---|---|---|
| `DRAFT` | 正在写 spec | Claude + 用户 | spec 写完整后切到 `REVIEW` |
| `REVIEW` | 待评审 | Claude | Claude 按评审清单给出意见；用户逐条 ✓/✗ 决定 |
| `FROZEN` | 评审通过，锁定契约 | — | 评审 ✓/✗ 全过 → `FROZEN`；任一 ✗ → 回 `DRAFT` |
| `IMPLEMENTING` | 按 spec 写代码 | Claude + 用户 | 代码完成切到 `TESTING` |
| `TESTING` | 对照 spec 验收 | Claude + 用户 | 全部 assertion 通过 → `DONE`；不一致 = bug，回 `IMPLEMENTING` |
| `DONE` | 已交付 | — | 需求变更时切 `CHANGED` → `DRAFT` |

**硬规则**：FROZEN 之后**禁止改 spec**。要改就走变更流程（切 `CHANGED` → 回 `DRAFT`），让所有人看到"契约被打破"。

---

## 5 + 1 步流程

### Step 0 · 检测意图 + 检查 Spec 状态（硬拦截点）

用户说"实现 X / 加个 X / 改 X 行为"时，**先查 `docs/specs/`**：

1. **有现成 spec**：读 frontmatter 的 `status`
   - `DONE` / `FROZEN` / `IMPLEMENTING` / `TESTING` → 提醒用户当前状态，问"继续 / 变更 / 中止"
   - `DRAFT` / `REVIEW` → 提示"先完成 spec 再写代码"
   - 没有 → 走 Step 1
2. **没有现成 spec**：进入 Step 1

**这是硬拦截**：没有 FROZEN spec 时，**拒绝写实现代码**。只允许写 spec、跑评审、改 spec。哪怕用户说"先写个 demo 看看"也要先开 DRAFT。

### Step 1 · 需求梳理 → 原始需求

不直接动笔，先把需求拆成结构化清单。读 [`references/spec-template.md`](references/spec-template.md) 了解完整模板。

**至少要挖出**：
- 功能点清单（用户能看见的 1-2 级行为）
- 业务规则（隐含的约束，如"窗口不能被拉伸"）
- 边界场景（无 sprite、首次启动、多窗口……）
- 异常场景（动画文件丢失、Tauri 报错）

跟用户确认完清单后进入 Step 2。

### Step 2 · 写 Spec 文档

落到 `docs/specs/<NN>-<kebab-name>.md`，`<NN>` 按实现顺序递增（01, 02, …）。

**必填 frontmatter**：
```yaml
---
name: <一句话功能名>
status: DRAFT        # ← 唯一可变字段, 状态机驱动
created: 2026-06-06
owner: darven
related: []          # 关联的其他 spec / memory 模块
---
```

**正文 6 章节**（按 [`references/spec-template.md`](references/spec-template.md) 走）：
1. 功能清单
2. 业务规则
3. 接口契约（Tauri command / IPC event / 前端 method / 窗口操作…逐条列 path / method / 入参 / 出参 / 错误码）
4. 数据结构
5. 异常与边界
6. 验收用例（可勾选 checkbox 形式）

写完 → 改 frontmatter `status: DRAFT` → 提示用户"请审 / 让 Claude 评审"。

### Step 3 · Spec 评审（Claude 出意见，用户勾选）

读 [`references/review-checklist.md`](references/review-checklist.md) 三方面问题清单。

**Claude 主动按清单出评审意见**，每条格式：

```markdown
### 完整性
- [✓] 列举了 idle/walk/sleep 三种动画
- [✗] 缺少"动画切换时的过渡帧"行为
- [✓] ...

### 可落地性
- [✓] CSS animation API 可用
- [✗] `capabilities/default.json` 没声明 `core:window:allow-set-size`,无法改窗口大小

### 可测性
- [✓] 每条动画都有"加载后能否播放"的验收项
- [✗] 缺"窗口尺寸改了 sprite 还在原位"的回退用例
```

用户逐条 ✓/✗/讨论。**全部 ✓** → 改 `status: FROZEN`。**任一 ✗** → 回 `status: DRAFT`，回到 Step 2 改 spec。

### Step 4 · 开发实现（按 Spec 编码）

`status: FROZEN` 之后才允许动实现代码。规则：

- 接口签名、字段名、错误码、参数校验**逐条对齐 spec**——不新增不删减
- 真要新增/删减 → **先回 Step 2 改 spec 并重新评审**，不能"代码先动 spec 后补"
- 改完代码 → 改 `status: IMPLEMENTING`

### Step 5 · 测试验收（对照 Spec 验收）

读 spec 第 6 节"验收用例"，**逐条勾选**：
- ✓ 实现行为 = spec 描述
- ✗ 不一致 = bug，回到 `status: IMPLEMENTING` 修

**验收用例里没写的场景不需要测**——这是 spec-first 的好处（测试范围在 Step 2 就锁了）。

全部 ✓ → 改 `status: DONE`。

### Step 6 · 需求变更（迭代流程）

`DONE` 之后用户说"加个 / 改个"：

1. 把对应 spec 的 `status` 改为 `CHANGED`
2. 回 Step 2 改 spec（先标 `CHANGED`，改完标 `DRAFT`）
3. 走 Step 3 重评
4. 走 Step 4-5 改代码 + 验收

**禁止**"代码改了 spec 不动"。

---

## 强制规则速查

| 场景 | 允许 | 禁止 |
|---|---|---|
| 没 spec 写代码 | ✗ | ✓ |
| 写完代码后补 spec | ✗（硬拦截） | ✓ |
| FROZEN 后改 spec 不评审 | ✗ | ✓ |
| 代码超出 spec 范围 | ✗（先改 spec） | ✓ |
| 评审时跳过 ✗ 项 | ✗ | ✓ |
| 验收时跳过 ✗ 项 | ✗ | ✓ |

---

## 文件约定

- **Spec 文档**：`docs/specs/<NN>-<kebab-name>.md`，编号递增
- **Spec 模板/评审清单**：`references/` 内（跟 SKILL.md 同级）
- **关联引用**：spec 的 `related:` 字段填其他 spec 文件名或 memory 模块路径

---

## 关联资源

- [`references/spec-template.md`](references/spec-template.md) — spec 完整 6 章节模板
- [`references/review-checklist.md`](references/review-checklist.md) — 评审三方面问题清单

---

## 为什么不"先 demo 再说"

SDD 的价值是"让契约在写代码前就稳定"。先 demo 会发生：
- 改 demo → 改代码 → 改接口 → 改测试 → 改需求文档（5 处变动、4 处可能漏）
- 改 spec → 改代码 → 改测试（3 处变动、spec 是唯一事实源）

**为什么"宁严勿松"**：SDD 的"严"是为了"省"——前期 30 分钟写 spec，换后期几小时少返工。每次"算了直接写"都是把债往后推。
