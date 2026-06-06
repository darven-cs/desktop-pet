---
name: AI 桌宠长期目标
description: 项目的最终形态——一个有 LLM 决策 + 桌面感知 + 长期记忆的智能桌宠，所有 spec 设计都要为这个方向留接口
type: project
---

# AI 桌宠长期目标

**记录时间**：2026-06-06
**来源**：用户在 SDD 第一阶段主动分享

本项目的最终形态不是一个"会动的小图标"，而是一个 **LLM 驱动的智能桌宠**。当前所有 spec 在设计时必须考虑：为这个终态**留好接口**，不要让当前 spec 把路堵死。

---

## 终态能力清单

| 能力 | 说明 | 触发/输入 | 输出 |
|---|---|---|---|
| **LLM 决策** | 状态切换由 LLM 判断，不是硬编码规则 | 定时器 tick / 用户事件 | `{ nextState, reason, message? }` |
| **桌面感知** | 能看到屏幕内容（截屏/OCR/选区检测） | 系统级截屏 API | 桌面快照 / 选中文本 / 当前窗口 |
| **文字选区操作** | 用户左键长按拖动选中文字，桌宠弹出"翻译/解释/搜索"等操作 | 选区事件 | 操作菜单 |
| **对话问答** | 用户在输入框问问题，桌宠调 LLM 回答 | 文本输入 | LLM 响应（流式输出 + 表情/动作） |
| **长期记忆** | 记住用户偏好、历史对话、习惯 | 跨会话存储 | 检索/注入到 prompt |

---

## Spec 路线图（建议顺序）

> 当前 `01-pet-interaction-layer` 只做"基础设施",不接 LLM。但**接口要预留 LLM hook**。

| 编号 | 主题 | 依赖 | 核心新增 |
|---|---|---|---|
| **01** | 交互层（多动画+右键菜单+状态机+Rust IPC+窗口自适应） | — | 状态机接口、ticker、决策钩子 |
| 02 | LLM 状态决策 | 01 | 把 01 的决策钩子从 hardcoded 换成 LLM 调用；定时器驱动 |
| 03 | 桌面感知（截屏+窗口/选区） | 01 | 截屏 command、OS 级事件监听（macOS Accessibility / Windows UI Automation） |
| 04 | 文字选区操作 | 03 | 选区检测 + 弹操作菜单（翻译/解释/搜索） |
| 05 | 对话问答（输入+流式输出+动作反馈） | 02 | 输入 UI、LLM 流式响应、动作绑定到 LLM 决策 |
| 06 | 长期记忆 | 01/05 | 持久化（sqlite?）、向量检索、注入 prompt |

---

## 当前 spec 设计约束（01 必须遵守）

为后续 02/03/05 留接口:

- **F4 状态机**的"决策"必须是**可插拔函数**:
  ```ts
  type Decider = (context: Context) => Promise<Decision>
  // 当前实现: hardcoded 规则
  // 未来实现: LLM 调用的 wrapper
  ```
  不能把 if/else 写死在 state machine 里

- **F4 ticker** 频率必须可配（环境变量 / config / spec），不能写死 30s

- **Rust 端 command** 命名要稳定、能承载"上下文"入参：
  - `decide_next_state(context: serde_json::Value) → Decision`
  - 即使当前 impl 是占位返回,接口先定

- **AnimationEntry** 等结构放在 `src/types/` 或 `src-tauri/src/types.rs`,**前后端共用序列化定义**——后期 LLM 决策要传 pet 状态上下文,不能重复定义

---

## 跟当前 spec 的边界

- 01 不做: 截屏、LLM 调用、SQLite、输入 UI、选区检测
- 01 必须留: 决策钩子、ticker、context 序列化、stable command 命名

---

## 变更历史

- 2026-06-06：建模块。用户主动分享终态 → 整理为 6 个 spec 路线图
