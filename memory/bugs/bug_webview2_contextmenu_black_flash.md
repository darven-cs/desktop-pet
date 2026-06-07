---
name: WebView2 transparent + 右键闪黑框
description: Tauri v2 + transparent 窗口在 Windows 上右击时 WebView2 自己的 Chromium context menu 覆盖层会闪一下；自定义 in-webview <div> 菜单还会被 setSize 异步 + #app overflow:hidden 裁掉
type: bug
---

# Tauri v2 Windows transparent 窗口右键黑框 + 菜单不显示

**发现时间**：2026-06-07
**触发平台**：Windows（Linux / macOS 不复现）
**根因定位**：WebView2 Chromium 层 native context menu + Vue 菜单异步渲染裁剪

---

## 现象

Windows 用户右击 pet sprite：
1. 屏幕闪一下**黑底**覆盖层（半透明全屏黑）
2. 自定义菜单**完全没显示**或只显示一两个顶部选项
3. 用户主观感受："啥都没出来"

Linux / macOS 正常：WebKitGTK / WKWebView 直接把 contextmenu 交给 JS，无 native 层介入，preventDefault 干净利落。

---

## 根因（两层叠加）

### 1. WebView2 native context menu 闪烁（黑框主因）

Tauri v2 在 Windows 上用 **WebView2 (Chromium 内核)** 渲染。Chromium 处理 right-click 时，**先在 native 层绘制一个 context menu 覆盖层**（半透明黑底 + 上下文菜单项），然后才 forward 给 webview 触发 `contextmenu` 事件让 JS 决定。

即便 JS 立刻 `e.preventDefault()`，黑底已经画到屏幕上了，会闪几十毫秒消失。**纯前端拦不住**，因为这是 Chromium 自己 native 层的 UI 行为。

**对比**：
- **WebKitGTK (Linux)**：contextmenu 完全交给 JS，无 native 层，preventDefault 就彻底没了
- **WKWebView (macOS)**：类似 WebKitGTK，没这层 native 闪烁
- **WebView2 (Windows)**：Chromium 引擎自带的 context menu UI

### 2. 自定义菜单被 `#app { overflow: hidden }` 裁掉

`useContextMenu.open()` 是 async，**调用方没 await**：

```ts
function onContextMenu(e: MouseEvent) {
  ...
  menuOpen({ x: e.clientX, y: e.clientY }, items);  // ← fire-and-forget
}
```

`open()` 内部先 `await win.outerPosition()`（IPC 几十 ms），再设 `isOpen.value = true`。`isOpen` 变 true 后，watcher 触发 `await setWindowSize`（又一次 IPC）。

**整段有 50-100ms 延迟，期间窗口还是 sprite 大小（240×240）**。自定义菜单在 `position: fixed; left: e.clientX; top: e.clientY` 渲染后，向下/右延伸 130×162 像素，**超出窗口可视区**。`#app { overflow: hidden }` 把溢出部分裁掉。

**两边都受影响**（Linux / Windows 都裁），但：
- **Linux WebKitGTK 的 setSize 响应快**，几乎同步完成，用户感知不到被裁
- **Windows WebView2 的 setSize 慢且伴随透明丢失**，刚好和 native menu 闪烁叠加放大成"啥都没出来"

---

## 修复（彻底方案）

**改用 Tauri v2 原生菜单**（`tauri::menu::Menu` + `WebviewWindow::popup_menu`），菜单由 OS 层绘制，完全绕开 WebView2 的 context menu 路径。

### 改动 1：Rust 加 `show_context_menu` 命令（`src-tauri/src/lib.rs`）

```rust
use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem};
use tauri::Emitter;

const CTX_MENU_EVENT: &str = "context-menu-click";

#[tauri::command]
fn show_context_menu(app: AppHandle, window: WebviewWindow) -> Result<(), AppError> {
    let status = MenuItemBuilder::with_id("ctx.status", "宠物状态").build(&app)?;
    let settings = MenuItemBuilder::with_id("ctx.settings", "宠物设定").build(&app)?;
    // ...chat / memory / separator / exit
    let menu = Menu::with_items(&app, &[&status, &settings, &chat, &memory, &separator, &exit])?;
    window.popup_menu(&menu)?;  // 在光标位置弹原生菜单
    Ok(())
}

// 在 tauri::Builder::default() 上：
.on_menu_event(|app, event| {
    let id = event.id().0.clone();
    let _ = app.emit(CTX_MENU_EVENT, id);  // 转发给前端
})
```

### 改动 2：前端 `onContextMenu` 改 invoke + 监听事件

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const CTX_MENU_EVENT = "context-menu-click";
let unlistenMenu: UnlistenFn | null = null;

function onContextMenu(e: MouseEvent) {
  e.preventDefault();
  closeOverlay();
  invoke("show_context_menu").catch((err) => {
    console.error("[PetMenu] show_context_menu failed:", err);
  });
}

onMounted(async () => {
  unlistenMenu = await listen<string>(CTX_MENU_EVENT, (e) => {
    switch (e.payload) {
      case "ctx.status": showOverlay("status"); break;
      case "ctx.settings": showOverlay("settings"); break;
      case "ctx.chat": showOverlay("chat"); break;
      case "ctx.memory": showOverlay("memory"); break;
      case "ctx.exit":
        getCurrentWindow().close().catch(console.error);
        break;
    }
  });
});

onUnmounted(() => { unlistenMenu?.(); });
```

### 改动 3：删 Vue 端的 in-webview 菜单

- 删 `<div class="context-menu" v-if="menuIsOpen">` 整段
- 删 `.context-menu` / `.menu-item` / `.menu-separator` / `.submenu-*` / `.current-dot` CSS
- 删 `useContextMenu` composable 文件
- 删 `MENU_ITEM_H` / `MENU_PADDING_H` / `MENU_EST_WIDTH` 常量
- 删 `needWindowW` / `needWindowH` 里的菜单尺寸计算
- 删 `menuPixelVal` helper

---

## Tauri v2 菜单 API 要点

- `MenuItemBuilder::with_id(id, text).build(&app)` → `MenuItem`
- `PredefinedMenuItem::separator(&app)` → 分隔线
- `Menu::with_items(&app, &[&item1, &item2])` → 拼成菜单
- `WebviewWindow::popup_menu(&menu)` → 在**光标位置**弹
- `WebviewWindow::popup_menu_at(&menu, position)` → 在**指定位置**（相对窗口左上角）
- `tauri::Builder::on_menu_event(|app, event| { ... })` → 全局点击监听，`event.id()` 是 `MenuId`，`.0` 拿 String
- 前端用 `listen<T>("event-name", cb)` 接 `app.emit(name, payload)` 发的事件

muda 的 `MenuId` 是 `pub struct MenuId(pub String)`，所以 `event.id().0` / `event.id().as_ref()` 都能拿到底层 string。

`popup_menu` 内部会自动 dispatch 到 main thread（macOS 需要），调用方无需 `run_main_thread!`。

---

## 排查清单（Windows / 菜单类 bug）

1. **Tauri 配置**：窗口是不是 `transparent: true` + `decorations: false`？是 → WebView2 + Chromium 路径已开启
2. **菜单实现方式**：in-webview `<div>` vs Tauri native `tauri::menu`？
   - in-webview → 必踩 WebView2 黑框
   - Tauri native → 完全绕开，但失去自定义样式
3. **`onContextMenu.prevent` 位置**：挂在元素上（`.pet-area`）还是 document？元素级只覆盖元素内，document 级才是全窗口兜底
4. **菜单定位 + 窗口尺寸**：
   - 菜单 `position: fixed` + 窗口在 `setSize` 中间状态 → 可能被裁
   - `#app { overflow: hidden }` 包裹的菜单，溢出必裁
5. **setSize 是不是 await**：fire-and-forget 的 setSize 让窗口 resize 永远滞后于内容渲染

---

## 教训

- **WebView2 上的 right-click 是 native 行为**，in-webview 替代品都会有 native 闪烁。需求稳定就走 Tauri native menu。
- **Tauri v2 菜单 API 在 webview / 窗口 / 菜单 / 应用 4 个层级都有**，查 `tauri::menu::*` 模块即可，不需要第三方库。
- **Vue 端"自绘菜单"看着灵活**，实际是给 WebView2 / WebKitGTK 各自行为差异填坑，跨平台成本高。OS 原生菜单是更稳的选择。
- **不要再尝试用 `additional_browser_args` / `--disable-features=` 关 WebView2 context menu**，这条路不通（Chromium 没有这个开关）。
- **删代码时连带删 composable / 常量 / 样式**，不要留 dead code。
