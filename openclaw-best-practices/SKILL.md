---
name: openclaw-best-practices
description: "Comprehensive OpenClaw operations guide for secure setup, routing, models, memory, sandboxing, skills, heartbeat, and cron. Use when setting up, auditing, deploying, or extending OpenClaw gateways and agents."
---

# OpenClaw Best Practices

OpenClaw is a self-hosted gateway for running AI agents across chat channels.
This skill is an operations playbook for secure and reliable setup, auditing, and extension.

## Verified Baseline (2026-02-23)

Re-validated against official docs and live package metadata.

- npm package: `openclaw@2026.2.22-2` (`latest` and `beta`)
- Node requirement: `>=22.12.0`
- Core sources:
  - `https://docs.openclaw.ai/cli`
  - `https://docs.openclaw.ai/gateway/configuration`
  - `https://docs.openclaw.ai/gateway/configuration-reference`
  - `https://docs.openclaw.ai/concepts/model-failover`
  - `https://docs.openclaw.ai/concepts/memory`
  - `https://docs.openclaw.ai/concepts/multi-agent`
  - `https://docs.openclaw.ai/channels/channel-routing`
  - `https://docs.openclaw.ai/gateway/sandboxing`
  - `https://docs.openclaw.ai/gateway/sandbox-vs-tool-policy-vs-elevated`
  - `https://docs.openclaw.ai/tools/skills`
  - `https://docs.openclaw.ai/tools/skills-config`
  - `https://docs.openclaw.ai/gateway/heartbeat`
  - `https://docs.openclaw.ai/automation/cron-jobs`
  - `https://docs.openclaw.ai/help/environment`
  - `https://github.com/openclaw/openclaw`

Quick re-check commands:

```bash
npm view openclaw version dist-tags engines --json
openclaw --version
openclaw doctor
openclaw security audit
```

## Required Workflow

For setup, debugging, or architecture changes, run this sequence:

1. Confirm runtime + install health:
   - `openclaw --version`
   - `node --version`
   - `openclaw doctor`
2. Run security baseline before feature work:
   - `openclaw security audit`
   - `openclaw security audit --deep`
3. Inspect active runtime:
   - `openclaw gateway status --deep`
   - `openclaw models status`
   - `openclaw skills list --eligible`
   - `openclaw sandbox explain`
4. Make minimal config changes.
5. Re-run audit + status checks after changes.
6. For unclear keys, use `gateway/configuration-reference` first.

## Architecture Mental Model

`channels -> gateway -> routing -> agent runtime -> tools -> sessions/memory -> outbound delivery`

- Gateway is control plane; agents are isolated by workspace, state, auth profiles, and sessions.
- Routing is deterministic and binding-order sensitive.
- Tool policy and sandboxing are separate control layers.
- Security posture is mostly about access boundaries, not prompt wording.

## Security First

Start with least privilege and widen only as needed.

1. Lock ingress first:
   - use pairing/allowlists
   - avoid broad `dmPolicy: "open"` with powerful tools
2. Isolate DMs for shared inboxes:
   - `session.dmScope: "per-channel-peer"`
   - for multi-account channels use `per-account-channel-peer`
3. Protect gateway surface:
   - loopback bind by default
   - token/password auth for non-loopback
4. Constrain execution:
   - enable sandbox (`non-main` or `all`)
   - keep `workspaceAccess` at `none` or `ro` by default
5. Restrict high-risk tools for untrusted surfaces:
   - `exec`, `browser`, `web_search`, `web_fetch`, `gateway`, `cron`
6. Prefer strong current models for tool-enabled agents.

Useful commands:

```bash
openclaw security audit
openclaw security audit --deep
openclaw security audit --fix
```

## Models and Failover

OpenClaw failover occurs in two phases:

1. Rotate auth profiles inside current provider.
2. Move through `agents.defaults.model.fallbacks`.

Important details:

- Primary model path: `agents.defaults.model.primary`
- Fallback path: `agents.defaults.model.fallbacks`
- Metadata routing: `auth.profiles` and `auth.order`
- Secret/auth state: `~/.openclaw/agents/<agentId>/agent/auth-profiles.json`
- Typical cooldown ladder: `1m -> 5m -> 25m -> 1h`
- Billing disables use longer backoffs.

CLI:

```bash
openclaw models list
openclaw models status --probe
openclaw models set <provider/model>
openclaw models fallbacks add <provider/model>
openclaw models auth add
```

## Memory and Compaction

Memory source of truth is Markdown in workspace.

- Daily logs: `memory/YYYY-MM-DD.md`
- Curated memory: `MEMORY.md`
- Search config is under `agents.defaults.memorySearch`.
- Compaction memory flush is under `agents.defaults.compaction.memoryFlush`.
- Session transcripts are disk data under `~/.openclaw/agents/<agentId>/sessions/*.jsonl`.

Optional backends and features:

- `memory.backend: "qmd"` for QMD sidecar
- hybrid retrieval and rerank settings under `memorySearch.query.hybrid`
- session indexing is opt-in (`memorySearch.experimental.sessionMemory`)

## Multi-Agent Routing

Use separate agents for real isolation.

- Configure `agents.list[]` with dedicated workspaces.
- Use `bindings[]` for deterministic routing.
- Place most-specific rules first.

Practical order:

1. `peer`
2. `parentPeer`
3. `guildId + roles`
4. `guildId`
5. `teamId`
6. `accountId`
7. channel-wide
8. default agent

CLI:

```bash
openclaw agents add <name>
openclaw agents list --bindings
openclaw channels status --probe
```

## Sandbox, Tool Policy, and Elevated

Treat these as separate layers:

1. Sandbox (`agents.defaults.sandbox`, `agents.list[].sandbox`): where execution runs
2. Tool policy (`tools.*`, `tools.byProvider`, `tools.sandbox.tools`, `agents.list[].tools`): what can run
3. Elevated (`tools.elevated`): exec-only host escape path

Critical reminders:

- `deny` wins over `allow`.
- `tools.elevated` is sender-gated and exec-only, not universal privilege.
- Sandbox sessions do not inherit host process env by default.
- Sandboxed skill binaries and env must exist inside sandbox runtime.

Debug:

```bash
openclaw sandbox explain
openclaw sandbox explain --json
```

## Heartbeat and Cron

Heartbeat:

- configured in `agents.defaults.heartbeat` or `agents.list[].heartbeat`
- supports `activeHours`, `target`, `to`, `accountId`, `includeReasoning`
- `HEARTBEAT_OK` is treated as suppressible ack token

Cron:

- persistent scheduler in gateway (`cron` section)
- `sessionTarget: "main"` for system-event style runs
- `sessionTarget: "isolated"` for dedicated cron session turns
- delivery modes: `announce`, `webhook`, `none`

Commands:

```bash
openclaw system event --text "..." --mode now
openclaw cron add --name "..." --cron "0 7 * * *" --session isolated --message "..."
openclaw cron list
openclaw cron runs --id <jobId>
```

## Skills System

Loading precedence (highest to lowest):

1. `<workspace>/skills`
2. `~/.openclaw/skills`
3. bundled skills
4. `skills.load.extraDirs`

Key config:

- `skills.allowBundled`
- `skills.load.extraDirs`
- `skills.load.watch`
- `skills.load.watchDebounceMs`
- `skills.install.nodeManager`
- `skills.entries.<skill>.enabled/env/apiKey/config`

Important caveat:

- `skills.entries.*.env` and `apiKey` inject into host run context.
- sandboxed skill execution needs sandbox env/image provisioning separately.

Inspect:

```bash
openclaw skills list
openclaw skills list --eligible
openclaw skills info <name>
openclaw skills check
```

## Common Mistakes

- Treating `tools.elevated` as generic policy instead of exec-only host path.
- Leaving DM policies open while enabling risky tools.
- Misreading `non-main` sandbox mode as "non-main agent" instead of session key behavior.
- Assuming host env injection automatically appears in sandbox containers.
- Forgetting to isolate DM scope for multi-user inboxes.
- Editing cron job store files manually while gateway is running.
- Using stale docs path `concepts/channel-routing` instead of `channels/channel-routing`.

## Reference Files

Read these local references for implementation details:

- [Configuration Examples](reference/configuration-examples.md)
- [Validated Baseline](reference/validated-baseline.md)
