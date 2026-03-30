# Design Blueprint — `asset-gateway`

> Universal, pluggable asset generation gateway. One service and one CLI surface for text, image, video, audio, and 3D model generation.

## 1. Purpose

Provide a single gateway that normalizes multi-provider asset generation into one stable interface for agents and operators.

`asset-gateway` unifies:
- LLM text generation
- image generation
- video generation
- audio generation (BGM/SFX)
- 3D model generation

Primary goals:
- one auth flow
- one command surface
- one JSON envelope contract
- pluggable provider backend

## 2. Classification

- **Primary role:** Service + CLI Gateway
- **Primary user type:** Agent-Primary + Human Admin
- **Primary interaction form:** HTTP API + Batch CLI
- **Statefulness:** DB-Stateful (SQLite)
- **Risk profile:** Mixed (credential handling is high-risk, generation orchestration is medium-risk)
- **Confidence level:** High

### 2b. Classification reasoning

**Why Service + CLI Gateway.**
The product center is a long-lived gateway process (`serve`) plus a thin CLI client that talks to it. This is not a pure local utility CLI.

**Why Agent-Primary + Human Admin.**
Agents call `generate`, `describe`, and automation commands. Humans handle provider setup, credential provisioning, and incident response.

**Why DB-Stateful.**
Runtime state must persist beyond process lifetime: users, providers, encrypted credentials, and jobs are durable records in SQLite.

## 3. Primary design stance

Use a **single Rust binary with dual mode**:

1. `asset-gateway serve` runs the HTTP gateway service.
2. All other subcommands act as HTTP clients against a running gateway.

This keeps deployment and usage simple:
- one executable
- one operational model
- one contract between automation and service

## 4. Command structure

```
asset-gateway serve       --host --port --db
asset-gateway auth        login|logout|whoami
asset-gateway generate    image|video|audio|model|text
asset-gateway provider    list|health
asset-gateway credential  set|list|delete
asset-gateway job         list|status|cancel
asset-gateway describe    [command]
```

Command intent:
- `serve`: run API server
- `auth`: establish and inspect identity
- `generate`: submit generation requests
- `provider`: inspect backend availability
- `credential`: manage encrypted provider secrets
- `job`: inspect and control persisted jobs
- `describe`: schema/command introspection

## 5. Input model

### CLI input

- Global endpoint: `--gateway-url` or `ASSET_GATEWAY_URL`
- Subcommand flags for common calls
- `describe` for machine-friendly command discovery

### API input

Primary generation payload shape:

```json
{
  "asset_type": "image|video|audio|model3d|text",
  "prompt": "optional",
  "model": "optional",
  "input_file": "optional",
  "provider": "optional provider override",
  "params": {}
}
```

Routing input principles:
- explicit provider override wins
- otherwise strategy routing picks best provider by asset type + capability + priority

## 6. Output model

Default output is JSON envelope:

```json
{
  "ok": true,
  "command": "generate.image",
  "data": {}
}
```

Error output:

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

Contract principles:
- stable envelope: `ok`, `command`, `data|error`
- machine-parseable error codes
- human-readable message retained

Exit code policy:
- `0` success
- `1` command error
- `2` system error
- `3` external service/provider error

## 7. Discoverability / introspection

`describe` is the primary discovery surface for agents:

```bash
asset-gateway describe
asset-gateway describe generate.image
asset-gateway describe provider.health
```

`--help` remains available for human diagnostics, but schema-oriented introspection is preferred for automation.

## 8. State model

SQLite-backed persistent state (DB-Stateful):

- `users` — identity, role, API keys
- `providers` — enabled adapters, config, priority
- `credentials` — encrypted secrets + nonce + provider association
- `jobs` — request lifecycle, status, outputs, error, timing

Configuration and runtime settings:
- `ASSET_GATEWAY_DB`
- `ASSET_GATEWAY_JWT_SECRET`
- `ASSET_GATEWAY_VAULT_KEY`
- `ASSET_GATEWAY_DATA_DIR`

## 9. Risk / safety model

### Security controls

- Credentials encrypted at rest with AES-256-GCM
- Random nonce per encryption operation
- Auth boundary between normal user and admin operations
- JWT/API key based authentication model for API access

### Operational controls

- Provider-level health checks
- Provider fallback behavior for resilience
- Job persistence for traceability and recovery

### Routing policy (v1)

- transparent images -> `gpt_image`
- non-transparent images -> `gemini_image` (cost-efficient default)
- explicit `--provider` override takes precedence
- if selected provider fails, use priority-based fallback

## 10. Provider model

v1 provider adapters:

1. `llm_proxy` — Claude/GPT/Gemini/Grok/GLM via proxy
2. `gpt_image` — OpenAI image generation (transparency capable)
3. `gemini_image` — Google image generation (cost-effective)
4. `elevenlabs` — audio generation (BGM/SFX)
5. `tripo3d` — image/text to 3D model

Provider architecture uses a unified `AssetProvider` trait so adapters remain swappable and independently testable.

## 11. v1 boundaries

v1 includes:
- dual-mode single binary (`serve` + client commands)
- auth foundation (JWT + API key model)
- encrypted credential CRUD
- provider registry + dynamic loading path
- job persistence
- strategy-based dispatcher
- six provider adapters listed above

v1 excludes:
- web admin frontend
- websocket push updates
- asynchronous long-job queue workers
- cost dashboard and advanced analytics
- global rate-limit governance layer

## 12. v2+ deferred roadmap

- Leptos frontend for admin operations
- WebSocket task progress push
- async queue workers for long-running video/3D workloads
- richer rate limiting and quota policies
- cost tracking and usage dashboard

## 13. Technology stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Performance, reliability, async ecosystem |
| HTTP server | Axum | Modern typed routing + middleware support |
| Database | SQLite + sqlx | Embedded persistent state + migrations |
| CLI | clap | Strong subcommand and env support |
| HTTP client | reqwest | Provider adapter integration |
| Auth | jsonwebtoken + argon2 | JWT issuance/validation + secure password hashing |
| Secret vault | AES-256-GCM | Authenticated encryption for provider credentials |
| Runtime | tokio | Async execution model for API + adapters |
