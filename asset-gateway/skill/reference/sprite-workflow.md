# Sprite Animation Workflow

Generate sprite animations from a static image using PixelEngine AI.

## Quick Start

```bash
# Animate a pixel art character (≤256×256 PNG)
asset-gateway generate sprite \
  --prompt "walk cycle animation, smooth movement" \
  --input ./character.png \
  --output-dir ./sprites

# Animate an HD illustration (256-2048px)
asset-gateway generate sprite \
  --prompt "idle breathing animation" \
  --input ./hero.png \
  --model frame-engine-v1.1 \
  --output-dir ./sprites
```

## Models

| Model | Best for | Input | Max size | Frames |
|-------|----------|-------|----------|--------|
| `pixel-engine-v1.1` | Pixel art sprites | PNG only | ≤256×256 | 2-16 (even) |
| `frame-engine-v1.1` | HD illustrations | PNG or JPEG | 256-2048px | 2-24 (even) |

## Output Formats

| Format | Description | Use case |
|--------|-------------|----------|
| `spritesheet` (default) | Horizontal strip PNG: `width = frame_count × frame_w` | Game engines (Godot, Unity) |
| `webp` | Animated WebP | Web previews |
| `gif` | Animated GIF | Sharing, documentation |

## Parameters

| Flag | Default | Description |
|------|---------|-------------|
| `--prompt` | required | Motion description (e.g. "walk cycle", "attack swing") |
| `--input` | required | Source sprite image (local path or URL) |
| `--model` | `pixel-engine-v1.1` | Animation model |
| `--output-frames` | `8` | Number of frames (must be even) |
| `--output-format` | `spritesheet` | Output: spritesheet, webp, gif |
| `--colors` | `24` | Pixel palette size (pixel model only, 2-256) |
| `--negative-prompt` | — | What to avoid |
| `--seed` | random | Reproducibility seed |
| `--matte-color` | — | Hex color for alpha flattening |
| `--enhance-prompt` | off | AI-enhance the prompt before generation |

## Typical Workflows

### Character Walk Cycle

```bash
# 1. Generate the reference character (via image generation)
asset-gateway generate image --transparent \
  --prompt "pixel art knight character, idle pose, facing right" \
  --size 128x128 --output-dir ./sprites

# 2. Animate it
asset-gateway generate sprite \
  --prompt "walk cycle, smooth left-right movement, feet alternating" \
  --input ./sprites/image_*.png \
  --output-frames 8 \
  --output-dir ./sprites
```

### Attack Animation

```bash
asset-gateway generate sprite \
  --prompt "sword slash attack, wind up then swing, motion blur on blade" \
  --input ./sprites/knight.png \
  --output-frames 6 \
  --output-dir ./sprites
```

### HD Character Idle

```bash
asset-gateway generate sprite \
  --prompt "gentle idle breathing animation, subtle movement" \
  --input ./hero_illustration.png \
  --model frame-engine-v1.1 \
  --output-frames 12 \
  --output-format webp \
  --output-dir ./sprites
```

## Cost

- 20 credits per animation (~$0.10 at monthly rate)
- Free: prompt enhancement, balance check, job cancellation

## Notes

- Input image aspect ratio must be between 1:2 and 2:1
- Max decoded image size: 5 MB
- Generation takes ~90 seconds
- Output files are available for 24 hours after generation
- Spritesheets are horizontal strips: `width = frame_count × frame_w`
