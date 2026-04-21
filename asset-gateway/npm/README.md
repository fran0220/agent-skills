# @doufunao123/asset-gateway

Lightweight npm CLI client for the universal asset generation gateway.

## Install

```bash
npm install -g @doufunao123/asset-gateway
```

Or run directly:

```bash
npx @doufunao123/asset-gateway provider list
```

## Quick Start

```bash
# 1. Save your API token
asset-gateway auth set agk_xxx

# 2. Generate assets
asset-gateway generate image --prompt "a red dragon"
asset-gateway generate video --prompt "ocean waves at sunset"
```

## Authentication

Supports `agk_` admin tokens or plain API keys.

**Resolution order:**

1. `--token <token>` CLI flag
2. `ASSET_GATEWAY_TOKEN` environment variable
3. `~/.config/asset-gateway/auth.json` (saved via `auth set`)

Config file format:

```json
{
  "token": "agk_...",
  "gateway_url": "https://asset.origingame.dev"
}
```

## Commands

```bash
# Auth
asset-gateway auth set <token>        # Save token locally
asset-gateway auth status             # Show current auth state
asset-gateway auth clear              # Remove saved credentials

# Generate
asset-gateway generate image --prompt "a cat" --size 1024x1024
asset-gateway generate image --prompt "icon" --transparent --provider flux
asset-gateway generate video --prompt "ocean waves"
asset-gateway generate audio --prompt "epic battle" --type bgm --duration 30
asset-gateway generate model --image https://example.com/ref.png
asset-gateway generate text --prompt "describe a forest" --model gpt-5.4

# Providers
asset-gateway provider list
asset-gateway provider health
asset-gateway provider health flux

# Jobs
asset-gateway job list
asset-gateway job list --status pending --limit 10
asset-gateway job status <id>
asset-gateway job cancel <id>

# Self-describe (JSON Schema)
asset-gateway describe
asset-gateway describe generate
```

## Output

JSON by default. Use `--human` for readable output, `--fields` to filter:

```bash
asset-gateway provider list --human
asset-gateway job status abc123 --fields "status,created_at"
```
