# 当前状态

**更新时间**：2026-06-07 14:00

---

## 正在做

- **任务**：v0.2.0 已发布（8 个新动画 + spec 05 + 修复退出/Mermaid 后的 tag 移动）；CI 正在重跑 release
- **下一动作**：等 GitHub Actions v0.2.0 build 完，检查 release assets 时间戳是否更新

## 下一步

1. 02/05/06 spec AC 验收通过 → FROZEN → DONE
2. 按需开 03 (桌面感知) 或打磨现有功能

## 已落地的产物

### Rust 后端
- `src-tauri/src/types.rs` — 共享 serde 结构（AnimationEntry / Decision / AppError 等，02 扩展了 DecisionContext）
- `src-tauri/src/registry.rs` — list_animations + is_known_animation
- `src-tauri/src/decider.rs` — async LLM 调用（OpenAI 兼容 API）+ fallback Stay
- `src-tauri/src/llm.rs` — OpenAI HTTP 客户端（静态 env 配置 + 运行时 DecisionContext 覆盖）
- `src-tauri/src/lib.rs` — 5 个 Tauri command（greet / list_animations / decide_next_state / get_llm_info / update_llm_config）

### 前端
- `src/types/pet.ts` — TS 类型镜像（02 扩展）
- `src/decider/index.ts` — Decider 接口 + getDefaultDecider
- `src/composables/useAnimationRegistry.ts` — 启动调 list_animations 缓存
- `src/composables/useAnimationStateMachine.ts` — 纯函数 reducer + dispatch + ticker + recentHistory ring buffer
- `src/composables/useContextMenu.ts` — 右键菜单（支持 separator / submenu）
- `src/composables/usePetSettings.ts` — 宠物设定 + localStorage 持久化
- `src/composables/usePetChat.ts` — 对话状态管理（消息列表 + sendMessage + 剪贴板上下文）
- `src/components/PetStatusPanel.vue` — 状态 overlay
- `src/components/PetSettingsPanel.vue` — 设定 overlay（LLM 开关 / API 地址 / API Key / 模型 / Ticker / 人格）
- `src/components/PetChatPanel.vue` — 对话 overlay（消息气泡 + typing 动画）
- `src/components/PetMemoryPanel.vue` — 记忆 overlay（类型筛选 + 时间倒序 + importance 星标）
- `src/App.vue` — 主组件（右键菜单 + overlay + 动态窗口尺寸 + 左键短按读剪贴板）

### 配置
- `.env.example` — LLM 环境变量模板

### Chat + Memory（新建）
- `src-tauri/src/memory/mod.rs` — MemoryManager 统一入口（record / build_context / get_memories）
- `src-tauri/src/memory/types.rs` — MemoryEntry / MemoryKind / ChatMessage / ChatResponse
- `src-tauri/src/memory/short_term.rs` — 短期记忆：VecDeque ring buffer（100 条/2 小时）
- `src-tauri/src/memory/long_term.rs` — 长期记忆：JSON 持久化 + 关键词检索 + pruning
- `src-tauri/src/memory/retrieval.rs` — 检索：build_memory_context() 注入 LLM prompt
- `src-tauri/src/memory/digest.rs` — 消化引擎：should_digest / build_digestion_prompt / parse_digestion_response
- `src-tauri/src/chat.rs` — 对话处理 + read_clipboard_text()

### Specs
- `docs/specs/01-pet-interaction-layer.md` — DONE
- `docs/specs/02-llm-decision.md` — DRAFT（代码已实现）

## 路线图全景

| Spec | 状态 | 备注 |
|---|---|---|
| 01 宠物交互层 | DONE | 3 个动画播放正常，验收通过 |
| 02 LLM 状态决策 | DRAFT | 代码实现完成，待验收 |
| 03 桌面感知 | 待 02 后 | 截屏 / OS 事件 |
| 04 文字选区操作 | 待 03 后 | 翻译/解释/搜索 |
| 05 对话问答 | DRAFT | 代码已实现（chat.rs + PetChatPanel + usePetChat + 左右键入口），待验收 |
| 06 长期记忆 | DRAFT | 代码已实现（5 层记忆架构），待验收 |

## 最近会话

- [session-2026-06-06.md](session-2026-06-06.md) — SDD 流程固化 + 01 spec 冻结 + 实现 + 验收
- [session-2026-06-07.md](session-2026-06-07.md) — 02 spec + LLM 后端 + 右键菜单重构 + overlay 面板
