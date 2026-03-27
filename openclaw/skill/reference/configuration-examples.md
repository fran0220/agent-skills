# OpenClaw Configuration Examples

Validated against official docs on 2026-02-23.

Notes:

- These examples use JSON5 (`~/.openclaw/openclaw.json`), so comments and trailing commas are allowed.
- Prefer `agents.defaults.*` paths for new configs.
- Re-run `openclaw doctor` and `openclaw security audit` after edits.

## 1) Secure Single-Agent Baseline

```json5
{
  gateway: {
    bind: "loopback",
    port: 18789,
    auth: { mode: "token", token: "${OPENCLAW_GATEWAY_TOKEN}" },
    reload: { mode: "hybrid", debounceMs: 300 },
  },

  session: {
    dmScope: "per-channel-peer",
    reset: { mode: "daily", atHour: 4 },
  },

  agents: {
    defaults: {
      workspace: "~/.openclaw/workspace",
      model: {
        primary: "anthropic/claude-opus-4-6",
        fallbacks: ["openai/gpt-5.2"],
      },
      sandbox: {
        mode: "non-main",
        scope: "agent",
        workspaceAccess: "none",
        docker: { network: "none" },
      },
    },
  },

  channels: {
    whatsapp: {
      dmPolicy: "pairing",
      groups: { "*": { requireMention: true } },
    },
  },

  tools: {
    deny: ["gateway", "cron"],
    elevated: {
      enabled: true,
      allowFrom: { whatsapp: ["+15551230001"] },
    },
  },
}
```

## 2) Multi-Agent Routing with Deterministic Bindings

```json5
{
  agents: {
    list: [
      {
        id: "home",
        default: true,
        workspace: "~/.openclaw/workspace-home",
      },
      {
        id: "work",
        workspace: "~/.openclaw/workspace-work",
        model: "anthropic/claude-opus-4-6",
      },
    ],
  },

  bindings: [
    // Most specific first: peer > account > channel
    {
      agentId: "work",
      match: {
        channel: "whatsapp",
        peer: { kind: "direct", id: "+15551230001" },
      },
    },
    { agentId: "home", match: { channel: "whatsapp", accountId: "personal" } },
    { agentId: "work", match: { channel: "whatsapp", accountId: "biz" } },
    { agentId: "work", match: { channel: "telegram" } },
  ],

  channels: {
    whatsapp: {
      accounts: {
        personal: {},
        biz: {},
      },
    },
  },
}
```

## 3) Untrusted Agent with Strict Sandbox and Tool Policy

```json5
{
  agents: {
    list: [
      {
        id: "public",
        workspace: "~/.openclaw/workspace-public",
        sandbox: {
          mode: "all",
          scope: "agent",
          workspaceAccess: "ro",
          docker: {
            network: "none",
            readOnlyRoot: true,
          },
        },
        tools: {
          allow: ["read", "sessions_list", "sessions_history", "session_status"],
          deny: ["exec", "process", "write", "edit", "apply_patch", "browser", "canvas", "nodes", "cron", "gateway"],
        },
      },
    ],
  },

  tools: {
    elevated: { enabled: false },
  },
}
```

## 4) Models, Auth Rotation, and Memory Search

```json5
{
  auth: {
    profiles: {
      "anthropic:subscription": {
        provider: "anthropic",
        mode: "oauth",
        email: "user@example.com",
      },
      "anthropic:api": { provider: "anthropic", mode: "api_key" },
      "openai:default": { provider: "openai", mode: "api_key" },
    },
    order: {
      anthropic: ["anthropic:subscription", "anthropic:api"],
      openai: ["openai:default"],
    },
  },

  agents: {
    defaults: {
      model: {
        primary: "anthropic/claude-sonnet-4-5",
        fallbacks: ["openai/gpt-5.2"],
      },
      models: {
        "anthropic/claude-sonnet-4-5": { alias: "sonnet" },
        "openai/gpt-5.2": { alias: "gpt" },
      },
      memorySearch: {
        provider: "openai",
        model: "text-embedding-3-small",
        fallback: "openai",
        cache: { enabled: true, maxEntries: 50000 },
        sync: { watch: true },
      },
      compaction: {
        reserveTokensFloor: 24000,
        memoryFlush: {
          enabled: true,
          softThresholdTokens: 6000,
        },
      },
    },
  },

  memory: {
    backend: "qmd",
    citations: "auto",
  },
}
```

## 5) Heartbeat and Cron

```json5
{
  agents: {
    defaults: {
      heartbeat: {
        every: "30m",
        target: "last",
        activeHours: {
          start: "09:00",
          end: "22:00",
          timezone: "America/New_York",
        },
        prompt: "Read HEARTBEAT.md if it exists (workspace context). Follow it strictly. If nothing needs attention, reply HEARTBEAT_OK.",
      },
    },
    list: [
      {
        id: "ops",
        workspace: "~/.openclaw/workspace-ops",
        heartbeat: {
          every: "1h",
          target: "telegram",
          to: "-1001234567890:topic:42",
          accountId: "ops-bot",
        },
      },
    ],
  },

  channels: {
    telegram: {
      accounts: {
        "ops-bot": { botToken: "${TELEGRAM_BOT_TOKEN_OPS}" },
      },
      heartbeat: { showOk: false, showAlerts: true, useIndicator: true },
    },
  },

  cron: {
    enabled: true,
    maxConcurrentRuns: 1,
    store: "~/.openclaw/cron/jobs.json",
  },
}
```

Common cron flows:

```bash
openclaw cron add --name "Morning brief" --cron "0 7 * * *" --tz "America/New_York" --session isolated --message "Summarize overnight updates."
openclaw cron list
openclaw cron runs --id <jobId>
openclaw system event --text "Check urgent follow-ups" --mode now
```

## 6) Skills Loading and Per-Skill Overrides

```json5
{
  skills: {
    allowBundled: ["gemini", "peekaboo"],
    load: {
      extraDirs: ["~/Projects/agent-skills"],
      watch: true,
      watchDebounceMs: 250,
    },
    install: {
      preferBrew: true,
      nodeManager: "npm",
    },
    entries: {
      "nano-banana-pro": {
        enabled: true,
        apiKey: "${GEMINI_API_KEY}",
        env: { GEMINI_API_KEY: "${GEMINI_API_KEY}" },
        config: {
          endpoint: "https://example.invalid",
          model: "nano-pro",
        },
      },
      sag: { enabled: false },
    },
  },
}
```

Important: `skills.entries.*.env` and `apiKey` are host-context injections.
For sandboxed sessions, add secrets to `agents.defaults.sandbox.docker.env` or bake them into the sandbox image.

## 7) Custom OpenAI-Compatible Providers

```json5
{
  models: {
    mode: "merge",
    providers: {
      moonshot: {
        baseUrl: "https://api.moonshot.ai/v1",
        apiKey: "${MOONSHOT_API_KEY}",
        api: "openai-completions",
        models: [{ id: "kimi-k2.5", name: "Kimi K2.5" }],
      },
      localproxy: {
        baseUrl: "http://127.0.0.1:4000/v1",
        apiKey: "${CUSTOM_API_KEY}",
        api: "openai-responses",
        models: [
          {
            id: "my-model",
            name: "My Model",
            contextWindow: 128000,
            maxTokens: 8192,
          },
        ],
      },
    },
  },
  agents: {
    defaults: {
      model: {
        primary: "moonshot/kimi-k2.5",
        fallbacks: ["localproxy/my-model"],
      },
    },
  },
}
```

## 8) Validation Commands

```bash
openclaw doctor
openclaw security audit --deep
openclaw gateway status --deep
openclaw models status --probe
openclaw sandbox explain
openclaw skills check
```

## 9) ContextEngine Plugin

```json5
{
  plugins: {
    entries: {
      "my-context-engine": {
        enabled: true,
      },
    },
    slots: {
      contextEngine: "my-context-engine",
    },
  },
}
```

The default engine is `legacy`. When a plugin provides `kind: "context-engine"`, set `plugins.slots.contextEngine` to delegate context assembly, `/compact`, and subagent context lifecycle hooks.

## 10) Hooks System

```json5
{
  hooks: {
    internal: {
      enabled: true,
      entries: {
        "session-memory": { enabled: true },
        "bootstrap-extra-files": {
          enabled: true,
          paths: ["packages/*/AGENTS.md", "packages/*/TOOLS.md"],
        },
        "command-logger": { enabled: true },
        "boot-md": { enabled: true },
      },
      load: {
        extraDirs: ["/path/to/more/hooks"],
      },
    },
  },
}
```

Bundled hooks: `session-memory` (saves context on `/new`), `bootstrap-extra-files` (inject monorepo files), `command-logger` (JSONL audit), `boot-md` (run BOOT.md on startup).

```bash
openclaw hooks list --eligible
openclaw hooks enable session-memory
openclaw hooks info session-memory
openclaw hooks check
```

## 11) ACP Agents (External Agent Protocol)

```json5
{
  acp: {
    enabled: true,
    dispatch: { enabled: true },
    defaultAgent: "codex",
    allowedAgents: ["codex", "claude", "pi", "gemini"],
  },

  plugins: {
    entries: {
      acpx: {
        enabled: true,
        config: {
          permissionMode: "approve-all",
          nonInteractivePermissions: "fail",
        },
      },
    },
  },

  channels: {
    discord: {
      threadBindings: { spawnAcpSessions: true },
    },
    telegram: {
      threadBindings: { spawnAcpSessions: true },
    },
  },
}
```

Setup:

```bash
openclaw plugins install acpx
openclaw config set plugins.entries.acpx.enabled true
/acp doctor
/acp spawn codex --mode persistent --thread auto --cwd /repo
/acp status
/acp sessions
```

## 12) Ollama (Local + Cloud)

```json5
{
  // Simplest: just set env OLLAMA_API_KEY="ollama-local"
  // For explicit config:
  models: {
    providers: {
      ollama: {
        baseUrl: "http://127.0.0.1:11434",
        apiKey: "ollama-local",
        api: "ollama",
        models: [
          {
            id: "glm-4.7-flash",
            name: "GLM-4.7 Flash",
            contextWindow: 32768,
          },
        ],
      },
    },
  },

  agents: {
    defaults: {
      model: {
        primary: "ollama/glm-4.7-flash",
        fallbacks: ["anthropic/claude-sonnet-4-5"],
      },
    },
  },
}
```

Onboarding: `openclaw onboard` → select Ollama → choose Local or Cloud + Local.
Auto-discovery: set `OLLAMA_API_KEY` without explicit `models.providers.ollama`.

## 13) Thinking Levels and Fast Mode

```json5
{
  agents: {
    defaults: {
      thinkingDefault: "adaptive",
      models: {
        "anthropic/claude-opus-4-6": { alias: "opus" },
        "openai/gpt-5.2": {
          alias: "gpt",
          params: { fastMode: false },
        },
      },
    },
  },
}
```

Slash commands: `/think high`, `/think adaptive`, `/fast on`, `/verbose full`, `/reasoning on`.
Claude 4.6 models default to `adaptive` thinking. Levels: `off | minimal | low | medium | high | xhigh | adaptive`.

## 14) Tool Loop Detection

```json5
{
  tools: {
    loopDetection: {
      enabled: true,
      historySize: 30,
      warningThreshold: 10,
      criticalThreshold: 20,
      globalCircuitBreakerThreshold: 30,
      detectors: {
        genericRepeat: true,
        knownPollNoProgress: true,
        pingPong: true,
      },
    },
  },
}
```

Disabled by default. Enable per-agent with `agents.list[].tools.loopDetection`.

## 15) Health Checks and Tailscale

```json5
{
  gateway: {
    bind: "loopback",
    // Health endpoints: /health, /healthz, /ready, /readyz (auto-registered)

    tailscale: {
      mode: "serve",          // off | serve | funnel
      resetOnExit: true,
    },

    auth: {
      mode: "password",       // required for funnel mode
      password: "${OPENCLAW_GATEWAY_PASSWORD}",
    },
  },
}
```

```bash
openclaw health --json
openclaw health --json --timeout 5000
openclaw status --all
openclaw status --deep
```

## 16) Multimodal Memory Search

```json5
{
  agents: {
    defaults: {
      memorySearch: {
        provider: "gemini",
        model: "gemini-embedding-2-preview",
        extraPaths: ["~/screenshots", "~/voice-notes"],
        fallback: "openai",
        cache: { enabled: true, maxEntries: 50000 },
      },
    },
  },
}
```

Opt-in image/audio indexing. Automatic reindexing when dimensions change.

## 17) Heartbeat Light Context

```json5
{
  agents: {
    defaults: {
      heartbeat: {
        every: "30m",
        lightContext: true,     // skip heavy file injections
        target: "last",
        includeReasoning: true,
      },
    },
  },

  cron: {
    enabled: true,
    maxConcurrentRuns: 1,
  },
}
```

`lightContext` retains only `HEARTBEAT.md` essentials, trimming startup overhead.

## 18) Validation Commands (Updated)

```bash
openclaw doctor
openclaw doctor --fix
openclaw security audit --deep
openclaw gateway status --deep
openclaw models status --probe
openclaw sandbox explain
openclaw skills check
openclaw hooks list --eligible
openclaw hooks check
openclaw health --json
openclaw config file
/acp doctor
/context list
/context detail
/status
```
