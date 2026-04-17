# Sprite Animation Workflow

`generate sprite` 通过 AutoSprite 生成角色动画 spritesheet。

## 基本用法

```bash
asset-gateway generate sprite \
  --prompt "pixel art knight in silver armor" \
  --animation-type walk \
  --output-dir ./sprites
```

## 使用参考图

传入参考图保持角色外观一致性：

```bash
asset-gateway generate sprite \
  --prompt "knight in silver armor" \
  --animation-type attack \
  --input ./character.png \
  --output-dir ./sprites
```

## 动画类型

通过 `--animation-type` 指定，支持预设和自定义文本：

| 类型 | 说明 |
|------|------|
| `walk`（默认） | 走路循环 |
| `run` | 跑步循环 |
| `idle` | 待机呼吸 |
| `attack` | 攻击 |
| `jump` | 跳跃 |
| `death` | 死亡 |
| `cast` | 施法 |
| `dance` | 跳舞 |

也可以传任意自定义文本：`--animation-type "charge up energy and release fireball"`

## 风格

通过 `--style` 指定：

| Style | 效果 |
|-------|------|
| `16-bit` | 16-bit 像素画 |
| `hd-pixel` | 高清像素 |
| `retro-8bit` | 复古 8-bit |
| `isometric` | 等距视角 |
| `anime` | 日系动画 |
| `chibi` | Q 版 |
| `painterly` | 手绘 |
| `vector` | 矢量 |

## 完整参数

| Flag | Default | 说明 |
|------|---------|------|
| `--prompt` | 必填 | 角色描述 |
| `--animation-type` | `walk` | 动画类型（预设或自定义） |
| `--input` | — | 参考图（本地路径或 URL） |
| `--style` | — | 画面风格 |
| `--frame-count` | `8` | 动画帧数 |
| `--frame-size` | `256` | 帧尺寸（像素，正方形） |
| `--is-humanoid` | `true` | 角色是否为人形 |
| `--output-dir` | `.` | 输出目录 |

## 示例

```bash
# 像素骑士 walk 循环
asset-gateway generate sprite --prompt "pixel art knight" --animation-type walk --style 16-bit --output-dir ./sprites

# 同一角色多套动画
for anim in walk run idle attack; do
  asset-gateway generate sprite \
    --prompt "robot with glowing eyes" \
    --animation-type "$anim" \
    --input ./robot.png \
    --output-dir ./sprites
done

# 非人形角色
asset-gateway generate sprite \
  --prompt "slime monster" \
  --animation-type idle \
  --is-humanoid false \
  --output-dir ./sprites
```
