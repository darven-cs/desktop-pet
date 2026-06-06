#!/bin/bash
# GIF → Sprite Sheet 转换工具
# 用法: gif-to-sheet.sh <英文标识符> [源文件名]
# 例如: gif-to-sheet.sh idle 捂鼻子.gif
#      gif-to-sheet.sh walk 走路.gif
#
# 命名规则:
#   - raw/ 中的源 GIF 保留原名（通常是中文）
#   - 生成的 sprite sheet 和所有代码引用都用英文标识符
#
# 流程:
#   1. 读取 public/sprites/raw/<源文件名>.gif
#   2. 分解为帧
#   3. 拼接为横排 sprite sheet
#   4. 输出到 public/sprites/<英文标识符>_sheet.png
#   5. 删除临时帧文件
#   6. 输出帧数和尺寸信息（供前端代码使用）

set -e

SKILL_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PROJECT_ROOT="$(cd "$SKILL_DIR/../../.." && pwd)"
SPRITES_DIR="$PROJECT_ROOT/public/sprites"
RAW_DIR="$SPRITES_DIR/raw"

if [ -z "$1" ]; then
  echo "用法: $0 <英文标识符> [源文件名]"
  echo ""
  echo "  英文标识符: 用于代码和 sprite sheet 文件名 (如 idle, walk, sleep)"
  echo "  源文件名:   raw/ 中的原始 GIF 文件名（可省略，默认与英文标识符相同）"
  echo ""
  echo "可用的原始素材:"
  ls "$RAW_DIR"/*.gif 2>/dev/null | xargs -I{} basename {} | sed 's/^/  /'
  exit 1
fi

NAME="$1"
SOURCE_NAME="${2:-${NAME}}"
# 智能处理后缀：用户可能写也可能不写 .gif
case "$SOURCE_NAME" in
  *.gif) INPUT="$RAW_DIR/$SOURCE_NAME" ;;
  *)     INPUT="$RAW_DIR/${SOURCE_NAME}.gif" ;;
esac
OUTPUT="$SPRITES_DIR/${NAME}_sheet.png"
TEMP_DIR=$(mktemp -d)

if [ ! -f "$INPUT" ]; then
  echo "错误: 找不到 $INPUT"
  echo "请先将 GIF 文件放到 $RAW_DIR/ 下"
  exit 1
fi

echo "=== 转换 ==="
echo "英文标识符: $NAME"
echo "源文件:     $SOURCE_NAME"
echo "输入:       $INPUT"

# 获取 GIF 信息
INFO=$(ffmpeg -i "$INPUT" 2>&1 || true)
DIMENSIONS=$(echo "$INFO" | grep -oP '\d{2,}x\d{2,}(?=,)' | head -1)
WIDTH=${DIMENSIONS%x*}
HEIGHT=${DIMENSIONS#*x}
FPS=$(echo "$INFO" | grep -oP '\d+(?= fps)' | head -1)

if [ -z "$WIDTH" ] || [ -z "$HEIGHT" ]; then
  echo "错误: 无法解析 GIF 尺寸"
  exit 1
fi

echo "尺寸: ${WIDTH}x${HEIGHT}"

# 分解帧
ffmpeg -y -vsync 0 -i "$INPUT" "$TEMP_DIR/frame_%04d.png" 2>/dev/null

# 统计帧数
FRAME_COUNT=$(ls "$TEMP_DIR"/frame_*.png | wc -l)
echo "帧数: $FRAME_COUNT"

# 拼接为 sprite sheet
ffmpeg -y -i "$TEMP_DIR/frame_%04d.png" -filter_complex "tile=${FRAME_COUNT}x1" \
  -frames:v 1 -update 1 "$OUTPUT" 2>/dev/null

SHEET_WIDTH=$((FRAME_COUNT * WIDTH))

echo "输出: $OUTPUT"
echo "Sheet 尺寸: ${SHEET_WIDTH}x${HEIGHT}"
echo ""
echo "=== 前端代码参数 ==="
echo "  帧数: $FRAME_COUNT"
echo "  单帧尺寸: ${WIDTH}x${HEIGHT}"
echo "  FPS: ${FPS}"
echo "  动画时长: $(echo "scale=0; $FRAME_COUNT * 1000 / $FPS" | bc)ms"
echo "  sheet 路径: /sprites/${NAME}_sheet.png"
echo ""

# 清理临时文件
rm -rf "$TEMP_DIR"

echo "=== 完成 ==="
