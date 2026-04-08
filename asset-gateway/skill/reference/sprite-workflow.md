# Sprite Animation Workflow

一条命令生成精灵动画：文本描述角色 → SpriteForge 自动完成 prompt 增强 → Gemini 生图 → 后处理输出 spritesheet/GIF。

## 概览

SpriteForge 是自包含的精灵动画生成管线，无需预先准备输入图。核心流程：

1. **文本描述** → 你提供角色描述（`--prompt`）和动画类型（`--animation-type`）
2. **LLM Prompt 增强** → 自动将简短描述扩展为高质量生成 prompt
3. **Gemini 图像生成** → 生成动画帧
4. **后处理** → 输出 spritesheet PNG 或 animated GIF

> 与旧版 PixelEngine 不同，SpriteForge **不再需要单独的输入图步骤**。prompt 描述的是**角色外观**，动作由 `--animation-type` 控制。

## 基本用法（单条命令）

```bash
asset-gateway generate sprite \
  --prompt "pixel art knight in silver armor with blue cape" \
  --animation-type walk \
  --output-dir ./sprites
```

输出：`sprites/sprite_<timestamp>.png` — 水平排列的 spritesheet（如 6 帧 × 256px = 1536×256）。

**关键区别：**
- `--prompt` 描述**角色外观**，不描述动作序列
- `--animation-type` 控制动画类型（walk、attack 等）
- 不需要 `--input`，SpriteForge 自动生成角色图

## 使用参考图（可选两步流程）

如果你已有角色概念图，可以作为参考传入：

```bash
# Step 1（可选）: 生成或准备参考图
asset-gateway generate image \
  --prompt "pixel art knight in silver armor, idle pose, facing right, solid green background" \
  --size 256x256 \
  --output-dir ./sprites

# Step 2: 用参考图生成动画
asset-gateway generate sprite \
  --prompt "knight in silver armor with blue cape" \
  --animation-type walk \
  --input ./sprites/image_*.png \
  --output-dir ./sprites
```

> 参考图帮助 SpriteForge 保持角色外观一致性，但不是必须的。

## 动画类型

通过 `--animation-type` 指定。支持预设动作和**任意自定义文本**。

### 常用预设

| 类型 | 说明 | 推荐帧数 |
|------|------|---------|
| `idle` | 待机呼吸 | 4-6 |
| `walk` | 走路循环 | 6-8 |
| `run` | 跑步循环 | 6-8 |
| `attack` | 攻击挥砍 | 8-10 |
| `death` | 死亡倒地 | 6-8 |
| `jump` | 起跳→滞空→落地 | 6-8 |
| `cast` | 施法释放魔法 | 8-10 |
| `dance` | 跳舞循环 | 8-10 |

### 自定义动画

`--animation-type` 接受任意文本，不限于预设：

```bash
# 自定义动画类型
asset-gateway generate sprite \
  --prompt "pixel art wizard with purple robe" \
  --animation-type "charge up energy and release lightning bolt" \
  --output-dir ./sprites

asset-gateway generate sprite \
  --prompt "pixel art slime monster" \
  --animation-type "split into two smaller slimes" \
  --output-dir ./sprites
```

## 网格尺寸

通过 `--grid-size` 控制每帧的像素尺寸。

| Grid Size | 适用 | 说明 |
|-----------|------|------|
| `32x32` | 小型 NPC、道具 | 经典像素尺寸 |
| `64x64` | 标准角色 | 平衡细节与性能 |
| `128x128` | 大型角色、Boss | 高细节 |
| `256x256`（默认） | 展示/高清 | 最大细节 |

```bash
asset-gateway generate sprite \
  --prompt "pixel art goblin with dagger" \
  --animation-type attack \
  --grid-size 64x64 \
  --output-dir ./sprites
```

## GIF 输出

适合预览、聊天贴图、社交分享。

```bash
asset-gateway generate sprite \
  --prompt "pixel art cat with witch hat" \
  --animation-type idle \
  --output-format gif \
  --fps 10 \
  --output-dir ./sprites
```

输出：`sprites/sprite_<timestamp>.gif` — 可直接作为聊天表情或预览动画。

> GIF 适合预览和分享，游戏引擎中建议使用 spritesheet PNG。

## 风格选项

通过 `--style` 指定画面风格：

| Style | 效果 |
|-------|------|
| `pixel`（默认） | 经典像素画，清晰边缘 |
| `retro` | 复古 8-bit / 16-bit 风格 |
| `modern` | 现代像素画，渐变和光影 |
| `chibi` | Q 版大头角色 |

```bash
asset-gateway generate sprite \
  --prompt "warrior with flaming sword" \
  --animation-type idle \
  --style chibi \
  --output-dir ./sprites
```

## 完整参数

| Flag | Default | 说明 |
|------|---------|------|
| `--prompt` | 必填 | **角色外观描述**（不描述动作） |
| `--animation-type` | `idle` | 动画类型（预设或自定义文本） |
| `--input` | — | 参考图（可选，本地路径或 URL） |
| `--output-format` | `spritesheet` | 输出格式：`spritesheet`（PNG）或 `gif` |
| `--grid-size` | `256x256` | 每帧像素尺寸 |
| `--style` | `pixel` | 画面风格 |
| `--fps` | `8` | GIF 帧率 |
| `--direction` | `right` | 角色朝向 |
| `--output-dir` | `.` | 输出目录 |

## 更多示例

### Idle 待机动画

```bash
asset-gateway generate sprite \
  --prompt "pixel art mage in dark blue robe, holding glowing staff" \
  --animation-type idle \
  --output-dir ./sprites
```

### Walk 走路循环

```bash
asset-gateway generate sprite \
  --prompt "pixel art knight in silver armor with red plume helmet" \
  --animation-type walk \
  --direction right \
  --grid-size 128x128 \
  --output-dir ./sprites
```

### Attack 攻击动画

```bash
asset-gateway generate sprite \
  --prompt "pixel art samurai with katana" \
  --animation-type attack \
  --grid-size 128x128 \
  --output-dir ./sprites
```

### GIF 表情贴图

```bash
asset-gateway generate sprite \
  --prompt "pixel art corgi puppy with tiny crown" \
  --animation-type dance \
  --output-format gif \
  --fps 12 \
  --style chibi \
  --output-dir ./sprites
```

## 费用

- 每次生成（LLM 增强 + Gemini 生图 + 后处理）：**~$0.05**
- 无额外 credit 系统，按实际 API 调用计费

## Tips

- **prompt 写角色，不写动作** — `--prompt` 只描述外观（服装、武器、颜色），动作由 `--animation-type` 控制
- **自定义动画尽量具体** — `"charge up energy and release fireball"` 比 `"magic attack"` 效果好
- **生成耗时 60-90 秒** — SpriteForge 需要完成 LLM 增强 → 生图 → 后处理全流程
- **效果不满意就重试** — 调整 prompt 措辞或换个 style，重试 2-3 次是正常的
- **GIF 用于预览** — 最终游戏资源用 spritesheet PNG，GIF 仅用于快速预览和分享
- **参考图提升一致性** — 如果需要同一角色多套动画，用参考图保持外观统一
- **小尺寸更稳定** — 64x64 和 128x128 的像素画质量通常比 256x256 更一致
