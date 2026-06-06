# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Desktop pet application built with **Tauri v2 + Vue 3 + TypeScript**. A transparent, frameless, always-on-top window displays an animated pet sprite on the desktop. The window can be dragged around and will support multiple animation states and interactions.

## Development Commands

```bash
# Development (starts Vite dev server + Rust backend)
npm run tauri dev

# Production build
npm run tauri build

# Frontend only (no Tauri window)
npm run dev

# Type check frontend
vue-tsc --noEmit

# Convert a GIF to sprite sheet (requires ffmpeg, see .claude/skills/gif-to-sheet skill)
bash .claude/skills/gif-to-sheet/scripts/gif-to-sheet.sh <english-name> [source-gif-filename]
```

## Architecture

- **Frontend** (`src/`): Vue 3 SPA with `<script setup>` composition API. Vite dev server on port 1420. The transparent window renders pet sprites using CSS `background-position` + `animation: steps()` to play sprite sheet frame-by-frame.
- **Backend** (`src-tauri/`): Rust/Tauri v2. Window is configured as transparent, undecorated, always-on-top, skip-taskbar. Tauri commands handle window control (dragging, position). The Rust lib is `desktop_pet_lib`.
- **Config**: Window properties are in `tauri.conf.json` under `app.windows[0]`. Permissions for window operations are in `src-tauri/capabilities/default.json`.
- **Sprites**: `public/sprites/raw/` holds source GIFs, `public/sprites/*_sheet.png` are generated sprite sheets (horizontal strip of all frames). Never edit generated sheets directly — use the conversion script.

## Key Tauri v2 Notes

- Cargo mirror is configured in `~/.cargo/config.toml` (uses rsproxy sparse registry for China network)
- Window transparency requires `transparent: true` in tauri.conf.json AND `background: transparent` on html/body/#app in CSS
- Window dragging uses `getCurrentWindow().startDragging()` from `@tauri-apps/api/window`
- Tauri features enabled: `tray-icon`, `devtools`

<!-- MEMORY-SYSTEM:BEGIN -->
## 记忆系统使用规范

### 记忆文件位置
- 总索引：`memory/_index.md`（始终加载）
- 详细记忆存放目录：`/memory/`
- 系统教程：`memory/_tutorial.md`（扩展用法/套用到新项目）

### 你必须遵守的记忆规则

**启动协议（强制）：在执行任何代码修改前，必须完成以下步骤：**

1. 扫描 `memory/_index.md`，找到与本次修改相关的条目（关键词取自即将修改的文件名、模块名、涉及的功能域）
2. 将相关条目中的关键约束列出来，得到用户确认后才能开始写代码
3. 修改完成后，按本规范更新相关记忆文件

> 此协议由 `memory-preflight` hook 在运行时自动提醒，但你必须内化为习惯——不要等 hook 提醒才做。

**日常规则：**

1. **开始任务前**，先快速扫描 `memory/_index.md`，确认涉及模块的当前状态和已知约束。
2. **任何对模块的变更**（新增功能、修改接口、改变依赖），在任务完成后，你必须：
   - 在对应模块的记忆文件中追加一条变更历史（日期 + 简要说明）
   - 如果模块公开接口变化，更新公开接口表格
   - 如果模块状态、最后更新日期改变，**同步更新 `memory/_index.md` 中的对应条目**
3. **当用户说"记一下"**，意味着当前变更点很重要，立即执行上述更新。
4. **会话结束时**，可选追加 `memory/progress/session-YYYY-MM-DD.md`，并更新 `progress/current-status.md`。
5. **新增模块**时，自动创建模块记忆文件，并在 `memory/_index.md` 中添加一行索引。
6. **新发现通用踩坑**，按"跨模块 + 代码看不出原因"标准判断是否进 `memory/bugs/`。

### 记忆系统的自动化
- 项目 `.claude/settings.json` 配置了两个 hook：
  - **PreToolUse**（`memory-preflight.mjs`）：修改非 memory/ 文件前，自动扫描 `_index.md` 中的相关条目，输出关键约束提醒
  - **PostToolUse**（`sync-memory-index.mjs`）：修改 memory/ 文件后，自动把 `_index.md` 对应条目的"最后更新"列同步为今天日期
- 如果 hook 没生效，检查 `.claude/hooks/` 下对应脚本是否存在并可执行。
<!-- MEMORY-SYSTEM:END -->
