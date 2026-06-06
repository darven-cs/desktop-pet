---
name: 桌面宠物窗口配置
description: 透明、无边框、置顶窗口的 Tauri v2 配置和 CSS 配合要点
type: project
---

# 桌面宠物窗口配置

**状态**：已上线
**上线时间**：2026-06
**所属业务**：核心基础设施

让 Tauri 窗口以"悬浮桌面宠物"形式呈现——没有系统边框、背景透明、始终在所有窗口之上、不出现在任务栏。

---

## 一、核心文件

| 文件 | 路径 | 职责 |
|---|---|---|
| Tauri 窗口配置 | `src-tauri/tauri.conf.json` | `app.windows[0]` 5 个关键 flag |
| 窗口权限 | `src-tauri/capabilities/default.json` | 声明前端可调用的窗口操作 |
| 透明度 CSS | `src/App.vue` 全局样式 + `index.html` body | 透出桌面的关键 |
| Tauri features | `src-tauri/Cargo.toml` | `tray-icon` / `devtools` 等 |

## 二、关键配置（不能漏）

**`tauri.conf.json` 的 `app.windows[0]`**：
```json
{
  "label": "main",
  "decorations": false,    // 关掉系统边框
  "transparent": true,     // 启用透明
  "alwaysOnTop": true,     // 置顶
  "resizable": false,      // 宠物不该被拉伸
  "skipTaskbar": true      // 不在任务栏显示
}
```

**`capabilities/default.json`**（拖拽/移动/置顶都要显式声明）：
```json
"permissions": [
  "core:default",
  "core:window:default",
  "core:window:allow-set-always-on-top",
  "core:window:allow-set-position",
  "core:window:allow-start-dragging",
  "core:window:allow-set-size"
]
```

**前端三处透明**（缺一就黑底）：
- `index.html` 的 `<body style="background: transparent;">`
- `src/App.vue` 全局 `html, body, #app { background: transparent; }`

## 三、尺寸约定

窗口尺寸 = 动画单帧尺寸。当前 240x240（与 `touch_nose_sheet.png` 一致）。后续若加新动画，**单帧尺寸不同就要改窗口尺寸**。

## 四、相关记忆

- [精灵图管线](project_sprite_pipeline.md) — 单帧尺寸的来源
- [Cargo 镜像坑](bug_cargo_crates_io_ssl.md) — 启动前必读

---

## 变更历史

- 2026-06-06：建模块。窗口 240x240，启用 transparent/decorations/alwaysOnTop/skipTaskbar
