---
name: Tauri v2 关窗权限缺失导致退出无效
description: 右键"退出"调 getCurrentWindow().close() 静默失败，因 capabilities 缺 core:window:allow-close
type: bug
---

# Tauri v2 关窗权限缺失导致退出无效

**发现时间**：2026-06-07
**触发位置**：`src/App.vue` 右键菜单"退出" → `getCurrentWindow().close()`
**关联文件**：`src-tauri/capabilities/default.json`

---

## 现象

用户右键菜单 → 点"退出"无反应，窗口不关闭。Console 没明显报错（或只看到 unhandled promise rejection warning）。

## 根因

Tauri v2 的 `core:window:default` 权限集合**不包含** `allow-close`（破坏性操作需显式授予）。原 capabilities：

```json
"permissions": [
  "core:default",
  "core:window:default",
  ...
]
```

`getCurrentWindow().close()` 内部抛 `NotAllowed` 异常，调用方没 `await` 也无 try/catch → 静默失败。

## 修复

1. **加权限**（`capabilities/default.json`）：
   ```json
   "core:window:allow-close"
   ```

2. **改调用方为 async + try/catch**（让错误可见）：
   ```ts
   onClick: async () => {
     try {
       await getCurrentWindow().close();
     } catch (e) {
       console.error("[PetExit] close failed:", e);
     }
   }
   ```
   原代码 `() => getCurrentWindow().close()` 是 fire-and-forget，Promise rejection 走 unhandled。

## 教训

**Tauri v2 破坏性 / 危险窗口操作需显式授权**。`core:window:default` 只覆盖安全的读 + 常用操作。`close` / `destroy` / `maximize` / `minimize` 这类要单独加 `core:window:allow-*`。Tauri v1 的 `tauri.conf.json > allowlist` 没有这个问题，迁移时容易踩坑。

**async Tauri API 必须 await 或 catch**。`getCurrentWindow().close()` / `invoke()` / `setSize()` 等都是 `Promise`，fire-and-forget 会让错误沉到 unhandled rejection 队列。

## 排查清单（关窗/退出类 bug）

1. capabilities 有没有 `core:window:allow-close`
2. 调用方有 await + try/catch 吗
3. `tauri.conf.json` 窗口的 `closable` 是不是 `false`（默认 true）
4. Rust 端有没有 `on_window_event` 拦截 `WindowEvent::CloseRequested`（已经查过，本项目没有）
5. devtools console 搜 `NotAllowed` / `[PetExit]`

## 相关提交

ee3d838 fix: right-click exit not working + simplify Mermaid diagram
