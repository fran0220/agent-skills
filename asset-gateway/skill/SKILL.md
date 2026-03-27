---
name: asset-gateway
description: "Provides a standardized workflow for using the asset-gateway CLI to authenticate and generate text, image, video, audio, and 3D assets. Use when an agent needs unified multi-provider asset generation through one gateway service."
---

# Asset Gateway Skill

Use this skill to operate `asset-gateway` as the single access layer for generation providers.

## When To Use

Trigger this skill when the user asks to:
- generate text/image/video/audio/3D assets via one interface
- avoid managing multiple provider SDKs directly
- set up or operate a shared asset generation gateway
- standardize auth + routing + provider fallback behavior

## Prerequisites

1. `asset-gateway` CLI is installed.
2. Gateway service is running.
3. Required environment variables are configured when serving:
   - `ASSET_GATEWAY_DB`
   - `ASSET_GATEWAY_JWT_SECRET`
   - `ASSET_GATEWAY_VAULT_KEY`
   - `ASSET_GATEWAY_DATA_DIR`
4. Client points to correct endpoint (`ASSET_GATEWAY_URL` or `--gateway-url`).

Basic checks:

```bash
asset-gateway --version
asset-gateway describe
asset-gateway provider list
```

## Workflow

### Step 1: Ensure gateway is running

```bash
asset-gateway serve --host 0.0.0.0 --port 6700 --db asset-gateway.db
```

If service is remote, set:

```bash
export ASSET_GATEWAY_URL=http://<host>:6700
```

### Step 2: Authenticate

```bash
asset-gateway auth login --username <username> --password <password>
asset-gateway auth whoami
```

If credentials/token are invalid, re-login before running generate commands.

### Step 3: Validate provider readiness

```bash
asset-gateway provider list
asset-gateway provider health
```

If a specific provider is required, check it directly:

```bash
asset-gateway provider health gpt_image
```

### Step 4: Generate assets

#### Image

```bash
asset-gateway generate image --prompt "isometric village, clean lighting" --size 1024x1024
```

Transparent image request:

```bash
asset-gateway generate image --prompt "game icon, transparent background" --transparent
```

Provider override:

```bash
asset-gateway generate image --prompt "anime forest" --provider gpt_image
```

#### Video

```bash
asset-gateway generate video --prompt "cinematic fly-through of floating islands"
```

#### Audio

```bash
asset-gateway generate audio --prompt "8-bit battle loop" --type bgm --duration 30
asset-gateway generate audio --prompt "menu click" --type sfx
```

#### 3D Model

Image-to-3D:

```bash
asset-gateway generate model --image "https://example.com/reference.png"
```

Text-to-3D:

```bash
asset-gateway generate model --prompt "low-poly treasure chest"
```

#### Text (LLM)

```bash
asset-gateway generate text --prompt "Write enemy lore for a desert biome" --model gpt-5.4 --max-tokens 1200
```

### Step 5: Track jobs

```bash
asset-gateway job list --limit 20
asset-gateway job status <job-id>
asset-gateway job cancel <job-id>
```

### Step 6: Use schema introspection for automation

```bash
asset-gateway describe
asset-gateway describe generate.image
asset-gateway describe credential.set
```

## Provider Selection Strategy

Default strategy:
1. Explicit `--provider` always wins.
2. Transparent image requests prefer `gpt_image`.
3. Non-transparent image requests default to `gemini_image` for cost efficiency.
4. If chosen provider fails, fallback by provider priority.

Recommended usage:
- Do not force provider unless user asks for deterministic backend behavior.
- Use provider overrides for debugging, A/B evaluation, or compliance constraints.

## Credential Management

Store provider secrets through gateway credential APIs:

```bash
asset-gateway credential set LLM_PROXY_KEY <value> --provider llm_proxy
asset-gateway credential set OPENAI_API_KEY <value> --provider gpt_image
asset-gateway credential set GEMINI_API_KEY <value> --provider gemini_image
asset-gateway credential set JIMENG_API_KEY <value> --provider jimeng
asset-gateway credential set ELEVENLABS_API_KEY <value> --provider elevenlabs
asset-gateway credential set TRIPO3D_API_KEY <value> --provider tripo3d
asset-gateway credential list
```

Never print or log raw credential values in agent responses.

## Error Handling

Use the JSON envelope fields to drive recovery:

- `ok=false` + `error.code=BAD_REQUEST`: fix input and retry.
- `ok=false` + `error.code=UNAUTHORIZED`: refresh login/token.
- `ok=false` + `error.code=PROVIDER_ERROR`: check provider health, then retry or force alternate provider.
- timeout/network failure: verify `ASSET_GATEWAY_URL`, service liveness, and outbound provider connectivity.

Preferred retry order:
1. Validate endpoint/auth.
2. Re-run `provider health`.
3. Retry with same provider.
4. Retry with explicit fallback provider.

## Output Contract

Expect:

```json
{
  "ok": true,
  "command": "generate.image",
  "data": {}
}
```

or:

```json
{
  "ok": false,
  "command": "generate.image",
  "error": {
    "code": "PROVIDER_ERROR",
    "message": "..."
  }
}
```

Treat envelope fields as the stable machine contract.
