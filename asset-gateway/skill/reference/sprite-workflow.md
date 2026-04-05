# Sprite Animation Workflow

Complete pipeline for generating consistent sprite animation frames using asset-gateway.

## Recommended: Video-Based Pipeline

This approach produces **perfectly consistent** animation frames (same character across all frames) by generating a video first, then extracting frames.

### Step 1: Generate Transparent Reference Character

```bash
asset-gateway generate image --transparent \
  --prompt "pixel art knight character, idle pose, facing right, clean edges, white outline" \
  --size 1024x1024 --output-dir ./sprites
```

### Step 2: Upload Reference Image

```bash
asset-gateway upload file ./sprites/image_*.png
# Get URL: https://upload.xiaomao.chat/uploads/xxx.png
```

### Step 3: Generate Animation Video (Image-to-Video)

Use the reference image as input for Seedance image-to-video. The character stays perfectly consistent across all frames.

```bash
asset-gateway generate video \
  --prompt "walk cycle animation, smooth movement, side view" \
  --input https://upload.xiaomao.chat/uploads/xxx.png \
  --output-dir ./sprites
```

### Step 4: Extract Frames

Extract evenly-spaced frames from the video:

```bash
asset-gateway process extract-frames \
  --input ./sprites/*.mp4 \
  --count 8 \
  --output-dir ./sprites/frames
```

### Step 5: Remove Backgrounds

Remove the video background from all frames (bgsweep AI):

```bash
asset-gateway process remove-bg \
  --input ./sprites/frames/frame_*.png \
  --output-dir ./sprites/nobg
```

### Step 6: Compose Sprite Sheet

Combine frames into a horizontal sprite strip:

```bash
asset-gateway process compose \
  --input ./sprites/nobg/frame_*.png \
  --direction horizontal \
  --frame-width 64 --frame-height 64 \
  --output-dir ./sprites/final
```

Or a grid layout:

```bash
asset-gateway process compose \
  --input ./sprites/nobg/frame_*.png \
  --direction grid --columns 4 \
  --frame-width 64 --frame-height 64 \
  --output-dir ./sprites/final
```

## Alternative: Frame-by-Frame Generation

For cases where video generation isn't suitable (e.g., very specific poses). Less consistent but more controllable per-frame.

### Step 1: Generate Reference

```bash
asset-gateway generate image --transparent \
  --prompt "pixel art knight, idle pose, facing right, clean edges" \
  --size 1024x1024 --output-dir ./sprites
```

### Step 2: Upload & Get URL

```bash
asset-gateway upload file ./sprites/image_*.png
```

### Step 3: Generate Each Frame

Pass `--ref` to maintain character consistency:

```bash
asset-gateway generate image --transparent \
  --prompt "same knight, walk frame 1, left foot forward" \
  --ref https://upload.xiaomao.chat/uploads/xxx.png --output-dir ./sprites

asset-gateway generate image --transparent \
  --prompt "same knight, walk frame 2, feet together" \
  --ref https://upload.xiaomao.chat/uploads/xxx.png --output-dir ./sprites

asset-gateway generate image --transparent \
  --prompt "same knight, walk frame 3, right foot forward" \
  --ref https://upload.xiaomao.chat/uploads/xxx.png --output-dir ./sprites
```

### Step 4: Crop, Resize, Compose

```bash
# Compose with auto-normalization
asset-gateway process compose \
  --input ./sprites/image_*.png \
  --direction horizontal \
  --frame-width 64 --frame-height 64 \
  --output-dir ./sprites/final
```

## Prompt Tips

- Reference frame: specify **style** (pixel art / hand-drawn / chibi) and **direction** (facing right)
- Per-frame prompts: start with "same character" and only describe the pose change
- More frames = smoother animation (walk: 4-8 frames, attack: 3-6 frames)
- Add "clean edges, no shadow on ground" for cleaner crop results
- Video approach: describe the motion type ("walk cycle", "attack swing", "idle breathing")

## Why Video-Based is Better

| Aspect | Video-Based | Frame-by-Frame |
|--------|-------------|----------------|
| Character consistency | Perfect (same model across frames) | Variable (AI may alter proportions) |
| Speed | ~1 video gen + pipeline | N × image gen calls |
| Control per frame | Less (auto motion) | Full (specific pose per frame) |
| Best for | Walk/run/idle cycles, simple motions | Specific pose sequences, complex actions |
