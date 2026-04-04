---
name: asset-gateway
description: "Unified multi-provider asset generation (text, image, video, audio, TTS, 3D) via CLI + 2D image post-processing and Tripo 3D pipeline steps (rig, animate, texture, convert, stylize). Use when the user asks to generate or transform images, audio, TTS/speech, video, 3D models, or text content."
---

# Asset Gateway

`asset-gateway` is the **single CLI** for all asset generation and post-processing. It routes requests to the best available provider automatically, with health-check filtering and fallback. Always prefer this over calling provider APIs directly.

## Install & Auth

```bash
npm install -g @doufunao123/asset-gateway
asset-gateway auth set <token>       # one-time, persisted to ~/.config/asset-gateway/auth.json
asset-gateway auth status            # verify
```

Default gateway: `https://upload.xiaomao.chat`. Override with `--gateway-url` or `ASSET_GATEWAY_URL`.

## Decision Guide: Which Command?

| User wants... | Command | Key flags |
|--------------|---------|-----------|
| An image from text | `generate image` | `--prompt`, `--transparent`, `--provider`, `--size` |
| Edit an existing image | `generate image` | `--prompt`, `--input <url>`, `--provider` |
| Edit with reference images | `generate image` | `--prompt`, `--ref <url>` (repeatable, up to 14) |
| Inpaint (semantic masking) | `generate image` | `--prompt`, `--input <url>`, `--edit-mode inpaint` |
| Restyle an image | `generate image` | `--prompt`, `--input <url>`, `--edit-mode restyle` |
| Multi-turn image editing | `generate image` | `--prompt`, `--session <id>` |
| Character consistency | `generate image` | `--prompt`, `--ref char1.png --ref char2.png` |
| A video clip | `generate video` | `--prompt` |
| Sound effect or BGM | `generate audio` | `--prompt`, `--type sfx\|bgm`, `--duration` |
| Text-to-speech (Chinese/multilingual) | `generate tts` | `--prompt`, `--voice-id`, `--model`, `--speed`, `--language-boost` |
| A game-ready 3D model | `generate model` | `--prompt`, `--face-limit`, `--pbr`, `--model-version` |
| 3D from reference image | `generate model` | `--image <url>`, `--face-limit`, `--pbr` |
| 3D from 4 angles | `generate model` | `--multiview front,left,back,right` |
| Convert 3D format | `process3d convert` | `--task-id`, `--format FBX\|USDZ\|OBJ` |
| Rig a 3D character | `process3d rig` | `--task-id`, `--spec mixamo` |
| Animate a character | `process3d animate` | `--task-id`, `--animation preset:walk` |
| Re-texture a model | `process3d texture` | `--task-id`, `--prompt`, `--pbr` |
| Reduce polygon count | `process3d reduce` | `--task-id`, `--face-limit 3000` |
| Stylize to voxel/lego | `process3d stylize` | `--task-id`, `--style lego` |
| Segment a mesh into parts | `process3d segment` | `--task-id` |
| Check whether a mesh is riggable | `process3d prerigcheck` | `--task-id` |
| Refine a draft model to higher quality | `process3d refine` | `--task-id` |
| Import an external 3D model for post-processing | `process3d import` | `--file-url` or `--file-path` |
| LLM text completion | `generate text` | `--prompt`, `--model`, `--max-tokens` |
| Remove image background | `process remove-bg` | `--input`, `--smart-crop` |
| Crop to power-of-2 | `process crop` | `--input`, `--mode power_of2` |
| Resize image | `process resize` | `--input`, `--width`, `--height` |
| AI upscale (2x/4x) | `process upscale` | `--input`, `--scale 4` |

**All generate commands require `--output-dir`** to save files locally. `process` and `process3d` commands also use `--output-dir`.

## Core Workflows

### Image Generation (most common)

```bash
# Default: auto-selects best provider (gemini_image, highest priority)
asset-gateway generate image --prompt "isometric village, clean lighting" --output-dir ./assets

# With specific size
asset-gateway generate image --prompt "banner" --size 1792x1024 --output-dir ./assets

# Image editing: modify an existing image with a prompt
asset-gateway generate image --prompt "make the background a sunset" --input "https://example.com/photo.png" --output-dir ./assets

# Image editing with model override
asset-gateway generate image --prompt "add a hat to the character" --input "https://example.com/char.png" --output-dir ./assets
```

### Multi-Image Reference & Editing

Gemini 3.1 Flash supports up to 14 reference images for composition, character consistency, and style transfer.

```bash
# Multi-image composition: combine reference images into a new scene
asset-gateway generate image \
  --prompt "create a scene with these characters in a forest" \
  --ref character1.png --ref character2.png --ref forest-bg.png \
  --output-dir ./assets

# Character consistency: maintain character across different scenes
asset-gateway generate image \
  --prompt "draw this character riding a horse at sunset" \
  --ref character-front.png --ref character-side.png \
  --output-dir ./assets

# Style transfer: apply the style of one image to another
asset-gateway generate image \
  --prompt "recreate this photo in the style of the reference painting" \
  --input photo.jpg --ref painting-style.png --edit-mode restyle \
  --output-dir ./assets

# Semantic inpaint: describe what to change in natural language
asset-gateway generate image \
  --prompt "replace the sofa with a red leather couch" \
  --input living-room.png --edit-mode inpaint \
  --output-dir ./assets
```

### Multi-Turn Image Editing (Sessions)

Generate an image, then iteratively edit it across multiple turns. The gateway automatically manages conversation history via sessions.

```bash
# Turn 1: Generate initial image → returns session_id
asset-gateway generate image \
  --prompt "create a vibrant infographic about photosynthesis" \
  --size 1792x1024 --output-dir ./assets
# Response includes session_id: "ses_abc123..."

# Turn 2: Edit using session (no need to re-upload the image)
asset-gateway generate image \
  --prompt "translate all text to Spanish, keep the layout the same" \
  --session ses_abc123... --output-dir ./assets

# Turn 3: Further refinement
asset-gateway generate image \
  --prompt "add a decorative border and make the title larger" \
  --session ses_abc123... --output-dir ./assets
```

**Session rules:**
- Sessions auto-expire after 24 hours
- Each session is pinned to one provider + model
- The gateway stores conversation history in PostgreSQL (survives restarts)
- `session_id` is returned in every image generation response

**Image size control** (`--size WxH`): The `--size` flag maps to Gemini's `imageConfig`:

| `--size` example | imageSize tier | aspectRatio | Typical output |
|-----------------|---------------|-------------|----------------|
| `512x512` | 512 | 1:1 | 512×512 |
| `1024x1024` | 1K | 1:1 | 1024×1024 |
| `1792x1024` | 2K | 16:9 | 2752×1536 |
| `1024x1792` | 2K | 9:16 | 1536×2752 |
| `4096x2304` | 4K | 16:9 | ~4K wide |

Supported aspect ratios: `1:1`, `16:9`, `9:16`, `4:3`, `3:4`, `3:2`, `2:3` (auto-matched to nearest). Gemini chooses the exact output resolution within the tier.

**Provider choice guide:**
- **Images** → `gemini_image` (default, ~15s, supports editing via `--input` and `--size`, ~$0.04/gen)
- **Video** → `grok_image` (auto-routed, ~$0.10/gen)

### 3D Model Generation

`generate model` is the entry point for Tripo's full 3D pipeline. It supports `--model-version` (latest flagship: `P1-20260311`), `--face-limit`, `--pbr`, `--texture-quality standard|detailed`, `--auto-size`, `--negative-prompt`, and `--multiview`.

Every successful 3D generation returns `metadata.tripo_task_id`. Use that task ID as the `--task-id` input for the first `process3d` step.

```bash
# High-quality text-to-3D with the latest P1 model
asset-gateway generate model \
  --prompt "low-poly warrior character, T-pose" \
  --model-version P1-20260311 \
  --face-limit 5000 \
  --pbr \
  --texture-quality detailed \
  --auto-size \
  --output-dir ./assets

# Image-to-3D from a public reference image
asset-gateway generate model \
  --image "https://upload.xiaomao.chat/uploads/character-concept.png" \
  --model-version P1-20260311 \
  --face-limit 8000 \
  --pbr \
  --output-dir ./assets

# Negative guidance for cleaner outputs
asset-gateway generate model \
  --prompt "stylized sci-fi soldier" \
  --negative-prompt "no weapons, no cape" \
  --output-dir ./assets

# Multi-view generation for highest accuracy
asset-gateway generate model \
  --multiview front.png,left.png,back.png,right.png \
  --model-version P1-20260311 \
  --face-limit 5000 \
  --pbr \
  --output-dir ./assets
```

### 3D Post-Processing Pipeline

All `process3d` commands continue an existing Tripo task. Use `metadata.tripo_task_id` from `generate model`, or `data.tripo_task_id` from a previous `process3d` response.

```bash
# Format conversion + optional quad remesh / face cap
asset-gateway process3d convert --task-id abc-123 --format FBX --quad --face-limit 3000 --output-dir ./assets

# Re-texture a model and upgrade to detailed PBR output
asset-gateway process3d texture --task-id abc-123 --prompt "cartoon style, bright colors" --pbr --quality detailed --output-dir ./assets

# Auto-rig with Mixamo-compatible skeleton
asset-gateway process3d rig --task-id abc-123 --spec mixamo --format fbx --output-dir ./assets

# Apply a preset animation
asset-gateway process3d animate --task-id abc-123 --animation preset:walk --format fbx --output-dir ./assets

# Reduce polygon count for mobile targets
asset-gateway process3d reduce --task-id abc-123 --face-limit 2000 --quad --output-dir ./assets

# Stylize geometry
asset-gateway process3d stylize --task-id abc-123 --style voxel --output-dir ./assets

# Segment parts or validate riggability
asset-gateway process3d segment --task-id abc-123 --output-dir ./assets
asset-gateway process3d prerigcheck --task-id abc-123 --output-dir ./assets

# Refine a draft model to higher quality geometry
asset-gateway process3d refine --task-id abc-123 --output-dir ./assets

# Import an external 3D model (GLB/FBX) for post-processing (rig, animate, texture, convert, etc.)
asset-gateway process3d import --file-url "https://example.com/model.glb" --output-dir ./assets
asset-gateway process3d import --file-path ./my-model.glb --output-dir ./assets
```

**What each `process3d` command does:**
- `convert` → export to `FBX`, `USDZ`, `OBJ`, `STL`, `GLTF`, or `3MF`, with optional quad remesh and face reduction
- `texture` → re-texture, enable PBR, or upgrade to higher texture quality with prompt guidance
- `rig` → add a skeleton using `mixamo` or `tripo` rig specs
- `animate` → retarget a preset animation such as walk, run, idle, slash, or dance
- `reduce` → optimize high-poly meshes down to a face budget
- `stylize` → generate themed variants like `lego`, `voxel`, `voronoi`, or `minecraft`
- `segment` → split the mesh into semantic parts for downstream editing
- `prerigcheck` → validate whether the mesh is suitable for auto-rigging
- `refine` → enhance a draft model with higher quality geometry and details
- `import` → import an external 3D model (GLB/FBX) so it can use all post-processing operations

### 3D Workflows

**Game character with animations:**

```bash
# 1. Generate the character
asset-gateway generate model --prompt "low-poly warrior character, T-pose" \
  --model-version P1-20260311 --face-limit 5000 --pbr --output-dir ./assets
# Response includes metadata.tripo_task_id = "abc-123"

# 2. Rig the model
asset-gateway process3d rig --task-id abc-123 --spec mixamo --output-dir ./assets

# 3. Animate using the rig task's returned tripo_task_id
asset-gateway process3d animate --task-id <rig-task-id> --animation preset:idle --output-dir ./assets
asset-gateway process3d animate --task-id <rig-task-id> --animation preset:walk --output-dir ./assets
asset-gateway process3d animate --task-id <rig-task-id> --animation preset:run --output-dir ./assets

# 4. Export for game engine import
asset-gateway process3d convert --task-id <animate-task-id> --format FBX --output-dir ./assets
```

**Game prop (optimized):**

```bash
asset-gateway generate model --prompt "wooden treasure chest" \
  --face-limit 3000 --pbr --texture-quality detailed --output-dir ./assets

asset-gateway process3d convert --task-id <id> --format FBX --quad --output-dir ./assets
```

**High-quality from reference images:**

```bash
# Upload reference images first if they are local
asset-gateway upload file ./character-concept.png

asset-gateway generate model --image "https://upload.xiaomao.chat/uploads/<uploaded-file>.png" \
  --model-version P1-20260311 --face-limit 8000 --pbr --output-dir ./assets
```

**Multi-view precision model:**

```bash
asset-gateway generate model \
  --multiview front.png,left.png,back.png,right.png \
  --face-limit 5000 --pbr --output-dir ./assets
```

**Reduce + re-texture workflow:**

```bash
asset-gateway process3d reduce --task-id <id> --face-limit 2000 --quad --output-dir ./assets
asset-gateway process3d texture --task-id <id> --prompt "cartoon style, bright colors" --pbr --quality detailed --output-dir ./assets
```

**Import external model + process:**

```bash
# Import your own GLB/FBX into Tripo pipeline, then rig & animate it
asset-gateway process3d import --file-url "https://example.com/character.glb" --output-dir ./assets
# Response returns tripo_task_id → use for subsequent operations
asset-gateway process3d rig --task-id <import-task-id> --spec mixamo --output-dir ./assets
asset-gateway process3d animate --task-id <rig-task-id> --animation preset:walk --output-dir ./assets
```

**Refine draft model:**

```bash
asset-gateway process3d refine --task-id <id> --output-dir ./assets
```

**Stylization:**

```bash
asset-gateway process3d stylize --task-id <id> --style voxel --output-dir ./assets
asset-gateway process3d stylize --task-id <id> --style minecraft --output-dir ./assets
```

### Animation Presets

Use `process3d animate --animation preset:<name>` with Tripo's preset library. Common presets include `walk`, `run`, `idle`, `jump`, `slash`, `shoot`, `dance_01` through `dance_06`, `box_01` through `box_03`, `climb`, `swim`, `surf`, `cast_a_spell`, `cheer`, `clap`, `bow`, `wave_goodbye`, `sit`, `fall`, `hurt`, `defeat`, `cry`, and `laugh`.

### Tripo Cost Guide

| Operation | Credits | ~USD |
|-----------|---------|------|
| `text_to_model` (`P1`) | 30-40 | $0.30-0.40 |
| `image_to_model` (`P1`) | 40-50 | $0.40-0.50 |
| `multiview_to_model` | 40-50 | $0.40-0.50 |
| `texture_model` | 20-40 | $0.20-0.40 |
| `animate_rig` | ~25 | $0.25 |
| `animate_retarget` | ~25 | $0.25 |
| `convert_model` | 5-10 | $0.05-0.10 |
| `highpoly_to_lowpoly` | ~30 | $0.30 |
| `stylize_model` | ~5 | $0.05 |
| `refine_model` | 20-30 | $0.20-0.30 |
| `import_model` | 0 | free |

### Post-Processing Pipeline

Process commands transform existing images via server-side tools (rembg BiRefNet, ImageMagick, Real-ESRGAN).

```bash
# Remove background (AI-powered, BiRefNet model)
asset-gateway process remove-bg --input ./character.png --output-dir ./sprites

# Remove background + auto crop to power-of-2 (game engine ready)
asset-gateway process remove-bg --input ./character.png --smart-crop --output-dir ./sprites

# Smart crop (trim transparent borders)
asset-gateway process crop --input ./sprite.png --mode tightest --output-dir ./out

# Crop to nearest power-of-2 dimensions
asset-gateway process crop --input ./sprite.png --mode power_of2 --output-dir ./out

# Resize to exact dimensions
asset-gateway process resize --input ./img.png --width 512 --height 512 --output-dir ./out

# AI upscale 4x (Real-ESRGAN)
asset-gateway process upscale --input ./low_res.png --scale 4 --output-dir ./out
```

**Typical game asset workflow:**
1. `generate image` → raw image with background
2. `process remove-bg --smart-crop` → transparent sprite, power-of-2 aligned
3. Ready for Bevy/Godot/Unity

Operations can also be chained via the API:
```bash
curl -X POST https://upload.xiaomao.chat/api/process \
  -H "Authorization: Bearer <token>" \
  -d '{
    "input": "https://example.com/raw.png",
    "operations": [
      {"op": "remove_bg"},
      {"op": "smart_crop", "mode": "power_of2"},
      {"op": "resize", "width": 256, "height": 256}
    ]
  }'
```

### Audio Generation

```bash
# Sound effect (default)
asset-gateway generate audio --prompt "sword slash impact" --output-dir ./assets

# Background music with duration
asset-gateway generate audio --prompt "ambient medieval tavern" --type bgm --duration 30 --output-dir ./assets
```

### Text-to-Speech (TTS)

See the dedicated **Qwen3-TTS** section below for full details, models, and voice management.

### Video Generation

```bash
# Auto-routes to grok_image video provider
asset-gateway generate video --prompt "camera slowly panning over a misty mountain" --output-dir ./assets
```

### Text / LLM

```bash
# Default model: claude-sonnet-4-6
asset-gateway generate text --prompt "Write a backstory for a desert kingdom" --output-dir ./assets

# Specific model
asset-gateway generate text --prompt "Describe a crafting system" --model gpt-5.4 --output-dir ./assets
```

## Pre-Flight Check

Before generating assets in a new session, verify the gateway is reachable and providers are healthy:

```bash
asset-gateway auth status
asset-gateway provider list
asset-gateway provider health          # all providers
asset-gateway provider health elevenlabs  # specific provider
```

Only proceed if the required provider type shows `healthy: true`.

## Available Providers

| ID | Asset Types | Speed | Notes |
|----|------------|-------|-------|
| `gemini_image` | image | ~15s | Default for images; supports editing (`--input`), up to 14 reference images (`--ref`), multi-turn sessions (`--session`), edit modes (inpaint/restyle/expand); ~$0.04-0.08/gen |
| `grok_image` | video | varies | Grok imagine video, ~$0.10/video |
| `qwen_tts` | tts | ~97ms first packet | Qwen3-TTS via DashScope Intl, 49+ system voices, VC + VD workflows, ~$0.115/10K chars |
| `elevenlabs` | audio | ~2s | BGM/SFX sound generation, ~$0.05/call |
| `tripo3d` | model3d | ~30-90s | text/image/multiview → 3D, plus rig, animate, texture, convert, stylize, reduce; full chain typically tops out around ~$1.00 |
| `llm_proxy` | text | ~1-3s | Claude/GPT/Gemini/Grok, ~$0.02-0.10/call |

## Output Contract

Every command returns a JSON envelope:

```json
{
  "ok": true,
  "command": "generate.image",
  "data": {
    "job_id": "uuid",
    "provider_id": "gemini_image",
    "cost_usd": 0.04,
    "elapsed_ms": 15000,
    "local_path": "./assets/image_1234.png",
    "metadata": { "model": "gemini-3.1-flash-image-preview" },
    "session_id": "ses_abc123-..."
  }
}
```

Process commands return:

```json
{
  "ok": true,
  "command": "process",
  "data": {
    "local_path": "./sprites/processed_20260330_120000.png",
    "width": 256,
    "height": 256,
    "operations_applied": ["remove_bg", "smart_crop:PowerOf2", "resize:256x256"],
    "elapsed_ms": 39000
  }
}
```

Process3d commands return:

```json
{
  "ok": true,
  "command": "process3d",
  "data": {
    "job_id": "uuid",
    "operation": "convert",
    "tripo_task_id": "tripo-task-456",
    "output_url": "https://upload.xiaomao.chat/uploads/model_123.fbx",
    "local_path": "./assets/model_123.fbx",
    "metadata": {
      "source_task_id": "tripo-task-123",
      "format": "FBX",
      "quad": true
    },
    "elapsed_ms": 5200,
    "cost_usd": 0.08
  }
}
```

**Key fields to use:**
- `data.local_path` — the saved file path (use this to reference the generated asset)
- `data.provider_id` — which provider handled the request (generate only)
- `data.metadata.tripo_task_id` — returned by `generate model`; pass this into the first `process3d --task-id`
- `data.tripo_task_id` — returned by `process3d`; use this to chain the next Tripo pipeline step
- `data.cost_usd` — estimated cost per generation (e.g. 0.04 for gemini image)
- `data.elapsed_ms` — generation/processing time in milliseconds
- `ok` — `true` on success, `false` on failure

## Error Recovery

```
ok=false → check error.code:
├── UNAUTHORIZED     → asset-gateway auth set <token>
├── PROVIDER_ERROR   → asset-gateway provider health
│   ├── provider unhealthy → try --provider <alternate>
│   └── all healthy       → retry (transient failure)
├── BAD_REQUEST      → fix prompt/params
└── network error    → check gateway URL, connectivity
```

**Automatic fallback**: When no `--provider` is specified, the gateway automatically tries the next healthy provider if the first one fails. You usually don't need manual retry logic.

## Job Tracking

For long-running tasks (video, `generate model`, `process3d`), check job history:

```bash
asset-gateway job list --limit 10
asset-gateway job list --status failed
asset-gateway job status <job-id>
asset-gateway job cancel <job-id>
```

## Upload & Reference Assets

Upload local files to get a public URL for use as input to generation or process commands:

```bash
# Upload a reference image → get URL
asset-gateway upload file ./reference.png
# Returns: { "url": "/uploads/uuid.png", "filename": "uuid.png" }

# Use the URL for image-to-3D
asset-gateway generate model --image "https://upload.xiaomao.chat/uploads/uuid.png" --output-dir ./out

# Use uploaded URL as process input
asset-gateway process remove-bg --input "https://upload.xiaomao.chat/uploads/uuid.png" --output-dir ./sprites
```

```bash
# List uploaded files
asset-gateway upload list

# Delete (admin only)
asset-gateway upload delete <filename>
```

Uploaded files are accessible at `https://upload.xiaomao.chat/uploads/<filename>` without authentication.

## Post-Generation Quality Validation (MANDATORY)

<HARD-GATE>
After EVERY asset generation call, you MUST validate the output before considering it done.
Never use `sips`, `convert`, or any tool to "fix" a failed generation into a placeholder.
A small/abstract/gradient file is a FAILURE, not a success that needs upscaling.
</HARD-GATE>

### Image Validation

After each `generate image` call, run this validation sequence:

```bash
# 1. Check file actually exists and has real content
SIZE=$(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null)
if [ "$SIZE" -lt 200000 ]; then
  echo "FAIL: $OUTPUT_FILE is only ${SIZE} bytes — likely a placeholder or failed generation"
  # DO NOT upscale or convert. Retry with asset-gateway instead.
fi

# 2. Check format matches extension
file "$OUTPUT_FILE" | grep -q "PNG image data" || echo "FAIL: not actually PNG"

# 3. Check resolution matches request
DIMS=$(file "$OUTPUT_FILE" | grep -oE '[0-9]+ x [0-9]+')
echo "Actual dimensions: $DIMS"
```

### Quality Thresholds

| Asset Type | Minimum File Size | Expected Size | If Below Threshold |
|-----------|-------------------|---------------|-------------------|
| Scene/Background (1920×1080) | 500 KB | 2-4 MB | FAIL → retry generation |
| Character sprite (1024×2048) | 200 KB | 600 KB-1 MB | FAIL → retry generation |
| CG illustration (1920×1080) | 1 MB | 2-4 MB | FAIL → retry generation |
| Title background | 500 KB | 2-4 MB | FAIL → retry generation |

### What To Do On Failure

1. **Do NOT** use `sips -s format png` to "convert" a JPEG placeholder to PNG
2. **Do NOT** use `sips --resampleHeightWidth` to upscale a tiny image
3. **Do NOT** use `ffmpeg` to create synthetic placeholder content
4. **DO** retry the `asset-gateway generate image` command (up to 3 times)
5. **DO** try a different `--provider` if the default keeps failing
6. **DO** report the failure honestly instead of masking it with post-processing

### Audio Validation

```bash
file "$OUTPUT_FILE" | grep -q "Ogg data" || echo "FAIL: not OGG format"
# If MP3, convert properly:
ffmpeg -i "$FILE" -acodec libvorbis -y "${FILE%.mp3}.ogg"
```

### Video Validation

```bash
DURATION=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$OUTPUT_FILE")
SIZE=$(stat -f%z "$OUTPUT_FILE" 2>/dev/null)
# A real AI-generated video should be > 500KB for even a few seconds
if [ "$SIZE" -lt 500000 ]; then
  echo "FAIL: video is only ${SIZE} bytes — likely placeholder"
fi
```

## Anti-Patterns

- **Don't call provider APIs directly** — always go through `asset-gateway`. It handles auth, routing, health checks, and fallback.
- **Don't skip `--output-dir`** — without it, generated files aren't saved locally.
- **Don't force `--provider` unless necessary** — let the gateway auto-route for best availability.
- **Don't retry blindly on PROVIDER_ERROR** — check `provider health` first to avoid wasting time on a down provider.
- **Don't use `generate text` for complex multi-turn conversations** — it's for single-shot completions only.
- **Don't manually remove backgrounds** — use `process remove-bg` for AI-powered removal with BiRefNet.
- **NEVER mask a failed generation by upscaling/converting a placeholder** — a 48KB abstract gradient upscaled to 1920×1080 is still garbage. Retry or report failure.
- **NEVER use `sips` to "fix" format mismatches from the gateway** — if gemini returns JPEG instead of PNG, that's a provider quirk to handle with proper conversion, not a signal to accept bad output.

## Schema Introspection

For automation, use `describe` to get JSON schemas of all commands:

```bash
asset-gateway describe                 # all commands
asset-gateway describe generate.image  # specific command
asset-gateway describe process         # process pipeline schema
asset-gateway describe process3d       # 3D pipeline schema
```

This returns input/output schemas suitable for programmatic tool integration.
