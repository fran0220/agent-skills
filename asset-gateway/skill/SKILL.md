---
name: asset-gateway
description: "Unified asset generation and post-processing CLI for images, video, audio, music, TTS, voice cloning, 3D, text, and image/video tools. Use when an agent needs to create or transform assets through one stable command surface."
---

# Asset Gateway

`asset-gateway` is the single CLI for asset generation and post-processing. Think in terms of **asset category and command**, not provider choice.

## Install & Auth

```bash
npm install -g @doufunao123/asset-gateway
asset-gateway auth set <token>
asset-gateway auth status
```

Default gateway: `https://upload.xiaomao.chat`. Override with `--gateway-url` or `ASSET_GATEWAY_URL`.

## Capabilities

| Category | Use Case | Command |
|----------|----------|---------|
| Image | Generate, edit, reference, inpaint, restyle, expand | `generate image` |
| Video | Image-to-video (Jimeng Seedance 2.0 VIP) | `generate video` |
| Audio | SFX, BGM | `generate audio` |
| Music | Music generation (Lyria 3) | `generate music` |
| Speech | TTS, multilingual, instructed style | `generate tts` |
| Voice Identity | Voice clone, voice design | `voice clone`, `voice design` |
| 3D Model | Text/image → 3D, rig/animate/convert | `generate model`, `process3d ...` |
| Character Animation | Text/image → character animation (spritesheet, GIF, or MP4) via Vertex AI Veo | `generate sprite` |
| 3D World | Text/image → photorealistic 3D environment (Gaussian Splat) | `generate world` |
| Text | Single-shot LLM text generation | `generate text` |
| Batch | Batch generate multiple assets with shared params, optional auto-compose | `generate batch` |
| Image/Video Tools | Crop, resize, compose, extract frames, remove background | `process ...` |

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
| Generate SFX / BGM | `generate audio` | `--prompt`, `--type`, `--duration`, `--output-dir` |
| Generate music | `generate music` | `--prompt`, `--duration`, `--output-dir` |
| Text-to-speech | `generate tts` | `--prompt`, `--voice`, `--language`, `--output-dir` |
| Instructed TTS | `generate tts` | `--prompt`, `--instructions`, `--output-dir` |
| Clone a voice | `voice clone` | `--audio`, `--name` |
| Design a voice | `voice design` | `--prompt`, `--preview-text`, `--name` |
| Generate 3D model | `generate model` | `--prompt` or `--image`, `--output-dir` |
| Rig / animate 3D | `process3d rig`, `process3d animate` | `--task-id`, `--output-dir` |
| Convert 3D format | `process3d convert` | `--task-id`, `--format`, `--output-dir` |
| Crop transparent borders | `process crop` | `--input`, `--mode`, `--output-dir` |
| Resize image | `process resize` | `--input`, `--width`, `--height`, `--output-dir` |
| Compose sprite sheet | `process compose` | `--input <files...>`, `--direction`, `--output-dir` |
| Extract video frames | `process extract-frames` | `--input`, `--count`, `--output-dir` |
| Remove background | `process remove-bg` | `--input <files...>`, `--output-dir` |
| Animate a sprite | `generate sprite` | `--prompt`, `--animation-type`, `--view`, `--background`, `--output-dir` |
| Generate 3D world | `generate world` | `--prompt`, `--input <image>`, `--model`, `--output-dir` |
| Generate text | `generate text` | `--prompt`, `--model`, `--output-dir` |
| Batch generate (e.g. sprite frames) | `generate batch` | `--prompt <p1> <p2> ...`, `--asset-type`, `--compose`, `--output-dir` |

All `generate`, `process`, `process3d` commands should include `--output-dir` to save output locally.

If a command needs a URL and you have a local file, upload first:

```bash
asset-gateway upload file ./reference.png
```

## Detailed References

For detailed usage, examples, and workflows, read the reference files:

- **reference/generate.md** — Image, video, audio, music, TTS, text generation
- **reference/voice.md** — Voice clone, design, list, delete
- **reference/process.md** — Crop, resize, compose, extract-frames, remove-bg
- **reference/3d-pipeline.md** — 3D model generation and process3d chain
- **reference/sprite-workflow.md** — Full sprite animation pipeline (recommended approach)
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
