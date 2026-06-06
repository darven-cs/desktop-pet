# 项目记忆索引

> 本索引始终加载。查找模块/问题/进度时先扫本文件，按条目跳到详细记忆。保持索引轻量——详情永远在子文件里。

> 📘 第一次接触这套记忆系统、或想复制到其他项目，看 [`_tutorial.md`](_tutorial.md)。

---

## 一、模块记忆 (`./modules/`)

### 1.1 已有模块

<!-- INDEX:MODULES:START -->
| 模块 | 路径 | 状态 | 最后更新 | 一句话 |
|---|---|---|---|---|
| SDD 工作流 | [modules/project_sdd_workflow.md](modules/project_sdd_workflow.md) | 已上线 | 2026-06-06 | 所有新功能必须走 Spec 先行 → 评审冻结 → 实现 → 验收，对应 sdd skill |
| AI 桌宠长期目标 | [modules/project_ai_pet_vision.md](modules/project_ai_pet_vision.md) | 路线图 | 2026-06-07 | LLM 决策+桌面感知+文字选区+对话+记忆；6 个 spec 路线图 |
| 01 宠物交互层 | [docs/specs/01-pet-interaction-layer.md](../../docs/specs/01-pet-interaction-layer.md) | DONE | 2026-06-06 | F1-F6 全部实现并验收通过：状态机+Rust IPC+右键菜单+窗口自适应+3 动画播放正常 |
| 02 LLM 状态决策 | [docs/specs/02-llm-decision.md](../../docs/specs/02-llm-decision.md) | DRAFT | 2026-06-07 | OpenAI 兼容 API 替换占位 decider + 右键菜单重构为宠物管理中心 + overlay 面板 |
| 05/06 Chat + Memory | [modules/project_chat_memory.md](modules/project_chat_memory.md) | 已上线 | 2026-06-07 | 对话系统+5 层记忆+LTM 持久化+ReAct 决策+工具系统+宠物名称 |
| 工具系统 | [modules/project_tools.md](modules/project_tools.md) | 已上线 | 2026-06-07 | ToolRegistry 统一管理 get_current_time / switch_animation / speak_to_user + 终端/非终端工具 |
| 窗口配置 | [modules/project_window_setup.md](modules/project_window_setup.md) | 已上线 | 2026-06-06 | 透明无边框置顶窗口的 Tauri 5 个 flag + 3 处 CSS；窗口尺寸运行时动态 |
| 精灵图管线 | [modules/project_sprite_pipeline.md](modules/project_sprite_pipeline.md) | 已上线 | 2026-06-06 | GIF → sprite sheet 的目录约定、命名映射、CSS steps 播放 |
| 窗口拖拽 | [modules/project_window_drag.md](modules/project_window_drag.md) | 已上线 | 2026-06-06 | 鼠标左键按住拖动整个 Tauri 窗口 |
| Rust 数据类型 | [modules/project_rust_types.md](modules/project_rust_types.md) | 已上线 | 2026-06-07 | types.rs 定义所有 serde 结构，前后端共用 R7 契约 |
| 前端组合式 | [modules/project_frontend_composables.md](modules/project_frontend_composables.md) | 已上线 | 2026-06-07 | useAnimationRegistry / useAnimationStateMachine / useContextMenu / usePetSettings / usePetChat + decider/ |
<!-- INDEX:MODULES:END -->

### 1.2 参考类记忆

<!-- INDEX:REFS:START -->
| 条目 | 路径 | 一句话 |
|---|---|---|
<!-- INDEX:REFS:END -->

---

## 二、踩过的坑 (`./bugs/`)

<!-- INDEX:BUGS:START -->
| 坑 | 路径 | 触发条件 | 一句话规则 |
|---|---|---|---|
| cargo 国内 SSL | [bugs/bug_cargo_crates_io_ssl.md](bugs/bug_cargo_crates_io_ssl.md) | 国内网络首次 cargo build | `~/.cargo/config.toml` 用 sparse+https://rsproxy.cn/index/ |
| UTF-8 切片 panic | [bugs/bug_rust_utf8_slice_panic.md](bugs/bug_rust_utf8_slice_panic.md) | 硬编码字节索引切中文前缀/日志截断 | 用 `strip_prefix()` 和 `chars().take()` 代替字节索引 |
<!-- INDEX:BUGS:END -->

---

## 三、当前进度 (`./progress/`)

每次会话结束时追加 `session-YYYY-MM-DD.md`，并更新 `current-status.md`。

**当前状态：** 目录暂空。

---

## 使用规则

1. **开始任务前**：扫描本文件，确认涉及模块的当前状态与已知约束，再决定是否深入读子文件。
2. **修改模块后**：在对应 `modules/*.md` 追加变更历史（日期 + 简要说明），并同步更新本索引的"最后更新"列与"一句话"。
3. **公开接口变化**：必须更新 `modules/*.md` 里的接口表。
4. **新增通用坑**：在 `bugs/` 下新建 `{技术栈}_{现象}.md`，本索引的坑表追加一行。
5. **新增模块**：
   - 内容不多（一页内说完）→ 单文件 `modules/{type}_{name}.md`
   - 内容多 → 子目录 `modules/{name}/` 含 overview/design/integration/known_issues
6. **遗留问题**：模块内部记在 `modules/{模块}/known_issues.md`；跨模块通用坑放 `bugs/`。

---

## 模块记忆文件结构约定

大模块子目录下四个文件各自回答一个问题：

| 文件 | 回答什么问题 |
|---|---|
| `overview.md` | 这个模块有什么类/接口/数据？新人花 5 分钟读完能上手吗？ |
| `design.md` | 为什么这样做？改掉某条之前需要注意什么？ |
| `integration.md` | 和项目其他部分怎么对接？新人复用这套接入路径要抄哪几个点？ |
| `known_issues.md` | 还有哪些未修的坑，以及修的建议方向 |

**如果某个文件内容少于半屏，合并到 overview.md 里即可**——不要为了形式制造空文件。
