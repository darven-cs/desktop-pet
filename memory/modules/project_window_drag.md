---
name: 窗口拖拽
description: 鼠标按住宠物在桌面上拖动窗口的实现方式
type: project
---

# 窗口拖拽

**状态**：已上线
**上线时间**：2026-06
**所属业务**：交互层

让用户用鼠标左键按住宠物任意位置拖动整个 Tauri 窗口在桌面上移动。

---

## 一、核心代码

**`src/App.vue` 的 `startDrag`**：
```ts
import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
  getCurrentWindow().startDragging();
});
```

挂在 `@mousedown` 上（不是 `mousedown.lazy`），左键 `e.button === 0` 才触发，避免右键菜单时误触。

**CSS 配合**：
```css
.pet-area { cursor: grab; }
.pet-area:active { cursor: grabbing; }
```

## 二、依赖的权限

`capabilities/default.json` 必须有 `core:window:allow-start-dragging`（参见 [窗口配置](project_window_setup.md)）。

## 三、已知坑

- **动态 import**：用 `import("@tauri-apps/api/window")` 异步加载而非顶部 import，避免 SSR 报错
- **右键冲突**：右键菜单的 `contextmenu` 事件记得 `preventDefault`，否则会触发系统菜单并吞掉点击

---

## 变更历史

- 2026-06-06：建模块。仅支持左键拖拽，右键菜单未实现
