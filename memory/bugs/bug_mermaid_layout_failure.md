---
name: GitHub Mermaid 图渲染失败
description: 复杂 Mermaid 图报 "Could not find a suitable point for the given distance"，节点/标签过多导致 layout 算不出坐标
type: bug
---

# GitHub Mermaid 图渲染失败

**发现时间**：2026-06-07
**触发位置**：README.md 的架构图
**报错**：`Could not find a suitable point for the given distance`

---

## 现象

GitHub README 里 Mermaid 图直接渲染不出来，错误信息：

```
Unable to render rich display

Could not find a suitable point for the given distance
```

## 根因

Mermaid 的 layout engine（ELK / dagre）在以下条件时算不出节点坐标：

- **节点太多 / 边太密**（图大）
- **label 太长**（含 `<br/>`、特殊字符、括号）
- **subgraph 嵌套深**（多层 subgraph）
- **label 里含 `/`、`<br/>`、`·` 等**（layout 计算复杂）

原图 14 节点 + 20 边 + 3 层 subgraph + label 含 `<br/>` 和 `·`，超出 ELK 算力。

## 修复

**三步组合**：

1. **`graph TB` → `flowchart LR`**：横向布局，节点排得开
2. **去掉 `<br/>` 和 `·`**：单行 label，layout 算得动
3. **缩短 label**：每节点 ≤ 12 字符、每边 ≤ 8 字符

修复后图（11 节点 + 14 边，2 层 subgraph，全单行 label）正常渲染。

## 教训

**Mermaid 图保持"小而清"**。GitHub 默认 renderer 是 ELK，节点超过 ~12 个、label 超过 2 行就可能炸。**三招救命**：

| 招 | 适用 | 例子 |
|---|---|---|
| `flowchart LR` | 节点多 / 边多 | 横向排比纵向稀 |
| 拆成多张小图 | 大图 | 一张 frontend，一张 backend |
| 回退 ASCII | 实在画不动 | box-drawing 字符 |

**label 字符安全清单**：避免 `<br/>` / `<br>` / `·` / `→` / 长 URL。括号和斜杠 OK 但 ≤20 字符。

## 替代方案

如果 Mermaid 实在搞不定，三个 fallback：

1. **ASCII box-drawing**（项目之前用的那种）— 100% 不会渲染失败，缺点是占地方
2. **draw.io / Excalidraw 导出 PNG** — 推 PNG 到 repo，引用 `![](docs/arch.png)`
3. **多张小 Mermaid** — 每张 5-6 节点，串成节

## 相关提交

ee3d838 fix: right-click exit not working + simplify Mermaid diagram（同时改了）
