---
name: gif-to-sheet
description: Convert a GIF animation into a horizontal sprite sheet PNG for the desktop pet. Use this skill whenever the user mentions adding a new animation, converting a GIF, creating a sprite sheet, or wants to add new pet behaviors/animations. Also trigger when the user provides a new GIF file for the pet, or asks about animation states, frame counts, or sprite parameters.
---

# GIF to Sprite Sheet Converter

This skill converts a source GIF into a horizontal sprite sheet PNG and provides the animation parameters needed by the Vue frontend.

## Prerequisites

- `ffmpeg` must be installed and available in PATH
- The project must be a Tauri + Vue desktop pet project with the standard sprite directory structure

## Naming convention

**Source GIFs in `raw/` keep their original (often Chinese) names, but the generated sprite sheet and all code references use an English identifier.**

Examples:
| Raw filename (user-facing) | English identifier (code-facing) |
|---|---|
| `捂鼻子.gif` | `idle` |
| `走路.gif` | `walk` |
| `睡觉.gif` | `sleep` |
| `吃东西.gif` | `eat` |

This means:
- `raw/` folder: Chinese or any non-ASCII names are fine
- `<english>_sheet.png` and all Vue/JS references: ASCII only

The mapping from Chinese → English is decided by you based on the GIF's content. If unsure, ask the user. Common mappings:
- 发呆 / 待机 / 静止 → `idle`
- 走 / 跑 / 移动 → `walk`
- 睡 / 躺 / 闭眼 → `sleep`
- 吃 / 喂食 → `eat`
- 捂鼻子 / 揉眼睛 / 任何小动作 → `idle` (or a custom name)

## Workflow

### Step 1: Find the source GIF

The user will either:
- Drop a GIF into `public/sprites/raw/` and tell you the name (likely Chinese)
- Tell you the original location of the GIF and you should move/copy it to `raw/`

If the file is in `raw/` with a Chinese name like `捂鼻子.gif`, the English identifier is the name **after** conversion — you should not rename the source. Instead, derive the English identifier from what the animation depicts.

### Step 2: Confirm or pick the English identifier

Look at the GIF (or trust the user's description) and pick an English identifier. If the user's intent is unclear, ask:
- "What should I call this animation in code? (e.g. idle, walk, sleep)"

### Step 3: Place the GIF in `raw/`

Ensure the source GIF is at `public/sprites/raw/<original-name>.gif`. Do NOT rename it to the English identifier — keep the original filename in `raw/`.

### Step 4: Run the conversion script with the English identifier

```bash
bash <skill-dir>/scripts/gif-to-sheet.sh <english-identifier> <source-gif-filename>
```

For example, if the user dropped `捂鼻子.gif` into `raw/` and it depicts an idle animation:
```bash
bash .claude/skills/gif-to-sheet/scripts/gif-to-sheet.sh idle 捂鼻子.gif
```

The script will:
1. Read `public/sprites/raw/<source-gif-filename>.gif`
2. Split it into frames
3. Stitch frames into `public/sprites/<english-identifier>_sheet.png`
4. Clean up temporary frames
5. Print animation parameters

### Step 5: Capture the output parameters

The script prints parameters like this:
```
=== 前端代码参数 ===
  帧数: 28
  单帧尺寸: 240x240
  FPS: 25
  动画时长: 1120ms
  sheet 路径: /sprites/<english-identifier>_sheet.png
```

Record these for the Vue animation setup.

### Step 6: Report results to the user

Tell the user:
- The sprite sheet was generated successfully at `public/sprites/<english>_sheet.png`
- The animation parameters (frame count, dimensions, FPS, duration)
- The source GIF was left untouched in `raw/`
- They can now use these values when adding the animation state to the Vue component

### Error handling

- **ffmpeg not found**: Tell the user to install ffmpeg (`sudo apt install ffmpeg` on Ubuntu/Debian)
- **GIF not found at expected path**: Ask the user to provide the correct file path or confirm they placed it in `raw/`
- **Script fails**: Show the error output and troubleshoot

## File structure

```
public/sprites/
├── raw/                              ← Source GIFs (keep original names, often Chinese)
│   ├── 捂鼻子.gif                   → maps to "idle"
│   ├── 走路.gif                     → maps to "walk"
│   └── ...
├── idle_sheet.png                   ← Generated sprite sheets (English, do not edit manually)
├── walk_sheet.png
└── ...
```

## How sprite sheets work in this project

Each sprite sheet is a single PNG with all animation frames arranged horizontally. For N frames of WxH pixels, the sheet is (N*W) x H pixels. The Vue component plays the animation by shifting `background-position-x` in CSS `steps(N)`.

Example for a 28-frame 240x240 animation at 25fps:
```css
.pet-sprite {
  width: 240px;
  height: 240px;
  background: url('/sprites/idle_sheet.png') no-repeat;
  background-size: 6720px 240px;
  animation: play 1120ms steps(28) infinite;
}

@keyframes play {
  from { background-position-x: 0; }
  to { background-position-x: -6720px; }
}
```
