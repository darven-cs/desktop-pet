# 🦊 Desktop Pet

AI 桌面宠物 — 基于 **Tauri v2 + Vue 3 + TypeScript** 的透明、无边框、置顶桌面精灵。

## 特性

- **LLM 自主决策** — 通过 OpenAI 兼容 API（支持 DeepSeek / OpenAI / Ollama）驱动宠物行为
- **ReAct 范式** — 记忆上下文 → 推理 → 工具调用 → 行动
- **工具系统** — `get_current_time` / `switch_animation` / `speak_to_user`，统一 OpenAI function calling
- **5 层记忆** — 事件流 → 短期记忆 ring buffer → 消化引擎 → 长期记忆 JSON 持久化 → 检索注入
- **对话问答** — 右键菜单 / 左键短按（读剪贴板）打开对话面板
- **主动对话** — 宠物可自主发起聊天，多次对话自动追加
- **宠物名称/人格** — 可自定义名称和性格，注入 LLM 系统提示

## 运行

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 生产构建
npm run tauri build
```

### 环境变量

复制 `.env.example` 为 `.env` 并配置 API：

```env
LLM_API_KEY=sk-your-key-here
LLM_API_ENDPOINT=https://api.openai.com/v1
LLM_MODEL=gpt-4o-mini
```

支持任何 OpenAI 兼容 API（DeepSeek / Ollama / vLLM 等）。

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri v2 (Rust) |
| 前端 | Vue 3 + TypeScript + Vite |
| LLM | OpenAI 兼容 API（function calling） |
| 记忆 | STM: VecDeque ring buffer / LTM: JSON 持久化 |
| 状态机 | 纯函数 reducer（playing / idle / transitioning） |

## 跨平台构建

GitHub Actions 自动构建 Linux (.deb/.AppImage)、macOS (.dmg/.app)、Windows (.msi/.exe)：

[![Build](https://github.com/darven-cs/desktop-pet/actions/workflows/build.yml/badge.svg)](https://github.com/darven-cs/desktop-pet/actions/workflows/build.yml)
