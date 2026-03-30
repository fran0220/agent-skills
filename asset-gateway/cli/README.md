# asset-gateway

Universal, pluggable asset generation gateway.

One binary, two modes:
- `asset-gateway serve` runs the gateway server
- other subcommands act as HTTP clients to the running gateway

`asset-gateway` provides a unified interface for:
- Text (LLM)
- Image
- Video
- Audio (BGM/SFX)
- 3D model generation

## Features

- Unified command surface: `generate`, `provider`, `credential`, `job`, `auth`
- Pluggable provider adapters
- SQLite persistence for users/providers/credentials/jobs
- AES-256-GCM credential vault
- JSON envelope output for agent automation

## Quick Start

### 1) Install

From this monorepo root:

```bash
cargo install --path cli/asset-gateway
```

For local development without install:

```bash
cargo run --manifest-path cli/asset-gateway/Cargo.toml -- --help
```

### 2) Start the gateway

```bash
asset-gateway serve --host 0.0.0.0 --port 6700 --db asset-gateway.db
```

### 3) Login

```bash
asset-gateway auth login --username admin --password 'your-password'
```

### 4) Generate an image

```bash
asset-gateway generate image --prompt "A stylized forest shrine" --size 1024x1024
```

## Command Reference

### `serve`

Run gateway service:

```bash
asset-gateway serve [--host 0.0.0.0] [--port 6700] [--db asset-gateway.db]
```

### `auth`

```bash
asset-gateway auth login [--url <gateway-url>] [--username <u>] [--password <p>]
asset-gateway auth logout
asset-gateway auth whoami
```

### `generate`

```bash
asset-gateway generate image --prompt <text> [--provider <id>] [--transparent] [--model <m>] [--size <wxh>] [-o <file>]
asset-gateway generate video --prompt <text> [--provider <id>] [-o <file>]
asset-gateway generate audio --prompt <text> [--type bgm|sfx] [--duration <sec>] [-o <file>]
asset-gateway generate model [--image <url-or-path> | --prompt <text>] [-o <file>]
asset-gateway generate text --prompt <text> [--model <m>] [--max-tokens <n>]
```

### `provider`

```bash
asset-gateway provider list
asset-gateway provider health [provider-id]
```

### `credential`

```bash
asset-gateway credential set <key> <value> [--provider <provider-id>]
asset-gateway credential list
asset-gateway credential delete <key>
```

### `job`

```bash
asset-gateway job list [--status <status>] [--limit 20]
asset-gateway job status <id>
asset-gateway job cancel <id>
```

### `describe`

```bash
asset-gateway describe
asset-gateway describe generate.image
```

## Environment Variables

Runtime and client configuration:

- `ASSET_GATEWAY_URL` (default: `http://localhost:6700`)
- `ASSET_GATEWAY_DB` (default: `asset-gateway.db`)
- `ASSET_GATEWAY_JWT_SECRET`
- `ASSET_GATEWAY_VAULT_KEY`
- `ASSET_GATEWAY_DATA_DIR`

Example:

```bash
export ASSET_GATEWAY_URL=http://localhost:6700
export ASSET_GATEWAY_DB=asset-gateway.db
export ASSET_GATEWAY_JWT_SECRET=change-me
export ASSET_GATEWAY_VAULT_KEY=change-me-32-char-key
export ASSET_GATEWAY_DATA_DIR=./data
```

## JSON Contract

All commands and API responses use the envelope:

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

Exit codes:

- `0` success
- `1` command error
- `2` system error
- `3` external provider/service error

## Providers (v1)

- `llm_proxy`: Claude/GPT/Gemini/Grok via proxy
- `gpt_image`: OpenAI image generation (supports transparency)
- `gemini_image`: Google image generation (cost-efficient)
- `grok_image`: Grok image generation/editing + video generation (xAI)
- `elevenlabs`: audio generation (BGM/SFX)
- `tripo3d`: image/text to 3D model

### Default Routing Strategy

- Transparent image requests -> `gpt_image`
- Non-transparent image requests -> `gemini_image`
- Explicit `--provider` override takes precedence
- If selected provider fails, fallback by priority

## Provider Management Workflow

1. Start gateway service.
2. Save required provider credentials with `credential set`.
3. Verify available providers via `provider list`.
4. Run `provider health` before production workloads.
5. Use `--provider` only when you need deterministic routing.

## Client Usage Notes

- `asset-gateway` client commands call HTTP endpoints on the gateway.
- If the server is not running, client commands fail fast.
- Use `--gateway-url` to target remote environments.

## Architecture Snapshot

- CLI: clap command parsing
- Server: Axum routes (`/auth/*`, `/api/*`)
- Core: dispatcher, provider registry, vault
- Storage: SQLite (`users`, `providers`, `credentials`, `jobs`)
- Adapters: reqwest-based provider integrations

## Roadmap

Planned for v2+:
- Leptos admin frontend
- WebSocket job progress push
- async queue for long-running video/3D jobs
- richer rate limiting and cost dashboard
