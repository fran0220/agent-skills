---
name: openclaw-best-practices
description: "Comprehensive OpenClaw operations guide for secure setup, routing, models, memory, sandboxing, skills, hooks, ACP agents, ContextEngine, and automation. Use when setting up, auditing, deploying, or extending OpenClaw gateways and agents."
---

# OpenClaw Best Practices

OpenClaw is a self-hosted gateway for running AI agents across chat channels.
This skill is an operations playbook for secure and reliable setup, auditing, and extension.

## Verified Baseline (2026-03-15)

Re-validated against official docs, npm metadata, and release notes.

- npm package: `openclaw@2026.3.13` (`latest`)
- Node requirement: `>=22` (recommended **Node 24**)
- Key releases since last baseline: 2026.3.1, 2026.3.7-beta.1, 2026.3.11, 2026.3.13
- Core sources: see [Validated Baseline](reference/validated-baseline.md)

Quick re-check commands:

```bash
npm view openclaw version dist-tags engines --json
openclaw --version
openclaw doctor
openclaw security audit
openclaw health --json
```

## Required Workflow

For setup, debugging, or architecture changes, run this sequence:

1. Confirm runtime + install health:
   - `openclaw --version`
   - `node --version`
   - `openclaw doctor`
   - `openclaw health --json`
2. Run security baseline before feature work:
   - `openclaw security audit`
   - `openclaw security audit --deep`
3. Inspect active runtime:
   - `openclaw gateway status --deep`
   - `openclaw models status`
   - `openclaw skills list --eligible`
   - `openclaw hooks list --eligible`
   - `openclaw sandbox explain`
4. Make minimal config changes.
5. Re-run audit + status checks after changes.
6. For unclear keys, use `gateway/configuration-reference` first.

## Architecture Mental Model

```
channels -> gateway -> routing -> agent runtime -> tools -> sessions/memory -> outbound delivery
                                       |
                                  ContextEngine (pluggable)
                                       |
                                  ACP agents (external: codex/claude/gemini)
```

- Gateway is control plane; agents are isolated by workspace, state, auth profiles, and sessions.
- Routing is deterministic and binding-order sensitive.
- Tool policy and sandboxing are separate control layers.
- ContextEngine is pluggable — default is `legacy`, override via `plugins.slots.contextEngine`.
- Hooks provide event-driven automation without modifying core code.
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
   - WebSocket origin validation for browser connections
   - use Tailscale Serve (tailnet-only) or Funnel (public with password auth)
4. Constrain execution:
   - enable sandbox (`non-main` or `all`)
   - keep `workspaceAccess` at `none` or `ro` by default
5. Restrict high-risk tools for untrusted surfaces:
   - `exec`, `browser`, `web_search`, `web_fetch`, `gateway`, `cron`
6. Prefer strong current models for tool-enabled agents.
7. ACP sessions run on host, not in sandbox — block ACP spawn from sandboxed sessions.
8. Enable tool loop detection for untrusted agents.

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
- Billing/balance errors (Venice, Poe, etc.) now trigger configured fallbacks.
- Gemini malformed responses treated as retryable timeouts.

### Thinking Levels

- Levels: `off | minimal | low | medium | high | xhigh | adaptive`
- Claude 4.6 defaults to `adaptive` (provider-managed reasoning budget).
- `xhigh` is GPT-5.2 + Codex models only.
- Set via: `/think <level>`, config `agents.defaults.thinkingDefault`, or session override.
- Fast mode: `/fast on|off` — reduces latency on routine queries.
- Verbose: `/verbose on|full|off` — shows tool call details.
- Reasoning: `/reasoning on|off|stream` — shows thinking blocks.

### Ollama (Local Models)

- First-class support via native API (`/api/chat`).
- Auto-discovery: set `OLLAMA_API_KEY` without explicit provider config.
- Cloud + Local mode via `openclaw onboard`.
- Cloud models: `kimi-k2.5:cloud`, `minimax-m2.5:cloud`, `glm-5:cloud`.

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

### Multimodal Memory (New)

- Opt-in image/audio indexing via `memorySearch.extraPaths`.
- Uses `gemini-embedding-2-preview` with configurable output dimensions.
- Automatic reindexing when dimensions change.
- Use case: index screenshots, voice notes, design comps for semantic search.

## ContextEngine (New)

Pluggable context management replacing hardcoded assembly logic.

- Default engine: `legacy` (built-in).
- Override: install a plugin with `kind: "context-engine"`, set `plugins.slots.contextEngine`.
- Lifecycle hooks: `bootstrap`, `ingest`, `assemble`, `compact`, `afterTurn`, `prepareSubagentSpawn`.
- Inspect context: `/context list`, `/context detail`, `/status`.
- Context includes: system prompt, conversation history, tool calls/results, attachments, compaction summaries.
- Bootstrap file injection: `AGENTS.md`, `SOUL.md`, `TOOLS.md`, `IDENTITY.md`, `USER.md`, `HEARTBEAT.md`, `BOOTSTRAP.md`.
- Truncation: `agents.defaults.bootstrapMaxChars` (default 20000), `agents.defaults.bootstrapTotalMaxChars` (default 150000).

## Hooks System (New)

Event-driven automation without modifying core code.

Discovery precedence (highest to lowest):

1. `<workspace>/hooks/`
2. `~/.openclaw/hooks/`
3. bundled hooks
4. `hooks.internal.load.extraDirs`

Bundled hooks:

- `session-memory`: saves session context to `memory/` on `/new`
- `bootstrap-extra-files`: injects additional workspace files during `agent:bootstrap`
- `command-logger`: JSONL audit log to `~/.openclaw/logs/commands.log`
- `boot-md`: runs `BOOT.md` on gateway startup

Event types: `command:new`, `command:reset`, `command:stop`, `agent:bootstrap`, `gateway:startup`, `message:received`, `message:transcribed`, `message:preprocessed`, `message:sent`.

```bash
openclaw hooks list --eligible
openclaw hooks enable session-memory
openclaw hooks info <name>
openclaw hooks check
```

## ACP Agents (New)

External agent protocol for running Codex, Claude Code, Gemini CLI, etc. as child agents.

- Enable: `acp.enabled: true` + install acpx plugin.
- Built-in harnesses: `pi`, `claude`, `codex`, `opencode`, `gemini`, `kimi`.
- Thread binding: spawn ACP sessions bound to Discord/Telegram threads.
- Session resume: `resumeSessionId` to continue previous ACP conversations.
- Permission modes: `approve-all`, `approve-reads`, `deny-all`.
- ACP sessions run on host — cannot spawn from sandboxed sessions.

```bash
openclaw plugins install acpx
/acp doctor
/acp spawn codex --mode persistent --thread auto --cwd /repo
/acp status
/acp sessions
```

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
- ACP sessions cannot be spawned from sandboxed sessions.

### Tool Loop Detection (New)

- Config: `tools.loopDetection` (disabled by default).
- Detectors: `genericRepeat`, `knownPollNoProgress`, `pingPong`.
- Thresholds: `warningThreshold < criticalThreshold < globalCircuitBreakerThreshold`.
- Per-agent override via `agents.list[].tools.loopDetection`.

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
- `lightContext: true` skips heavy file injections for faster automation runs

Cron:

- persistent scheduler in gateway (`cron` section)
- `sessionTarget: "main"` for system-event style runs
- `sessionTarget: "isolated"` for dedicated cron session turns
- delivery modes: `announce`, `webhook`, `none`
- isolated cron delivery prevents leaking into ad hoc agent sends
- `openclaw doctor --fix` migrates legacy cron storage

Commands:

```bash
openclaw system event --text "..." --mode now
openclaw cron add --name "..." --cron "0 7 * * *" --session isolated --message "..."
openclaw cron list
openclaw cron runs --id <jobId>
```

## Plugins System (New)

Extends OpenClaw with installable plugins (e.g., acpx, diffs, community plugins).

```bash
openclaw plugins install <name>
openclaw plugins list
openclaw config set plugins.entries.<name>.enabled true
```

Plugins can provide: agent tools, context engines, ACP backends.

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

## Health Checks (New)

- HTTP endpoints: `/health`, `/healthz`, `/ready`, `/readyz` (auto-registered for Docker/K8s).
- CLI: `openclaw health --json`, `openclaw status --all`, `openclaw status --deep`.
- `openclaw config file` shows active config paths.

## Gateway Networking (New)

- Tailscale: `gateway.tailscale.mode` (`off` | `serve` | `funnel`).
  - `serve`: tailnet-only HTTPS, uses Tailscale identity headers.
  - `funnel`: public HTTPS, requires `gateway.auth.mode: "password"`.
  - `gateway.tailscale.resetOnExit` undoes Serve/Funnel on shutdown.
- `gateway.bind` must stay `loopback` when Serve/Funnel is enabled.
- Gateway config errors now show up to 3 validation issues.

## Common Mistakes

- Treating `tools.elevated` as generic policy instead of exec-only host path.
- Leaving DM policies open while enabling risky tools.
- Misreading `non-main` sandbox mode as "non-main agent" instead of session key behavior.
- Assuming host env injection automatically appears in sandbox containers.
- Forgetting to isolate DM scope for multi-user inboxes.
- Editing cron job store files manually while gateway is running.
- Using stale docs path `concepts/channel-routing` instead of `channels/channel-routing`.
- Spawning ACP sessions from sandboxed sessions (they run on host).
- Setting `permissionMode: "approve-reads"` for ACP without `nonInteractivePermissions: "deny"` (causes crashes on writes).
- Using legacy `hooks.internal.handlers[]` config instead of discovery-based hooks.
- Forgetting to restart gateway after changing hook or plugin config.
- Enabling `gateway.tailscale.mode: "funnel"` without `gateway.auth.mode: "password"`.

## Reference Files

Read these local references for implementation details:

- [Configuration Examples](reference/configuration-examples.md)
- [Validated Baseline](reference/validated-baseline.md)
