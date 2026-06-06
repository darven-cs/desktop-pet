---
name: Chat + Memory 系统
description: 对话系统（左右键入口+剪贴板上下文）+ 5 层记忆架构（STM/LTM/消化/检索），模块化设计
type: project
---

# Chat + Memory 系统

**状态**：已上线
**上线时间**：2026-06-07
**所属业务**：05 对话问答 + 06 长期记忆

---

## 一、架构

```
Layer 5: Retrieval（检索）→ build_memory_context() 注入 LLM prompt
Layer 4: Long-Term Memory → JSON 文件持久化（~/.local/share/desktop-pet/memory.json）
Layer 3: Digestion（消化）→ LLM 定期反思产出摘要（代码就绪，待触发调度）
Layer 2: Short-Term Memory → VecDeque ring buffer（100 条/2 小时）
Layer 1: Event Stream → MemoryManager.record() 统一记录入口
```

## 二、Rust 模块

```
src-tauri/src/
├── memory/
│   ├── mod.rs          # MemoryManager: record / build_context / get_memories / digest
│   ├── types.rs        # MemoryEntry, MemoryKind, ChatMessage, ChatResponse
│   ├── short_term.rs   # VecDeque ring buffer, auto-evict
│   ├── long_term.rs    # JSON 读写 + 关键词检索 + pruning
│   ├── retrieval.rs    # build_memory_context: STM 对话/事件 + LTM top-5
│   └── digest.rs       # should_digest / build_digestion_prompt / parse_digestion_response
├── chat.rs             # send_chat_message: 构建 chat prompt + 调 LLM + read_clipboard_text
```

## 三、Tauri Commands

| Command | 用途 |
|---|---|
| `send_message(text, context_text)` | 发对话 → LLM 回复 + 记忆记录 |
| `get_chat_history(limit)` | 获取对话历史 |
| `get_memories(kind, limit)` | 获取记忆列表（按类型过滤） |
| `record_interaction(kind, content)` | 前端上报事件 |
| `read_clipboard()` | 读取系统剪贴板（Linux: xclip/xsel） |

## 四、前端

- `usePetChat.ts` — 消息列表 + sendMessage + 从 localStorage 读 API 配置
- `PetChatPanel.vue` — 对话 overlay（消息气泡 + 输入框 + typing 动画）
- `PetMemoryPanel.vue` — 记忆浏览器（按类型筛选 + 时间倒序 + importance 星标）

## 五、对话入口

1. **右键菜单** → 「宠物对话」 → 打开 PetChatPanel
2. **左键短按宠物**（<300ms + <5px 位移） → 读剪贴板 → 自动填入/发送

## 六、相关记忆

- [Rust 数据类型](project_rust_types.md)
- [前端组合式](project_frontend_composables.md)
- [AI 桌宠长期目标](project_ai_pet_vision.md)

---

## 变更历史

- 2026-06-07：建模块。6 个 memory/ 子模块 + chat.rs + 前端 PetChatPanel/usePetChat + PetMemoryPanel 改造。消化引擎代码就绪，待 ticker 中调度触发。
- 2026-06-07（同日二轮）：Memory Fix — 决策流注入记忆上下文、LTM 自动持久化（record 时同步写磁盘）、宠物名称贯穿全链路（settings → ctx → prompt）、PetChatPanel watch 追加多次主动对话、系统提示增加记忆参考规则。简易消化策略：Conversation（≥0.3）/ Decision（≥0.5）自动提升到 LTM。
