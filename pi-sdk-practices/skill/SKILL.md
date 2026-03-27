---
name: pi-sdk-practices
description: Captures Origin's verified Pi SDK integration patterns, migration rules, and pitfalls. Use when building or refactoring Pi SDK sessions, extensions, skills, RPC bridges, and provider wiring in this repository.
---

# Pi SDK Practices

Use Pi SDK as the runtime, not as a library hidden behind another agent framework.

## When To Use This Skill

- Building new agent runtime features with `@mariozechner/pi-coding-agent`.
- Refactoring legacy `origin-agent/` glue code to native Pi workspace conventions.
- Designing extension hooks, worker orchestration, review gates, and memory hooks.
- Integrating external systems through Pi RPC mode.
- Registering proxy providers and model routing policy.

## Verified Baseline

- Local project dependency: `@mariozechner/pi-coding-agent@^0.56.2` (`origin-agent/package.json`).
- Latest upstream release at authoring time: `0.62.0` (`npm view @mariozechner/pi-coding-agent version`).
- Facts in this skill are grounded in local installed source under `origin-agent/node_modules/@mariozechner/pi-coding-agent/dist/`.
- Upstream docs are used as supplemental guidance, not as replacement for local source reality.

## Official/Community References

- Official docs: `docs/sdk.md`, `docs/extensions.md`, `docs/rpc.md`, `docs/providers.md`, `docs/settings.md` in `github.com/badlogic/pi-mono`.
- Official changelog: `packages/coding-agent/CHANGELOG.md`.
- Community integration references used for comparison:
  - `github.com/clawdbot/clawdbot`
  - `github.com/openclaw/openclaw`
  - `github.com/dnouri/pi-coding-agent` (Emacs frontend)

## Non-Negotiables

- Do not add a custom WS orchestration layer when Pi RPC (`runRpcMode` or `RpcClient`) already satisfies integration needs.
- Prefer `.pi/extensions/` file-based extension loading over `extensionFactories` injection.
- Prefer `.pi/skills/` discovery over manual skill path plumbing.
- Keep provider registration in extension files (`api.registerProvider`) instead of app bootstrap code.
- Treat AGENTS/CLAUDE context loading as Pi-native behavior; avoid duplicating it in app code.

## Standard Workflow

1. Start with workspace-native structure under `.pi/`.
2. Implement behavior as extension files (`.pi/extensions/*.ts`).
3. Implement instructions as skills (`.pi/skills/**/SKILL.md`).
4. Wire external systems through RPC mode instead of a custom agent server.
5. Validate against local `.d.ts` contracts before coding handlers.
6. Check upstream changelog only for upgrade planning.

## Origin Migration Checklist

1. Move the 6 Origin extensions into standalone files under `.pi/extensions/`:
   - `memory.ts`, `review.ts`, `worker.ts`, `image-gen.ts`, `web-search.ts`, `visual.ts`.
2. Remove extension factory aggregation in `origin-agent/src/agent.ts`.
3. Remove manual skill listing logic (`listAvailableSkills`) and trust Pi discovery.
4. Remove manual provider bootstrap in runtime init; register providers via extension.
5. Replace WS wrapper orchestration with Pi RPC client/server protocol.
6. Keep role/model policy in one place (provider extension + model strategy doc).

## Where To Read Next

- `reference/workspace-configuration.md`
- `reference/extension-patterns.md`
- `reference/rpc-integration.md`
- `reference/provider-registration.md`
- `reference/skill-authoring.md`
- `reference/anti-patterns.md`
- `reference/model-selection.md`

## Quick Audit Commands

```bash
# Confirm installed version
npm view @mariozechner/pi-coding-agent version

# Inspect extension contracts
rg "export interface ToolCallEventResult|ExtensionAPI" origin-agent/node_modules/@mariozechner/pi-coding-agent/dist/core/extensions/types.d.ts

# Inspect RPC command surface
rg "type: \"prompt\"|type: \"get_commands\"" origin-agent/node_modules/@mariozechner/pi-coding-agent/dist/modes/rpc/rpc-types.d.ts

# Inspect workspace auto-discovery behavior
rg "addAutoDiscoveredResources|collectAutoSkillEntries|collectAutoExtensionEntries" origin-agent/node_modules/@mariozechner/pi-coding-agent/dist/core/package-manager.js
```

## Upgrade Note (0.56.2 -> 0.62.x)

- Newer versions introduce `sourceInfo` metadata changes in RPC and extension APIs.
- Validate API compatibility before upgrading any Origin integration code that reads command/skill metadata.
