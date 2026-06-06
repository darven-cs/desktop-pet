---
name: 生产环境 sprites 目录找不到
description: 安装 .deb 后运行，Rust locate_sprites_dir() 只搜索 CWD 相对路径，找不到精灵图
type: project
---

# 生产环境 sprites 目录找不到

**状态**：已修复
**发现日期**：2026-06-07
**触发条件**：通过 .deb 安装后，从任意目录运行 `desktop-pet`

## 现象

```
[PetError] list_animations → E_FRAMES_MISSING: sprites dir not found: public/sprites
```

## 根因

`registry.rs:locate_sprites_dir()` 只搜索 CWD 相对路径（`public/sprites`、`../public/sprites`、`../../public/sprites`）和 ancestor 遍历。安装后的二进制从 `/usr/bin/desktop-pet` 运行，CWD 是用户当前目录，不会包含精灵图。

## 修复

1. **`tauri.conf.json`**：添加 `"resources": ["../public/sprites/*_sheet.png"]`，将精灵图打包进 Tauri 资源目录（Linux .deb 中位于 `/usr/lib/desktop-pet/`）
2. **`registry.rs:locate_sprites_dir()`**：新增 exe-relative 路径搜索，覆盖 Linux .deb/AppImage（`../lib/desktop-pet/`）和 macOS .app bundle（`../Resources/`）的常见资源目录布局

## 修复后搜索顺序

1. CWD 相对路径（dev 模式）
2. exe 相对路径（production）: `public/sprites` | `sprites` | `../lib/desktop-pet` | `../lib/desktop-pet/public/sprites` | `../lib/desktop-pet/sprites` | `../share/desktop-pet/sprites` | `../Resources` | `../Resources/public/sprites` | `../Resources/sprites`
3. CWD ancestor 遍历（兜底）
