---
name: Rust UTF-8 字符串切片 panic
description: 用硬编码字节索引切中文字符串导致 panic，应用 strip_prefix() 代替
type: bug
---

# Rust UTF-8 字符串切片 panic

**发现时间**：2026-06-07
**触发位置**：`src-tauri/src/lib.rs` → `send_message` / `get_chat_history`

---

## 现象

```
thread 'tokio-rt-worker' panicked at src/lib.rs:141:30:
start byte index 4 is not a char boundary; it is inside '户' (bytes 3..6 of string)
```

## 根因

两处命令对中文前缀字符串做了硬编码字节切片：

```rust
// BAD: "用户: " = 8 bytes, not 4
e.content[4..].to_string()
```

- `用` = 3 字节（E7 94 A8）
- `户` = 3 字节（E6 88 B7）
- `: ` = 2 字节

`[4..]` 切到了 `户` 的第 2 个字节位置，触发 UTF-8 边界 panic。

## 修复

用 `strip_prefix()` 替代手工数字节：

```rust
fn strip_conversation_prefix(content: &str) -> String {
    if let Some(rest) = content.strip_prefix("用户: ") {
        return rest.to_string();
    }
    if let Some(rest) = content.strip_prefix("宠物: ") {
        return rest.to_string();
    }
    content.to_string()
}
```

## 教训

**永远不要对非 ASCII 字符串用硬编码字节索引切片**。凡是涉及中文、emoji 等多字节字符的场景，用 `strip_prefix` / `strip_suffix` / `chars()` / `char_indices()` 等 Unicode-safe 方法。

## 再次发生

**2026-06-07（同日）**：同样 bug 再次出现在 `chat.rs:76` 和 `llm.rs:421` 的错误处理代码中：
```rust
// BAD: 截取错误日志时用字节索引
let raw = if trimmed.len() > 150 { &trimmed[..150] } else { trimmed };

// FIX: 用 chars().take() 确保字符边界安全
let raw: String = trimmed.chars().take(150).collect();
```
**教训加强**：不仅业务逻辑要避免字节切片，错误处理/日志截断等边缘代码同样要避免。全局搜索 `[..` 或 `[N..` 模式应成为修复后的检查步骤。
