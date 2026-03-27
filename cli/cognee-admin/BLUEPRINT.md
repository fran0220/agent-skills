# Design Blueprint — `cognee-admin`

> Cognee knowledge engine management CLI and web dashboard. One binary, two modes: web panel at `cognee.xiaomao.chat` and CLI client for `api.cognee.xiaomao.chat`.

## 1. Purpose

Provide a unified management surface for the Cognee knowledge engine, combining an HTMX web dashboard with a CLI client.

`cognee-admin` enables:
- Health monitoring of the Cognee stack
- Dataset and document lifecycle management
- Knowledge graph construction (cognify) and search
- Pipeline inspection and control
- Request logging and historical analysis via web panel

Primary goals:
- one binary for both web panel and CLI operations
- dual data source: Cognee REST API for writes, PostgreSQL direct reads for analytics
- operational visibility into an otherwise API-only knowledge engine
- agent-friendly JSON output for automation

## 2. Classification

- **Primary role:** Service + CLI Client
- **Primary user type:** Human Admin + Agent-Secondary
- **Primary interaction form:** Web Dashboard + Batch CLI
- **Statefulness:** Config-Stateful (auth token persisted, no sessions)
- **Risk profile:** Mixed (data deletion is high-risk, reads are low-risk)
- **Confidence level:** High

### 2b. Classification reasoning

**Why Service + CLI Client.**
`cognee-admin serve` runs a long-lived HTMX web panel. Other subcommands are thin HTTP clients that talk directly to the Cognee API. This is the same single-binary dual-mode pattern as `asset-gateway`.

**Why Human Admin + Agent-Secondary.**
The web panel is human-centric (HTMX, Tailwind). CLI subcommands produce JSON envelopes for agent automation, but the primary audience is operators monitoring and managing Cognee.

**Why Config-Stateful.**
`cognee-admin login` persists a Cognee API token to `~/.config/cognee-admin/auth.json`. No server-side sessions — the web panel reads directly from PG and proxies writes through the Cognee API.

**Why Mixed risk.**
Read operations (health, dataset list, search) are low-risk. Data mutations (dataset delete, cognify trigger) and pipeline resets are medium-to-high risk. No credential vault needed — we delegate auth to Cognee's own API.

## 3. Primary design stance

Use a **single Rust binary with dual mode**:

1. `cognee-admin serve` runs the HTMX web panel, serving SSR templates with PG-backed analytics.
2. All other subcommands act as HTTP clients against the Cognee REST API at `api.cognee.xiaomao.chat`.

This keeps deployment simple:
- one executable on the Oracle ARM VPS
- web panel and CLI share the same Cognee client code
- PG connection provides read-side analytics without additional services

## 4. Command structure

```
cognee-admin serve       --host --port                  # web panel
cognee-admin health      [--watch]                      # Cognee stack health
cognee-admin dataset     list|get|delete|status         # dataset CRUD
cognee-admin data        add|list|delete                # document management
cognee-admin cognify     [dataset-id]                   # trigger knowledge graph build
cognee-admin search      --query <text> [--type graph|insights|chunks]
cognee-admin config      show|set|reset                 # Cognee engine config
cognee-admin log         list [--limit] [--endpoint]    # request log query
cognee-admin pipeline    list|status|reset              # pipeline inspection
cognee-admin describe    [command]                      # schema introspection
cognee-admin login       --url <cognee-api> --token <t> # save auth config
```

Command intent:
- `serve`: run web management panel (HTMX + Tailwind)
- `health`: check Cognee API, database, and component status
- `dataset`: inspect and manage knowledge datasets
- `data`: add documents to datasets, list contents, remove documents
- `cognify`: trigger knowledge graph construction on a dataset
- `search`: query the knowledge graph (graph traversal, insights, chunks)
- `config`: view and modify Cognee engine configuration
- `log`: query request logs from `cognee_admin.request_logs`
- `pipeline`: inspect pipeline tasks and reset stuck pipelines
- `describe`: machine-friendly command/schema introspection
- `login`: persist Cognee API token for CLI usage

## 5. Input model

### Dual-track input

**CLI flags** for simple operations:
```bash
cognee-admin search --query "knowledge graph" --type graph
cognee-admin dataset list --limit 20
cognee-admin health --watch
```

**Global flags:**
- `--cognee-url` or `COGNEE_URL` — Cognee API endpoint
- `--human` — human-readable output instead of JSON
- `--verbose` — include request/response details in output

### Web panel input

HTMX form submissions to server-side handlers that proxy to Cognee API and log to PG.

## 6. Output model

Default output is JSON envelope:

```json
{
  "ok": true,
  "command": "dataset.list",
  "data": {}
}
```

Error output:

```json
{
  "ok": false,
  "command": "cognify",
  "error": {
    "code": "COGNEE_API_ERROR",
    "message": "Cognee returned 503",
    "suggestion": "Check if Cognee is running: cognee-admin health"
  }
}
```

`--human` flag switches to a compact, human-readable table/list format on stderr.

Exit code policy:
- `0` success
- `1` command/input error
- `2` system error (DB/config/internal)
- `3` external service error (Cognee API unreachable)

## 7. Discoverability / introspection

`describe` is the primary discovery surface for agents:

```bash
cognee-admin describe              # list all commands
cognee-admin describe search       # search command schema
cognee-admin describe dataset.list # specific subcommand schema
```

`--help` is available for human diagnostics but schema-oriented introspection via `describe` is preferred for automation.

## 8. State / session model

Config-Stateful with no server sessions:

- `~/.config/cognee-admin/auth.json` — persisted Cognee API token and endpoint
- No server-side session state — the web panel is stateless SSR
- PG `cognee_admin` schema stores operational data (request logs, health snapshots), not user sessions

Configuration hierarchy:
1. CLI flags (`--cognee-url`)
2. Environment variables (`COGNEE_URL`, `COGNEE_ADMIN_DB`)
3. Config file (`~/.config/cognee-admin/auth.json`)
4. Defaults (localhost)

## 9. Risk / safety model

### Low risk (read-only)
- `health`, `dataset list`, `dataset get`, `data list`, `log list`, `pipeline list`, `config show`, `describe`, `search`

### Medium risk (state mutation via API)
- `data add`, `cognify`, `config set` — trigger Cognee processing or change configuration

### High risk (destructive)
- `dataset delete`, `data delete`, `pipeline reset`, `config reset` — destructive operations

Safety controls:
- High-risk CLI commands require `--confirm` flag or interactive confirmation
- Web panel uses confirmation modals for destructive actions
- All mutations logged to `cognee_admin.request_logs` with request body and response preview
- No direct write access to Cognee's database — all mutations go through Cognee REST API

## 10. Hardening model

- **Request logging**: every Cognee API call logged to `cognee_admin.request_logs` with latency, status, and preview
- **Health snapshots**: periodic health checks stored in `cognee_admin.health_snapshots` for trend analysis
- **Timeout policy**: 30s default for Cognee API calls, configurable per-command
- **Graceful degradation**: web panel shows last-known state if Cognee API is temporarily unreachable
- **No credential storage**: Cognee API tokens are user-managed, not encrypted vaulted (Cognee handles its own auth)

## 11. Secondary surface contract

### Web panel (HTMX + Tailwind)

The `serve` command runs an SSR web application:

| Page | Route | Data Source |
|------|-------|-------------|
| Dashboard | `/` | Cognee API health + PG logs |
| Datasets | `/datasets` | Cognee API list |
| Dataset detail | `/datasets/:id` | Cognee API + PG logs |
| Documents | `/datasets/:id/documents` | Cognee API |
| Search | `/search` | Cognee API search |
| Pipelines | `/pipelines` | Cognee API |
| Config | `/config` | Cognee API |
| Logs | `/logs` | PG `cognee_admin.request_logs` |
| Health | `/health` | PG `cognee_admin.health_snapshots` |

### JSON API routes

```
GET  /api/health          → health check
GET  /api/datasets        → dataset list
GET  /api/datasets/:id    → dataset detail
POST /api/cognify         → trigger cognify
POST /api/search          → search query
GET  /api/logs            → request logs (from PG)
```

Web panel uses HTMX `hx-get`/`hx-post` against HTML fragment endpoints; JSON API routes serve agent/CLI clients.

## 12. v1 boundaries

v1 includes:
- Dual-mode single binary (`serve` + CLI client commands)
- Core commands: `health`, `dataset`, `data`, `cognify`, `search`, `config`, `login`
- Web panel: dashboard, datasets, search, logs, health pages
- PG schema: `cognee_admin.request_logs` + `cognee_admin.health_snapshots`
- Request logging for all Cognee API calls
- JSON envelope output for all CLI commands
- `describe` command for introspection

v1 excludes:
- User management / multi-tenant auth
- Background job scheduling (cron-based cognify)
- WebSocket real-time updates on web panel
- Graph visualization (knowledge graph rendering)
- File upload via web panel (CLI-only for v1)
- Pipeline execution engine (Cognee handles this)

## 13. Direction for implementation

### Phase 1: Foundation (current)
Core infrastructure: `AppConfig`, `PgPool`, `AppError`, JSON output, `CogneeClient`, migration.

### Phase 2: CLI client commands
Implement `health`, `dataset`, `data`, `cognify`, `search`, `config`, `login` as Cognee API clients. Each command logs to PG via `CogneeClient`.

### Phase 3: Web panel
HTMX + Askama templates for dashboard, datasets, search, logs, health. Static assets via `rust-embed`.

### Phase 4: Polish
`describe` introspection, `--human` output mode, health monitoring with snapshots, `pipeline` commands, `--watch` mode for health.

## Technology stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust (edition 2021) | Shared pattern with asset-gateway, single-binary deployment |
| HTTP server | Axum 0.8 + tower-http | Typed routing, middleware, static files |
| CLI framework | clap 4 (derive) | Strong subcommand + env var support |
| Database | PostgreSQL + sqlx 0.8 | Shared PG with Cognee stack, schema isolation |
| HTTP client | reqwest 0.12 | Cognee API integration |
| Templates | Askama 0.12 + askama_axum | Type-safe SSR templates |
| Static assets | rust-embed 8 | Embed CSS/JS in binary |
| Frontend | HTMX + Tailwind CSS | Lightweight, no JS build step |
| Serialization | serde + serde_json | JSON envelope contract |
| Runtime | tokio | Async execution for server + client |
