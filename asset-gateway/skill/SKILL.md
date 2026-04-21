---
name: asset-gateway
description: "Unified asset generation and post-processing CLI for images, video, sound effects, music, TTS, 3D, text, and image/video tools. Use when an agent needs to create or transform assets through one stable command surface."
---

# Asset Gateway

`asset-gateway` is the single CLI for asset generation and post-processing. Think in terms of **asset category and command**, not provider choice.

## Install & Auth

```bash
npm install -g @doufunao123/asset-gateway
asset-gateway auth set <token>
asset-gateway auth status
```

Gateway URL is hardcoded to `https://asset.origingame.dev`. Only the API token is needed.

## Capabilities

| Category | Use Case | Command | Provider |
|----------|----------|---------|----------|
| Image | Generate, edit, reference, inpaint, restyle, expand | `generate image` | GPT Image (primary, with fallback) / Grok (non-transparent fallback) |
| Video | Text/image-to-video | `generate video` | Grok (primary) / Veo (Vertex AI) / Jimeng |
| Sound Effects | SFX, ambient sounds | `generate sfx` | ElevenLabs |
| Music | Music generation | `generate music` | Lyria 3 (Vertex AI) |
| Speech | TTS with 21 prebuilt voices, multi-speaker | `generate tts` | Gemini 3.1 Flash TTS |
| Sprite | Character animation spritesheet | `generate sprite` | AutoSprite |
| 3D Model | Text/image → 3D, rig/animate/convert | `generate model`, `process3d ...` | Tripo3D |
| 3D World | Photorealistic 3D environment (Gaussian Splat) | `generate world` | WorldLabs Marble |
| Text | Single-shot LLM text generation | `generate text` | LLM Proxy |
| Batch | Batch generate with shared params, auto-compose | `generate batch` | (any) |
| Image/Video Tools | Crop, resize, compose, extract frames, remove bg | `process ...` | Local |

## Quick Decision Guide

| I want to... | Command | Key flags |
|--------------|---------|-----------|
| Generate an image | `generate image` | `--prompt`, `--size`, `--output-dir` |
| Generate transparent PNG | `generate image` | `--prompt`, `--transparent`, `--output-dir` |
| Edit an existing image | `generate image` | `--prompt`, `--input <url>`, `--output-dir` |
| Use reference images | `generate image` | `--prompt`, `--ref <url> [<url>...]`, `--output-dir` |
| Inpaint / restyle / expand | `generate image` | `--prompt`, `--input`, `--edit-mode`, `--output-dir` |
| Continue editing (multi-turn) | `generate image` | `--prompt`, `--session <id>`, `--output-dir` |
| Generate video | `generate video` | `--prompt`, `--input <image_url>`, `--output-dir` |
| Generate sound effects | `generate sfx` | `--prompt`, `--duration` (required! 1-5s short, 5-15s ambience), `--output-dir` |
| Generate music | `generate music` | `--prompt`, `--duration`, `--output-dir` |
| Text-to-speech | `generate tts` | `--prompt`, `--voice`, `--output-dir` |
| Generate sprite animation | `generate sprite` | `--prompt`, `--animation-type`, `--output-dir` |
| Generate 3D model | `generate model` | `--prompt` or `--image`, `--output-dir` |
| Rig / animate 3D | `process3d rig`, `process3d animate` | `--task-id`, `--output-dir` |
| Convert 3D format | `process3d convert` | `--task-id`, `--format`, `--output-dir` |
| Generate 3D world | `generate world` | `--prompt`, `--input <image>`, `--model`, `--output-dir` |
| Crop transparent borders | `process crop` | `--input`, `--mode`, `--output-dir` |
| Resize image | `process resize` | `--input`, `--width`, `--height`, `--output-dir` |
| Compose sprite sheet | `process compose` | `--input <files...>`, `--direction`, `--output-dir` |
| Extract video frames | `process extract-frames` | `--input`, `--count`, `--output-dir` |
| Remove background | `process remove-bg` | `--input <files...>`, `--output-dir` |
| Generate text | `generate text` | `--prompt`, `--model`, `--output-dir` |
| Batch generate | `generate batch` | `--prompt <p1> <p2> ...`, `--asset-type`, `--compose`, `--output-dir` |

All `generate`, `process`, `process3d` commands should include `--output-dir` to save output locally.

If a command needs a URL and you have a local file, upload first:

```bash
asset-gateway upload file ./reference.png
```

## Detailed References

For detailed usage, examples, and workflows, read the reference files:

- **reference/generate.md** — Image, video, SFX, music, TTS, text generation
- **reference/process.md** — Crop, resize, compose, extract-frames, remove-bg
- **reference/3d-pipeline.md** — 3D model generation and process3d chain
- **reference/sprite-workflow.md** — Sprite animation via AutoSprite
- **reference/world-workflow.md** — 3D world generation with WorldLabs Marble

## Schema Introspection

```bash
asset-gateway describe                  # all schemas
asset-gateway describe generate.image   # specific command
asset-gateway describe process
```

## Troubleshooting

- Check auth: `asset-gateway auth status`
- Check gateway: `asset-gateway provider list`, `asset-gateway provider health`
- Track long tasks: `asset-gateway job list`, `asset-gateway job status <id>`
- Upload local files first if command needs URL: `asset-gateway upload file`
- `UNAUTHORIZED` → re-run `asset-gateway auth set <token>`
- `PROVIDER_ERROR` → check `provider health`, then retry
- Always include `--output-dir` to get local file output
