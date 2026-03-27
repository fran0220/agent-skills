# asset-gateway Skill

This Skill teaches agents how to use the `asset-gateway` CLI as a unified interface for multi-provider asset generation.

## What This Skill Covers

- prerequisite checks for gateway and CLI
- auth flow (`auth login`, `whoami`)
- generation workflows for image/video/audio/model/text
- provider selection strategy and override rules
- credential management and safe handling
- JSON-envelope-based error handling and retry flow

## Intended Use

Use this Skill when an agent needs to generate assets without directly integrating provider-specific SDKs or APIs.

Typical triggers:
- "generate game assets"
- "use one gateway for LLM + image + audio + 3D"
- "switch providers without rewriting the workflow"

## Prerequisites

1. `asset-gateway` CLI is installed.
2. Gateway is running via `asset-gateway serve`.
3. Required env vars are configured:
   - `ASSET_GATEWAY_URL`
   - `ASSET_GATEWAY_DB`
   - `ASSET_GATEWAY_JWT_SECRET`
   - `ASSET_GATEWAY_VAULT_KEY`
   - `ASSET_GATEWAY_DATA_DIR`

## Main References

- Skill entry: `skills/asset-gateway/SKILL.md`
- CLI docs: `cli/asset-gateway/README.md`
- CLI blueprint: `cli/asset-gateway/BLUEPRINT.md`
- CLI agent constraints: `cli/asset-gateway/AGENTS.md`
