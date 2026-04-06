# Sprite Animation Workflow

两步生成精灵动画：用 Gemini 生成角色图 → 用 PixelEngine 生成动画。

## 推荐流程

### Step 1: 生成角色图（Gemini，默认 provider）

生成**纯色背景**的像素角色。不需要透明通道，纯色背景即可（PixelEngine 会处理）。

```bash
asset-gateway generate image \
  --prompt "pixel art knight character, idle pose, facing right, clean pixel edges, solid bright green background, retro game sprite style, no shadow" \
  --size 256x256 \
  --output-dir ./sprites
```

**关键 prompt 要素：**
- `solid bright green background`（或任意纯色）— 不要复杂背景
- `facing right` — 指定朝向，动画更自然
- `idle pose` — 静止姿态作为动画起点
- `clean pixel edges` — 减少模糊边缘
- `no shadow` — 避免地面阴影干扰

> PixelEngine 会自动缩放到 256px（pixel 模型）或直接使用（HD 模型），无需手动调整。

### Step 2: 生成精灵动画（PixelEngine）

```bash
asset-gateway generate sprite \
  --prompt "walk cycle animation, the knight walks to the right with smooth alternating leg movement, arms swinging naturally" \
  --input ./sprites/image_*.png \
  --output-frames 8 \
  --output-format spritesheet \
  --output-dir ./sprites
```

输出：`sprites/sprite_<timestamp>.png` — 水平排列的 spritesheet（如 8 帧 × 256px = 2048×256）。

## 模型选择

| 模型 | 适用 | 输入要求 | 帧数 |
|------|------|---------|------|
| `pixel-engine-v1.1`（默认） | 像素画角色 | PNG，≤256×256（自动缩放） | 2-16（偶数） |
| `frame-engine-v1.1` | HD 插画 | PNG/JPEG，256-2048px | 2-24（偶数） |

**默认用 pixel 模型**。输入图超过 256px 时，网关会自动用 ImageMagick 缩放。

## 输出格式

| 格式 | 说明 | 用途 |
|------|------|------|
| `spritesheet`（默认） | 水平条状 PNG：`width = frame_count × frame_w` | 游戏引擎（Godot AnimatedSprite2D, Unity） |
| `gif` | 动画 GIF | 预览、分享 |
| `webp` | 动画 WebP | Web 预览 |

## Prompt 写法

PixelEngine 的 prompt 应该**描述完整的动作序列**，而不是静态描述。

### ✅ 好的 prompt（时间密集、动作聚焦）

```
walk cycle animation, the knight walks to the right with smooth alternating
leg movement, arms swinging naturally, feet alternating step by step
```

```
from a standing position, the wizard raises his staff above his head,
energy accumulates around the staff, a ball of yellow light fires from
the staff, then the wizard lowers the staff back to neutral position
```

### ❌ 差的 prompt

```
knight walking    ← 太短，模型不知道具体动作
a cool character  ← 描述外观而非动作
```

### 各动作推荐帧数

| 动作 | 帧数 |
|------|------|
| idle（待机） | 4-8 |
| walk（走路） | 6-8 |
| run（跑步） | 4-8 |
| attack（攻击） | 8-12 |
| death（死亡） | 6-8 |

## 输入图技巧

1. **给角色留空间** — 角色不要填满画面，四周留白让动画有伸展空间
2. **起始姿态匹配动作** — 走路动画用站立姿态，攻击动画用举武器姿态
3. **纯色背景** — `solid green/blue/white background`，不用透明
4. **24 色左右** — 颜色太少会丢细节，太多会帧间闪烁。默认 24 色适合大多数情况

## 完整参数

| Flag | Default | 说明 |
|------|---------|------|
| `--prompt` | 必填 | 动作描述 |
| `--input` | 必填 | 角色图（本地路径或 URL） |
| `--model` | `pixel-engine-v1.1` | 动画模型 |
| `--output-frames` | `8` | 帧数（偶数） |
| `--output-format` | `spritesheet` | 输出格式 |
| `--colors` | `24` | 调色板颜色数（pixel 模型，2-256） |
| `--negative-prompt` | — | 排除内容 |
| `--seed` | 随机 | 复现种子 |
| `--matte-color` | — | 透明通道展平色（6位 hex） |

## 更多示例

### 攻击动画

```bash
# Step 1: 生成持剑角色
asset-gateway generate image \
  --prompt "pixel art warrior, attack ready pose with sword raised, facing right, solid blue background, retro style" \
  --size 256x256 --output-dir ./sprites

# Step 2: 生成攻击动画
asset-gateway generate sprite \
  --prompt "sword slash attack, the warrior swings sword in a wide arc from above, creating a white motion trail, then returns to ready stance" \
  --input ./sprites/image_*.png \
  --output-frames 10 \
  --output-dir ./sprites
```

### Idle 待机动画

```bash
asset-gateway generate sprite \
  --prompt "gentle idle breathing animation, the character subtly shifts weight, chest rises and falls" \
  --input ./sprites/character.png \
  --output-frames 6 \
  --output-dir ./sprites
```

### GIF 预览

```bash
asset-gateway generate sprite \
  --prompt "walk cycle" \
  --input ./sprites/character.png \
  --output-format gif \
  --output-dir ./sprites
```

## 费用

- 角色图生成（Gemini）：**~$0.04**
- 精灵动画（PixelEngine）：**20 credits/次**（~$0.10）
- Prompt 增强、余额查询、取消任务：免费

## 注意事项

- 输入图宽高比须在 1:2 到 2:1 之间
- 图片最大 5MB（解码后）
- 生成耗时约 90 秒
- 输出文件 24 小时后过期
- 效果不满意时，调整 prompt 描述或帧数后重试 3-4 次是正常的
