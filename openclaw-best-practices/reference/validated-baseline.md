# OpenClaw Skill Validation Baseline

Validated on 2026-02-23.

## Runtime and Package Snapshot

Validation commands:

```bash
npm view openclaw version dist-tags engines --json
curl -s https://api.github.com/repos/openclaw/openclaw | jq '{stargazers_count, forks_count, pushed_at}'
```

Observed output at validation time:

```json
{
  "version": "2026.2.22-2",
  "dist-tags": {
    "latest": "2026.2.22-2",
    "beta": "2026.2.22-2"
  },
  "engines": {
    "node": ">=22.12.0"
  }
}
```

```json
{
  "stargazers_count": 219418,
  "forks_count": 41705,
  "pushed_at": "2026-02-23T07:22:17Z"
}
```

## High-Signal Facts Confirmed

- Correct channel-routing page is `channels/channel-routing` (not `concepts/channel-routing`).
- Secure DM isolation recommendation is `session.dmScope: "per-channel-peer"`; use `per-account-channel-peer` for multi-account channels.
- `tools.elevated` is exec-only host escape behavior, not generic role-based authorization.
- Sandbox policy and tool policy are separate layers; inspect with `openclaw sandbox explain`.
- Skill env overrides (`skills.entries.*.env` / `apiKey`) are host-context; sandboxed runs need `sandbox.docker.env` or custom image env.
- Heartbeat supports `activeHours`, channel/account delivery controls, and `HEARTBEAT_OK` suppression semantics.
- Cron supports `main` and `isolated` execution patterns with `announce`, `webhook`, and `none` delivery modes.
- Environment precedence includes `OPENCLAW_HOME`, `OPENCLAW_STATE_DIR`, and `OPENCLAW_CONFIG_PATH`.

## Source URLs Used

- https://docs.openclaw.ai/cli
- https://docs.openclaw.ai/cli/skills
- https://docs.openclaw.ai/cli/security
- https://docs.openclaw.ai/gateway/configuration
- https://docs.openclaw.ai/gateway/configuration-reference
- https://docs.openclaw.ai/gateway/sandboxing
- https://docs.openclaw.ai/gateway/sandbox-vs-tool-policy-vs-elevated
- https://docs.openclaw.ai/gateway/heartbeat
- https://docs.openclaw.ai/concepts/models
- https://docs.openclaw.ai/concepts/model-failover
- https://docs.openclaw.ai/concepts/memory
- https://docs.openclaw.ai/concepts/multi-agent
- https://docs.openclaw.ai/channels/channel-routing
- https://docs.openclaw.ai/tools/skills
- https://docs.openclaw.ai/tools/skills-config
- https://docs.openclaw.ai/automation/cron-jobs
- https://docs.openclaw.ai/help/environment
- https://docs.openclaw.ai/gateway/security
- https://github.com/openclaw/openclaw

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
```

If docs and behavior diverge, prioritize:

1. `gateway/configuration-reference`
2. command-specific CLI docs (`/cli/*`)
3. latest repository docs and changelog
