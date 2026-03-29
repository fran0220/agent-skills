---
name: asset-gateway
description: "Unified multi-provider asset generation (text, image, video, audio, 3D) via CLI. Use when the user asks to generate images, audio, video, 3D models, or text content — or when any project needs asset creation."
---

# Asset Gateway

`asset-gateway` is the **single CLI** for all asset generation. It routes requests to the best available provider automatically, with health-check filtering and fallback. Always prefer this over calling provider APIs directly.

## Install & Auth

```bash
npm install -g @doufunao123/asset-gateway
asset-gateway auth set <token>       # one-time, persisted to ~/.config/asset-gateway/auth.json
asset-gateway auth status            # verify
```

Default gateway: `https://assets.xiaomao.chat`. Override with `--gateway-url` or `ASSET_GATEWAY_URL`.

## Decision Guide: Which Command?

| User wants... | Command | Key flags |
|--------------|---------|-----------|
| An image from text | `generate image` | `--prompt`, `--transparent`, `--provider`, `--size` |
| A transparent PNG (icon/sprite) | `generate image` | `--prompt`, **`--transparent`** |
| A video clip | `generate video` | `--prompt` |
| Sound effect or BGM | `generate audio` | `--prompt`, `--type sfx\|bgm`, `--duration` |
| 3D model from text | `generate model` | `--prompt` |
| 3D model from image | `generate model` | `--image <url>` |
| LLM text completion | `generate text` | `--prompt`, `--model`, `--max-tokens` |

**All generate commands require `--output-dir`** to save files locally. The output `local_path` in the JSON response is the saved file path.

## Core Workflows

### Image Generation (most common)

```bash
# Default: auto-selects best provider (gemini_image, highest priority)
asset-gateway generate image --prompt "isometric village, clean lighting" --output-dir ./assets

# Transparent background (auto-routes to gpt_image)
asset-gateway generate image --prompt "game icon, potion bottle" --transparent --output-dir ./assets

# Force specific provider
asset-gateway generate image --prompt "anime forest" --provider gpt_image --output-dir ./assets

# With specific size
asset-gateway generate image --prompt "banner" --size 1792x1024 --output-dir ./assets
```

**Provider choice guide for images:**
- **Need transparency** → add `--transparent` (auto-routes to `gpt_image`)
- **Fast & cheap** → default (routes to `gemini_image`, ~15s)
- **Highest quality** → `--provider gpt_image` (~60-90s, slower but detailed)
- **即梦风格** → `--provider jimeng`

### Audio Generation

```bash
# Sound effect (default)
asset-gateway generate audio --prompt "sword slash impact" --output-dir ./assets

# Background music with duration
asset-gateway generate audio --prompt "ambient medieval tavern" --type bgm --duration 30 --output-dir ./assets
```

### Video Generation

```bash
# Auto-routes to available video provider (jimeng or grok_image)
asset-gateway generate video --prompt "camera slowly panning over a misty mountain" --output-dir ./assets
```

### 3D Model Generation

```bash
# Text to 3D
asset-gateway generate model --prompt "low-poly wooden chair" --output-dir ./assets

# Image to 3D (better results)
asset-gateway generate model --image "https://example.com/chair-ref.png" --output-dir ./assets
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
| `gemini_image` | image | ~15s | Default for images, cost-effective |
| `gpt_image` | image | ~60-90s | Transparency support, high detail |
| `jimeng` | image, video | varies | 即梦 + Seedance video |
| `grok_image` | image, video | varies | Grok imagine |
| `elevenlabs` | audio | ~2s | SFX and BGM, TTS |
| `tripo3d` | model3d | ~30-60s | Text/image to 3D |
| `llm_proxy` | text | ~1-3s | Claude, GPT, Gemini, Grok |

## Output Contract

Every command returns a JSON envelope:

```json
{
  "ok": true,
  "command": "generate.image",
  "data": {
    "job_id": "uuid",
    "provider_id": "gemini_image",
    "elapsed_ms": 15000,
    "local_path": "./assets/image_1234.png",
    "metadata": { "model": "gemini-3.1-flash-image-preview" }
  }
}
```

**Key fields to use:**
- `data.local_path` — the saved file path (use this to reference the generated asset)
- `data.provider_id` — which provider handled the request
- `data.elapsed_ms` — generation time in milliseconds
- `ok` — `true` on success, `false` on failure

Use `--fields local_path,provider_id` to get only specific fields.

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

For long-running tasks (video, 3D), check job history:

```bash
asset-gateway job list --limit 10
asset-gateway job list --status failed
asset-gateway job status <job-id>
asset-gateway job cancel <job-id>
```

## Upload & Reference Assets

Upload local files to get a public URL for use as input to generation commands:

```bash
# Upload a reference image → get URL
asset-gateway upload file ./reference.png
# Returns: { "url": "/uploads/uuid.png", "filename": "uuid.png" }

# Use the URL for image-to-3D
asset-gateway generate model --image "https://assets.xiaomao.chat/uploads/uuid.png" --output-dir ./out

# Use for Grok image editing (via generate with input_file)
asset-gateway generate image --prompt "add a hat" --provider grok_image --output-dir ./out
```

**Upload → Generate workflow:**
1. `upload file ./local-image.png` → get `url`
2. Prepend gateway URL: `https://assets.xiaomao.chat{url}`
3. Pass full URL to `--image` or provider-specific params

```bash
# List uploaded files
asset-gateway upload list

# Delete (admin only)
asset-gateway upload delete <filename>
```

Uploaded files are accessible at `https://assets.xiaomao.chat/uploads/<filename>` without authentication.

## Anti-Patterns

- **Don't call provider APIs directly** — always go through `asset-gateway`. It handles auth, routing, health checks, and fallback.
- **Don't skip `--output-dir`** — without it, generated files aren't saved locally.
- **Don't force `--provider` unless necessary** — let the gateway auto-route for best availability.
- **Don't retry blindly on PROVIDER_ERROR** — check `provider health` first to avoid wasting time on a down provider.
- **Don't use `generate text` for complex multi-turn conversations** — it's for single-shot completions only.

## Schema Introspection

For automation, use `describe` to get JSON schemas of all commands:

```bash
asset-gateway describe                 # all commands
asset-gateway describe generate.image  # specific command
```

This returns input/output schemas suitable for programmatic tool integration.
