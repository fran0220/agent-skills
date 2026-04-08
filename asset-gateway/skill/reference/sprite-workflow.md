# Sprite Animation Workflow

一条命令生成精灵动画：文本/图片 → Grok 视频生成（2 秒） → ffmpeg 逐帧提取 → 白色背景移除 → spritesheet/GIF。

## 概览

SpriteForge 基于 **xAI Grok 视频生成**（`grok-imagine-video`），直接从文本或参考图生成动画视频，再通过后处理提取帧并输出 spritesheet 或 GIF。核心流程：

1. **输入** → 文本描述（`--prompt`）+ 可选参考图（`--input`）
2. **Grok 视频生成** → 生成 2 秒动画视频（默认 8fps = 16 帧，一个完整动画循环）
3. **帧提取** → ffmpeg 按指定帧率从视频中提取帧序列
4. **后处理** → 白色背景移除 → 输出透明背景 spritesheet PNG 或 animated GIF

> 与旧版 Gemini 网格图方案不同，SpriteForge 现在**直接生成视频**——动画流畅度和角色一致性大幅提升。无需 LLM prompt 增强，prompt 直接传给 Grok。

## 基本用法（Text-to-Sprite）

```bash
asset-gateway generate sprite \
  --prompt "pixel art knight in silver armor with blue cape" \
  --animation-type walk \
  --output-dir ./sprites
```

输出：`sprites/sprite_<timestamp>.png` — 水平排列的 spritesheet（16 帧透明背景 PNG）。

**关键点：**
- `--prompt` 描述**角色外观**，不描述动作序列
- `--animation-type` 控制动画类型（walk、run、attack 等）
- 无需 LLM 增强，prompt 直接发送给 Grok 视频模型
- 生成耗时约 **15-20 秒**

## 使用参考图（Image-to-Sprite）

传入参考图保持角色外观一致性，适合为同一角色生成多套动画：

```bash
# 用已有角色图生成走路动画
asset-gateway generate sprite \
  --prompt "knight in silver armor with blue cape" \
  --animation-type walk \
  --input ./character_concept.png \
  --output-dir ./sprites

# 同一角色生成攻击动画
asset-gateway generate sprite \
  --prompt "knight in silver armor with blue cape" \
  --animation-type attack \
  --input ./character_concept.png \
  --output-dir ./sprites
```

`--input` 支持本地路径和 URL。Grok 会以参考图为基准生成角色动画视频（image-to-video）。

## 动画类型

通过 `--animation-type` 指定。支持预设动作和**任意自定义文本**。

### 常用预设

| 类型 | 说明 |
|------|------|
| `walk`（默认） | 走路循环 |
| `run` | 跑步循环 |
| `idle` | 待机呼吸 |
| `attack` | 攻击挥砍 |
| `death` | 死亡倒地 |
| `jump` | 起跳→滞空→落地 |
| `cast` | 施法释放魔法 |
| `dance` | 跳舞循环 |

### 自定义动画

`--animation-type` 接受任意文本，不限于预设：

```bash
# 自定义动画类型
asset-gateway generate sprite \
  --prompt "wizard with purple robe" \
  --animation-type "charge up energy and release lightning bolt" \
  --output-dir ./sprites

asset-gateway generate sprite \
  --prompt "slime monster" \
  --animation-type "split into two smaller slimes" \
  --duration 4 \
  --output-dir ./sprites
```

> 复杂动画建议增加 `--duration`，更长的视频能容纳更多动作细节。

## 视频时长

通过 `--duration` 控制 Grok 生成的视频时长（秒），直接影响帧数。

| Duration | FPS=8 帧数 | 适用 | 费用 |
|----------|-----------|------|------|
| `1` | 8 帧 | 简单循环（idle、blink） | $0.05 |
| `2`（默认） | 16 帧 | 标准动画（walk、run） | $0.10 |
| `4` | 32 帧 | 复杂动画（attack combo） | $0.20 |
| `8` | 64 帧 | 长序列（death、cutscene） | $0.40 |
| `15`（最大） | 120 帧 | 超长序列 | $0.75 |

## GIF 输出

适合预览、聊天贴图、社交分享。

```bash
asset-gateway generate sprite \
  --prompt "cat with witch hat" \
  --animation-type idle \
  --output-format gif \
  --fps 10 \
  --output-dir ./sprites
```

输出：`sprites/sprite_<timestamp>.gif` — 可直接作为聊天表情或预览动画。

> GIF 适合预览和分享，游戏引擎中建议使用 spritesheet PNG（透明背景）。

## 风格选项

通过 `--style` 指定画面风格。SpriteForge 是**风格无关的**，任何视觉风格都可以：

| Style | 效果 |
|-------|------|
| `pixel art` | 经典像素画 |
| `realistic` | 写实风格 |
| `cartoon` | 卡通风格 |
| `anime` | 日系动画风格 |
| `chibi` | Q 版大头角色 |
| `watercolor` | 水彩画风 |

也可以在 `--prompt` 中直接描述风格，效果等价：

```bash
# 两种方式等价
asset-gateway generate sprite --prompt "knight" --style "pixel art"
asset-gateway generate sprite --prompt "pixel art knight"
```

## 完整参数

| Flag | Default | 说明 |
|------|---------|------|
| `--prompt` | 必填 | **角色外观描述**（不描述动作） |
| `--animation-type` | `walk` | 动画类型（预设或自定义文本） |
| `--input` | — | 参考图（可选，本地路径或 URL，用于 image-to-video） |
| `--output-format` | `spritesheet` | 输出格式：`spritesheet`（透明 PNG）或 `gif` |
| `--duration` | `2` | 视频时长（秒），范围 1-15 |
| `--style` | — | 视觉风格（pixel art, realistic, anime 等） |
| `--fps` | `8` | 帧提取率（每秒提取帧数） |
| `--direction` | `right` | 角色朝向 |
| `--output-dir` | `.` | 输出目录 |

## 更多示例

### Idle 待机动画

```bash
asset-gateway generate sprite \
  --prompt "mage in dark blue robe, holding glowing staff" \
  --animation-type idle \
  --style "pixel art" \
  --duration 1 \
  --output-dir ./sprites
```

### Walk 走路循环

```bash
asset-gateway generate sprite \
  --prompt "knight in silver armor with red plume helmet" \
  --animation-type walk \
  --direction right \
  --output-dir ./sprites
```

### Attack 攻击动画

```bash
asset-gateway generate sprite \
  --prompt "samurai with katana" \
  --animation-type attack \
  --duration 3 \
  --output-dir ./sprites
```

### 写实风格角色

```bash
asset-gateway generate sprite \
  --prompt "medieval warrior with chainmail and longsword" \
  --animation-type run \
  --style realistic \
  --output-dir ./sprites
```

### 参考图多动画套组

```bash
# 先用参考图生成一套动画
for anim in walk run idle attack death; do
  asset-gateway generate sprite \
    --prompt "robot with glowing blue eyes" \
    --animation-type "$anim" \
    --input ./robot_concept.png \
    --output-dir ./sprites
done
```

### GIF 表情贴图

```bash
asset-gateway generate sprite \
  --prompt "corgi puppy with tiny crown" \
  --animation-type dance \
  --output-format gif \
  --fps 12 \
  --style chibi \
  --output-dir ./sprites
```

## 费用

- 基于 Grok 视频生成按秒计费：**$0.05/秒**
- 默认 2 秒视频：**$0.10/sprite**
- 后处理（ffmpeg + 背景移除）无额外费用

## Tips

- **prompt 写角色，不写动作** — `--prompt` 只描述外观（服装、武器、颜色），动作由 `--animation-type` 控制
- **自定义动画尽量具体** — `"charge up energy and release fireball"` 比 `"magic attack"` 效果好
- **生成约 15-20 秒** — Grok 视频生成 + ffmpeg 后处理全流程
- **效果不满意就重试** — 调整 prompt 措辞或换个 style，重试 2-3 次是正常的
- **GIF 用于预览** — 最终游戏资源用 spritesheet PNG（透明背景），GIF 仅用于快速预览和分享
- **参考图提升一致性** — 如果需要同一角色多套动画，用 `--input` 传入参考图保持外观统一
- **短时长更稳定** — 2 秒（默认）对大多数动画循环已经足够，复杂动画再加长
- **风格无限制** — 像素画、写实、卡通、动漫皆可，不再局限于像素风格
