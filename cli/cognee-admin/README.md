# cognee-admin

Cognee knowledge engine management CLI and web dashboard.

One binary, two modes:
- `cognee-admin serve` runs a web management panel
- Other subcommands act as CLI clients for the Cognee REST API

## Features

- **Web dashboard** at `cognee.xiaomao.chat` — HTMX + Tailwind, server-rendered
- **CLI client** — JSON envelope output for agent automation
- **Dual data source** — Cognee REST API for writes, PostgreSQL direct reads for analytics
- **Request logging** — all API calls logged to `cognee_admin.request_logs`
- **Health monitoring** — periodic snapshots stored for trend analysis

## Quick Start

### 1) Install

From this monorepo root:

```bash
cargo install --path cli/cognee-admin
```

For local development:

```bash
cargo run --manifest-path cli/cognee-admin/Cargo.toml -- --help
```

### 2) Configure

```bash
export COGNEE_URL=https://api.cognee.xiaomao.chat
export COGNEE_ADMIN_DB=postgres://cognee:cognee@localhost:5433/cognee_db
```

### 3) Login

```bash
cognee-admin login --url https://api.cognee.xiaomao.chat --token <your-token>
```

### 4) Check health

```bash
cognee-admin health
```

### 5) Start web panel

```bash
cognee-admin serve --host 0.0.0.0 --port 3000
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `COGNEE_URL` | `https://api.cognee.xiaomao.chat` | Cognee REST API endpoint |
| `COGNEE_ADMIN_DB` | `postgres://cognee:cognee@localhost:5433/cognee_db` | PostgreSQL connection string |
| `COGNEE_ADMIN_HOST` | `0.0.0.0` | Web panel bind address |
| `COGNEE_ADMIN_PORT` | `3000` | Web panel bind port |

## Command Reference

### `serve`

Run the web management panel:

```bash
cognee-admin serve [--host 0.0.0.0] [--port 3000]
```

### `health`

Check Cognee stack health:

```bash
cognee-admin health [--watch]
```

### `dataset`

Manage knowledge datasets:

```bash
cognee-admin dataset list [--limit 20]
cognee-admin dataset get <dataset-id>
cognee-admin dataset delete <dataset-id> --confirm
cognee-admin dataset status <dataset-id>
```

### `data`

Manage documents within datasets:

```bash
cognee-admin data add <dataset-id> --file <path> [--type text|pdf]
cognee-admin data list <dataset-id>
cognee-admin data delete <dataset-id> <document-id> --confirm
```

### `cognify`

Trigger knowledge graph construction:

```bash
cognee-admin cognify [dataset-id]
```

### `search`

Query the knowledge graph:

```bash
cognee-admin search --query "knowledge graph" [--type graph|insights|chunks]
```

### `config`

View and modify Cognee engine configuration:

```bash
cognee-admin config show
cognee-admin config set <key> <value>
cognee-admin config reset --confirm
```

### `log`

Query request logs from `cognee_admin.request_logs`:

```bash
cognee-admin log list [--limit 50] [--endpoint /api/v1/cognify]
```

### `pipeline`

Inspect and manage pipelines:

```bash
cognee-admin pipeline list
cognee-admin pipeline status <pipeline-id>
cognee-admin pipeline reset --confirm
```

### `describe`

Schema introspection for agent automation:

```bash
cognee-admin describe
cognee-admin describe search
cognee-admin describe dataset.list
```

### `login`

Save Cognee API authentication:

```bash
cognee-admin login --url <cognee-api-url> --token <api-token>
```

## Web Panel

The `serve` command runs an HTMX web dashboard with the following pages:

| Page | Route | Description |
|------|-------|-------------|
| Dashboard | `/` | Health overview, recent requests, dataset stats |
| Datasets | `/datasets` | Dataset list, search, delete |
| Dataset Detail | `/datasets/:id` | Documents, status, cognify trigger |
| Search | `/search` | Interactive knowledge graph search |
| Logs | `/logs` | Request log table with filtering |
| Health | `/health` | Component status and trend charts |
| Config | `/config` | Engine configuration viewer |

<!-- TODO: Add screenshots -->

## JSON Contract

All CLI commands output a JSON envelope:

```json
{
  "ok": true,
  "command": "dataset.list",
  "data": {}
}
```

Error shape:

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

Use `--human` for human-readable output format.

Exit codes:

- `0` success
- `1` command/input error
- `2` system error
- `3` external service error (Cognee API)

## Architecture

```
                    ┌──────────────────────────────────────┐
                    │         cognee-admin binary           │
                    │                                      │
                    │  ┌────────────┐  ┌────────────────┐  │
                    │  │  CLI cmds  │  │  Web panel     │  │
                    │  │  (clap)    │  │  (Axum+HTMX)   │  │
                    │  └─────┬──────┘  └───────┬────────┘  │
                    │        │                 │            │
                    │  ┌─────┴─────────────────┴────────┐  │
                    │  │       CogneeClient              │  │
                    │  │  (reqwest + request logging)    │  │
                    │  └─────┬──────────────────┬───────┘  │
                    └────────┼──────────────────┼──────────┘
                             │                  │
              REST API       │                  │  PG direct
              (writes)       │                  │  (reads)
                             ▼                  ▼
                    ┌────────────────┐  ┌──────────────┐
                    │  Cognee API    │  │  PostgreSQL   │
                    │  (Docker)      │  │              │
                    │  api.cognee.   │  │  public.*    │
                    │  xiaomao.chat  │  │  cognee_admin│
                    └────────────────┘  │  .*          │
                                       └──────────────┘

    Oracle VPS: 4C ARM / 22G RAM
    Cognee stack runs in Docker containers
```

## Deployment

### Cross-compile for ARM (Oracle VPS)

```bash
# Install target
rustup target add aarch64-unknown-linux-gnu

# Build release
cargo build --release --target aarch64-unknown-linux-gnu \
  --manifest-path cli/cognee-admin/Cargo.toml

# Deploy
scp target/aarch64-unknown-linux-gnu/release/cognee-admin oracle-vps:/usr/local/bin/
```

### systemd Service

```ini
[Unit]
Description=Cognee Admin Web Panel
After=network.target postgresql.service docker.service

[Service]
Type=simple
User=cognee
ExecStart=/usr/local/bin/cognee-admin serve --host 0.0.0.0 --port 3000
Environment=COGNEE_URL=https://api.cognee.xiaomao.chat
Environment=COGNEE_ADMIN_DB=postgres://cognee:cognee@localhost:5433/cognee_db
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo cp cognee-admin.service /etc/systemd/system/
sudo systemctl enable --now cognee-admin
```

### Reverse Proxy (Caddy)

```
cognee.xiaomao.chat {
    reverse_proxy localhost:3000
}
```

## Database Schema

cognee-admin uses a dedicated `cognee_admin` schema in the same PostgreSQL instance as Cognee:

- `cognee_admin.request_logs` — API request audit trail
- `cognee_admin.health_snapshots` — periodic health check records

Migrations run automatically on `serve` startup.

## License

MIT
