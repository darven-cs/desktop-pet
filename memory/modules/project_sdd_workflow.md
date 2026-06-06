---
name: SDD 工作流（Spec-Driven Development）
description: 本项目所有新功能开发必须走 Spec 先行 → 评审冻结 → 实现 → 验收 流程，对应 sdd skill
type: project
---

# SDD 工作流

**状态**：已上线
**上线时间**：2026-06-06
**所属业务**：工程方法论（横切所有功能模块）

本项目的**新功能/新行为**开发流程。Spec 是产品/开发/测试唯一契约，代码严格派生自 Spec。

---

## 一、SOP 速查

| 阶段 | 动作 | 状态 | 产出物 |
|---|---|---|---|
| 1 | 需求梳理 | — | 功能点/业务规则/边界/异常 清单 |
| 2 | 写 Spec | `DRAFT` | `docs/specs/<NN>-<name>.md` |
| 3 | 评审 | `REVIEW` | 三方面 ✓/✗ 意见 |
| 4 | 冻结 | `FROZEN` | spec 锁定 |
| 5 | 实现 | `IMPLEMENTING` | 代码（严格对齐 spec） |
| 6 | 验收 | `TESTING` → `DONE` | 逐条 AC ✓/✗ |
| 变更 | 改 spec | `CHANGED` → `DRAFT` | 走 2-6 重做 |

**硬规则**：没 FROZEN spec 就不许写实现代码（sdd skill 硬拦截）。

---

## 二、文件约定

- Spec 文档：`docs/specs/<NN>-<kebab-name>.md`（编号递增）
- Spec frontmatter：`status: DRAFT|REVIEW|FROZEN|IMPLEMENTING|TESTING|DONE|CHANGED`
- 模板与评审清单：`.claude/skills/sdd/references/`

---

## 三、跟其他模块的关系

- **窗口配置**（`project_window_setup.md`）— 实现时窗口尺寸/permission 约束的来源
- **精灵图管线**（`project_sprite_pipeline.md`）— 实现时 sprite 路径/单帧尺寸的来源
- **窗口拖拽**（`project_window_drag.md`）— 已有行为，外部 spec 不变

任何新功能 spec 都要 `related:` 引用上述模块。

---

## 四、什么**不**走 SDD

| 场景 | 走什么 |
|---|---|
| 修 bug（不改外部行为） | 直接改 |
| 重构（不改外部行为） | 直接改 |
| 加 sprite 资源 | `gif-to-sheet` skill |
| 改 CLAUDE.md / 维护 memory | 直接改 |

判断不确定时倾向走 SDD（"宁严勿松"）。

---

## 五、相关链接

- Skill 定义：[`.claude/skills/sdd/SKILL.md`](../../.claude/skills/sdd/SKILL.md)
- Spec 模板：[`.claude/skills/sdd/references/spec-template.md`](../../.claude/skills/sdd/references/spec-template.md)
- 评审清单：[`.claude/skills/sdd/references/review-checklist.md`](../../.claude/skills/sdd/references/review-checklist.md)
- Spec 目录：[`docs/specs/`](../../docs/specs/)

---

## 变更历史

- 2026-06-06：建模块。固化 SDD 5+1 步流程为 `sdd` skill + 状态机 + 硬拦截
- 2026-06-06：01 spec 走完 DRAFT → REVIEW → FROZEN → IMPLEMENTING → TESTING（首跑流程）
