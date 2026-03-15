# OpenClaw Skill Validation Baseline

Validated on 2026-03-15.

## Runtime and Package Snapshot

Validation commands:

```bash
npm view openclaw version dist-tags engines --json
curl -s https://api.github.com/repos/openclaw/openclaw | jq '{stargazers_count, forks_count, pushed_at}'
```

Observed output at validation time:

```json
{
  "version": "2026.3.13",
  "dist-tags": {
    "latest": "2026.3.13"
  },
  "engines": {
    "node": ">=22"
  }
}
```

Node recommendation: **Node 24** preferred, Node 22 LTS (`22.16+`) for compatibility.

## Key Releases Since Last Baseline

- **2026.3.1** (Mar 2): Adaptive thinking for Claude 4.6, health endpoints, light heartbeat, Android nodes, Feishu overhaul
- **2026.3.7-beta.1** (Mar 7): ContextEngine plugin interface, dual-engine model routing, 200+ bug fixes
- **2026.3.11** (Mar 11): First-class Ollama, OpenCode Zen+Go, multimodal memory indexing, ACP resume, cron isolation tightening
- **2026.3.13** (Mar 14): Latest stable

## High-Signal Facts Confirmed

### Unchanged from prior baseline
- Correct channel-routing page is `channels/channel-routing` (not `concepts/channel-routing`).
- Secure DM isolation recommendation is `session.dmScope: "per-channel-peer"`; use `per-account-channel-peer` for multi-account channels.
- `tools.elevated` is exec-only host escape behavior, not generic role-based authorization.
- Sandbox policy and tool policy are separate layers; inspect with `openclaw sandbox explain`.
- Skill env overrides (`skills.entries.*.env` / `apiKey`) are host-context; sandboxed runs need `sandbox.docker.env` or custom image env.
- Environment precedence includes `OPENCLAW_HOME`, `OPENCLAW_STATE_DIR`, and `OPENCLAW_CONFIG_PATH`.

### New in 2026.3.x
- **ContextEngine**: Pluggable context management via `plugins.slots.contextEngine`. Lifecycle hooks: `bootstrap`, `ingest`, `assemble`, `compact`, `afterTurn`, `prepareSubagentSpawn`. Default engine is `legacy`.
- **Hooks system**: Event-driven automation replacing legacy `hooks.internal.handlers[]`. Discovery-based from `<workspace>/hooks/`, `~/.openclaw/hooks/`, bundled hooks. Bundled: `session-memory`, `bootstrap-extra-files`, `command-logger`, `boot-md`.
- **ACP Agents**: External agent protocol via acpx plugin. Harnesses: `pi`, `claude`, `codex`, `opencode`, `gemini`, `kimi`. Thread binding, `resumeSessionId`, permission modes (`approve-all`, `approve-reads`, `deny-all`). ACP sessions run on host, not in sandbox.
- **Thinking levels**: `off | minimal | low | medium | high | xhigh | adaptive`. Claude 4.6 defaults to `adaptive`. Fast mode (`/fast`), verbose (`/verbose`), reasoning visibility (`/reasoning`).
- **Ollama first-class**: Auto-discovery via `OLLAMA_API_KEY` + `/api/tags`. Cloud + Local mode. Onboarding wizard support.
- **Tool loop detection**: `tools.loopDetection` with `genericRepeat`, `knownPollNoProgress`, `pingPong` detectors. Disabled by default.
- **Health check endpoints**: `/health`, `/healthz`, `/ready`, `/readyz` for Docker/K8s. CLI: `openclaw health --json`, `openclaw status --deep`.
- **Multimodal memory**: Opt-in image/audio indexing via `memorySearch.extraPaths` with `gemini-embedding-2-preview`.
- **Heartbeat lightContext**: `agents.*.heartbeat.lightContext` skips heavy file injections for automation runs.
- **Tailscale integration**: `gateway.tailscale.mode` (`off` | `serve` | `funnel`) with auto-configured Serve/Funnel.
- **Plugins system**: `openclaw plugins install/list`, manifest-based, community plugins, agent-tools plugins.
- **WebSocket security**: Origin validation tightened for browser connections; trusted-proxy auth.
- **Discord enhancements**: `autoArchiveDuration`, inactivity-based thread lifecycles (`idleHours`, `maxAgeHours`), `/session idle` and `/session max-age` commands.
- **Telegram enhancements**: Topic-level agent isolation, per-DM configs (allowlists, dmPolicy, skills, systemPrompt), persistent channel bindings.
- **Feishu**: Docx tables, image/file uploads, reactions, group session scopes (`group_sender`, `group_topic`), outbound dm:/group: routing.
- **Cron tightening**: Isolated cron delivery, `openclaw doctor --fix` migration for legacy cron storage.
- **Config error clarity**: Gateway config errors show up to 3 validation issues.
- **Child command detection**: `OPENCLAW_CLI` env flag for subprocesses.

## Source URLs Used

### Core (unchanged)
- https://docs.openclaw.ai/cli
- https://docs.openclaw.ai/gateway/configuration
- https://docs.openclaw.ai/gateway/configuration-reference
- https://docs.openclaw.ai/concepts/model-failover
- https://docs.openclaw.ai/concepts/memory
- https://docs.openclaw.ai/concepts/multi-agent
- https://docs.openclaw.ai/channels/channel-routing
- https://docs.openclaw.ai/gateway/sandboxing
- https://docs.openclaw.ai/gateway/sandbox-vs-tool-policy-vs-elevated
- https://docs.openclaw.ai/tools/skills
- https://docs.openclaw.ai/tools/skills-config
- https://docs.openclaw.ai/gateway/heartbeat
- https://docs.openclaw.ai/automation/cron-jobs
- https://docs.openclaw.ai/help/environment
- https://docs.openclaw.ai/gateway/security
- https://github.com/openclaw/openclaw

### New sources (2026.3.x)
- https://docs.openclaw.ai/concepts/context
- https://docs.openclaw.ai/automation/hooks
- https://docs.openclaw.ai/tools/acp-agents
- https://docs.openclaw.ai/providers/ollama
- https://docs.openclaw.ai/tools/thinking
- https://docs.openclaw.ai/tools/loop-detection
- https://docs.openclaw.ai/gateway/health
- https://docs.openclaw.ai/gateway/tailscale
- https://docs.openclaw.ai/tools/plugin
- https://docs.openclaw.ai/plugins/manifest
- https://docs.openclaw.ai/gateway/trusted-proxy-auth
- https://docs.openclaw.ai/security/THREAT-MODEL-ATLAS
- https://docs.openclaw.ai/tools/subagents
- https://docs.openclaw.ai/channels/feishu
- https://docs.openclaw.ai/channels/discord
- https://docs.openclaw.ai/channels/telegram

## Re-Validation Checklist

Run these whenever this skill is updated:

```bash
openclaw --version
node --version
openclaw doctor
openclaw security audit --deep
openclaw gateway status --deep
openclaw models status --probe
openclaw skills list --eligible
openclaw sandbox explain
openclaw hooks list --eligible
openclaw health --json
```

If docs and behavior diverge, prioritize:

1. `gateway/configuration-reference`
2. command-specific CLI docs (`/cli/*`)
3. latest repository docs and changelog
