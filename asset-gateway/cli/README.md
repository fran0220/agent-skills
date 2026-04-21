# asset-gateway

Universal, pluggable asset generation gateway.

`asset-gateway` is now split cleanly into two parts:

- Rust binary: server only, started with `asset-gateway serve`
- npm package: `@doufunao123/asset-gateway`, the only supported CLI client

The server exposes one unified API for text, image, video, audio, music, speech, sprite animation, 3D models, and 3D worlds. The npm CLI is the user-facing command surface for all of those capabilities.

## Architecture

- Server runtime: Rust + Axum + PostgreSQL
- Config: `config.toml` with env override support
- Auth: JWT + API key + admin token
- Providers: pluggable adapters behind one dispatcher
- Client: npm CLI in `../npm`
- Admin UI: embedded HTML SPA at `/admin`

## Product Mapping

| Category | Backend | npm CLI command |
|----------|---------|------------------|
| Image | Gemini / GPT Image (transparent) | `asset-gateway generate image` |
| Video | Jimeng Seedance | `asset-gateway generate video` |
| Audio / SFX / BGM | ElevenLabs | `asset-gateway generate audio` |
| Music | Google Lyria 3 | `asset-gateway generate music` |
| TTS | Qwen / DashScope | `asset-gateway generate tts` |
| Voice clone / design | Qwen / DashScope | `asset-gateway voice ...` |
| 3D model | Tripo3D | `asset-gateway generate model`, `asset-gateway process3d ...` |
| Sprite animation | SpriteForge | `asset-gateway generate sprite` |
| 3D world | WorldLabs Marble | `asset-gateway generate world` |
| Text | LLM Proxy | `asset-gateway generate text` |

## Key Changes

- Rust no longer ships its own client command surface.
- SpriteForge now uses xAI Grok video generation, then `ffmpeg` frame extraction and white-background removal.
- `generate music` now uses Google Lyria 3 via the proxy gateway.
- ElevenLabs remains for `generate audio`, not `generate music`.
- A new `[xai]` config section is required for SpriteForge.

## Quick Start

### 1. Run the server

From this monorepo root:

```bash
cargo run --manifest-path asset-gateway/cli/Cargo.toml -- serve \
  --host 0.0.0.0 \
  --port 6700 \
  --database-url postgres://localhost/asset_gateway \
  --config /path/to/config.toml
```

Or build a release binary:

```bash
cargo build --manifest-path asset-gateway/cli/Cargo.toml --release
```

### 2. Install the npm CLI

```bash
npm install -g @doufunao123/asset-gateway
asset-gateway auth set <token>
```

### 3. Call the gateway

```bash
asset-gateway generate image --prompt "isometric village, soft morning light" --output-dir ./assets
asset-gateway generate music --prompt "uplifting indie game theme" --output-dir ./assets
asset-gateway generate sprite --prompt "pixel art knight" --animation-type walk --duration 2 --output-dir ./assets
```

## Server Dependencies

The gateway server depends on local tools for parts of the media pipeline:

- ImageMagick 6 for crop / resize / compose
- `ffmpeg` for frame extraction and SpriteForge post-processing
- Blender for `process3d render-sprites`

## Config

Runtime config is loaded from `config.toml`, with environment variables taking precedence.

```toml
[server]
admin_token = "agk_admin_..."

[proxy]
url = "https://api.xiaomao.chat"
key = "..."
default_model = "claude-sonnet-4-6"

[elevenlabs]
key = "..."

[tripo3d]
key = "..."

[dashscope]
url = "https://dashscope-intl.aliyuncs.com"
key = "..."

[grok2api]
url = "https://grok.xiaomao.chat"
key = "..."

[jimeng]
url = "http://127.0.0.1:5100"
token = "..."

[worldlabs]
key = "..."

[xai]
key = "..."
```

## Providers

Current documented provider set:

- `llm_proxy` for text generation
- `gemini_image` for default image generation and editing
- `gpt_image` for transparent image output
- `jimeng` for image + Seedance video
- `qwen_tts` for TTS and custom voice workflows
- `elevenlabs` for audio / SFX / BGM
- `lyria` for music generation
- `tripo3d` for 3D generation and processing
- `spriteforge` for sprite animation generation
- `worldlabs` for Gaussian Splat world generation

## API Contract

The server returns a unified JSON envelope:

```json
{
  "ok": true,
  "command": "generate.image",
  "data": {}
}
```

Error shape:

```json
{
  "ok": false,
  "command": "generate.image",
  "error": {
    "code": "PROVIDER_ERROR",
    "message": "provider timeout"
  }
}
```

## Development

From the repo root, the main checks are:

```bash
cargo fmt
cargo test --manifest-path asset-gateway/cli/Cargo.toml --lib
cargo clippy --manifest-path asset-gateway/cli/Cargo.toml
cargo fmt --manifest-path asset-gateway/cli/Cargo.toml --check
```

## Deployment

- Server: `Oracle` (`161.33.13.122`)
- Binary: `/opt/asset-gateway/asset-gateway`
- Config: `/opt/asset-gateway/config.toml`
- Port: `6700`
- Domain: `asset.origingame.dev`
- Service manager: `systemd`
- Deploy path: GitHub Actions on push to `main`

## More Docs

- Development guide: `./AGENTS.md`
- Design source of truth: `./BLUEPRINT.md`
- Agent skill docs: `../skill/SKILL.md`
