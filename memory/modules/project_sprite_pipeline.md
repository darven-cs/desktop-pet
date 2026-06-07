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

**当前实现**（App.vue 中由 `spriteStyle` computed 驱动）：
```ts
spriteStyle = {
  "--end-x": "-<frameCount × frameWidth>px",
  width: "<frameWidth>px",
  height: "<frameHeight>px",
  backgroundImage: "url('<sheetPath>')",
  backgroundSize: "<frameCount × frameWidth>px <frameHeight>px",
  animation: "play <frameCount/fps * 1000>ms steps(<frameCount>) <infinite|1>",
};
```

```css
@keyframes play {
  from { background-position-x: 0; }
  to   { background-position-x: var(--end-x); }
}
```

**关键技巧**：`--end-x` 用 CSS 变量 + v-bind，1 个 @keyframes 适配所有动画。

**多动画切换**：dispatch 改 state.current → computed 重新算 → 整个 style 一次更新。preload 在 watch 里完成（先缓存所有 sheet），切的时候零黑帧。

## 五、当前 sheet 元数据（来自 registry.rs）

| 动画 | 单帧 | 帧数 | FPS | 循环 |
|---|---|---|---|---|
| touch_nose | 240×240 | 28 | 25 | infinite |
| think | 155×155 | 26 | 25 | infinite |
| poop | 155×155 | 121 | 25 | once |
| shush | 120×120 | 2 | 50 | once |
| thumbs_up | 120×120 | 9 | 20 | once |
| nervous | 155×155 | 13 | 25 | infinite |
| sleep | 120×120 | 31 | 25 | infinite |
| peek | 120×120 | 57 | 20 | infinite |
| knead | 240×240 | 3 | 25 | infinite |
| heartbeat | 155×155 | 6 | 25 | infinite |
| cloud | 120×120 | 32 | 25 | infinite |

> 注意：`think_sheet.png` 是 4030÷155=**26** 帧（不是源 GIF 的 22 帧，ffmpeg 抽帧会插重复帧）；`poop_sheet.png` 是 121 帧（源 GIF 36 帧）。sheet 帧数才是播放真相。

## 六、相关记忆

- [窗口配置](project_window_setup.md) — 窗口尺寸 = 单帧尺寸
- [Rust 数据类型](project_rust_types.md) — registry 怎么算 frame_count

---

## 变更历史

- 2026-06-06：建模块。当前 3 个 sheet（touch_nose/think/poop），单帧 240×240 或 155×155
- 2026-06-06：CSS 改 v-bind + CSS 变量；元数据表记录 sheet 真实帧数 vs 源 GIF 帧数差异
- 2026-06-07：新增 8 个 sheet（shush/thumbs_up/nervous/sleep/peek/knead/heartbeat/cloud）。命名映射：阿马提拉斯→shush（捂嘴嘘）、点赞→thumbs_up、紧张→nervous、睡觉→sleep、偷看→peek、无聊踩奶→knead、心动→heartbeat、一切都是浮云→cloud。手势类（shush/thumbs_up）用 once，状态类用 infinite
