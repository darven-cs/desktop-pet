---
name: 精灵图管线
description: GIF → sprite sheet 的目录约定、命名规则和播放原理
type: project
---

# 精灵图管线

**状态**：已上线
**上线时间**：2026-06
**所属业务**：核心基础设施

把 GIF 动画源文件转化为前端可播放的横排 sprite sheet，并约定好命名 / 路径 / 播放参数。

---

## 一、目录结构

```
public/sprites/
├── raw/                      ← 用户丢的原始 GIF（保留中文名）
│   ├── 捂鼻子.gif
│   ├── 思考.gif
│   └── 拉屎中.gif
├── touch_nose_sheet.png      ← 生成的 sprite sheet（英文名）
├── think_sheet.png
└── poop_sheet.png
```

**核心约定**：源文件用中文名（贴近用户），sheet 文件和代码引用一律英文（避免编码问题）。

## 二、命名映射（中文源 → 英文标识符）

由 Claude 根据 GIF 含义挑选：

| 中文含义 | 常用英文标识符 |
|---|---|
| 发呆/待机/静止 | `idle` |
| 走/跑 | `walk` |
| 睡 | `sleep` |
| 吃/喂食 | `eat` |
| 任何小动作（捂鼻子、揉眼…） | 自定义，如 `touch_nose` |

不确定时问用户。

## 三、生成流程

1. 源文件放进 `public/sprites/raw/<中文名>.gif`
2. 运行：`bash .claude/skills/gif-to-sheet/scripts/gif-to-sheet.sh <english> [源文件名]`
3. 脚本自动分解帧、拼横排 sheet、清理临时帧、输出帧数/尺寸/FPS
4. 输出的 `*_sheet.png` 直接被前端 `<img>` 或 `background-image` 引用

## 四、前端播放（CSS `steps()`）

```css
.pet-sprite {
  width: 240px;                              /* = 单帧宽 */
  height: 240px;                             /* = 单帧高 */
  background: url('/sprites/touch_nose_sheet.png') no-repeat;
  background-size: 6720px 240px;             /* = (帧数×单帧宽) × 单帧高 */
  animation: play 1120ms steps(28) infinite; /* 1120ms = 帧数×40ms (25fps) */
}
@keyframes play {
  from { background-position-x: 0; }
  to   { background-position-x: -6720px; }   /* 负的 (帧数-1)×单帧宽 ... 实际是 -总宽 */
}
```

**多动画切换**：通过 JS 改 `background-image` / `animation` 即可，无需新组件。

## 五、相关记忆

- [窗口配置](project_window_setup.md) — 窗口尺寸 = 单帧尺寸

---

## 变更历史

- 2026-06-06：建模块。当前 3 个 sheet（touch_nose/think/poop），单帧 240×240 或 155×155
