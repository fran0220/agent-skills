# cognee-admin CLI

Cognee knowledge engine management CLI and web dashboard.

One binary, two modes:

- `cognee-admin serve` starts the HTMX web management panel.
- All other subcommands call the Cognee REST API or read operational data from PostgreSQL.

## Install

From the monorepo root:

```bash
cd cognee-admin/cli
cargo install --path .
```

For local development:

```bash
cargo run -- --help
```

## Auth Model

`cognee-admin` intentionally separates user-level Cognee access from panel administration.

| Credential | Scope | How to get it | Where it is used |
|------------|-------|---------------|------------------|
| Cognee JWT | Cognee API commands | `cognee-admin login --username ... --password ...` | `health`, `dataset`, `data`, `cognify`, `search`, `config`, `ontology` |
| `ca_xxx` admin token | Web/panel admin surface | Created by an admin via `token create` | `https://cognee.xiaomao.chat`, Nginx `auth_request`, token distribution |

The local config file is `~/.config/cognee-admin/auth.json` and stores `cognee_jwt`, `admin_token`, and `cognee_url`.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `COGNEE_URL` | `https://cogneeapi.xiaomao.chat` | Cognee REST API endpoint |
| `COGNEE_JWT` | none | Cognee API JWT override |
| `COGNEE_ADMIN_TOKEN` | none | Admin token override for panel/admin usage |
| `COGNEE_ADMIN_DB` | `postgres://cognee:cognee@localhost:5433/cognee_db` | PostgreSQL connection string |
| `COGNEE_SERVICE_JWT` | none | Required by `serve` for server-side upstream Cognee API access |

## Quick Start

```bash
# 1. Authenticate and persist the Cognee JWT
cognee-admin login --username alice --password '<password>'

# 2. Create a dataset
cognee-admin dataset create gamedb-atoms

# 3. Upload one markdown file
cognee-admin data add-file --dataset gamedb-atoms ./atoms/zelda-botw.md

# 4. Build the knowledge graph in background mode
cognee-admin cognify --dataset-name gamedb-atoms --background

# 5. Search it
cognee-admin search "open world traversal" --datasets gamedb-atoms --top-k 10
```

## Command Summary

| Command | Description | Example |
|---------|-------------|---------|
| `serve` | Start the web dashboard | `cognee-admin serve --host 0.0.0.0 --port 9847` |
| `health` | Health check, optional watch mode | `cognee-admin health --detailed --watch --interval 15` |
| `login` | Save a Cognee JWT locally | `cognee-admin login --username alice --password '<password>'` |
| `dataset` | Dataset CRUD, graph, and status | `cognee-admin dataset delete-all --yes` |
| `data` | Inline add, file upload, bulk import, list, delete, raw, update | `cognee-admin data add-dir --dataset gamedb-atoms --glob "*.md" ./atoms/` |
| `cognify` | Trigger knowledge construction | `cognee-admin cognify --dataset-name gamedb-atoms --custom-prompt-file ./prompt.txt` |
| `search` | Search with filters or inspect history | `cognee-admin search history` |
| `config` | Get or update Cognee settings | `cognee-admin config set '{"llm":{"provider":"openai","model":"claude-sonnet-4-6","api_key":"<key>"}}'` |
| `log` | Inspect request logs | `cognee-admin log stats` |
| `pipeline` | Inspect recent pipeline runs | `cognee-admin pipeline detail <id>` |
| `token` | Create/list/revoke/delete admin tokens | `cognee-admin token create --name web-admin --role admin` |
| `ontology` | Upload or list ontologies | `cognee-admin ontology upload --key game-design ./ontology.owl` |
| `describe` | Emit machine-friendly command schema | `cognee-admin describe data` |

## Common Workflows

### Batch Import

```bash
cognee-admin data add-dir --dataset gamedb-atoms --glob "*.md" /path/to/atoms/md/
```

`add-dir` uploads recursively, batches files in groups of 10, sleeps 1 second between batches, and returns uploaded/failed/skipped counts.

### Custom Cognify Prompt

```bash
cognee-admin cognify \
  --dataset-name gamedb-atoms \
  --custom-prompt "Extract mechanics, progression systems, and monetization markers." \
  --background
```

### Search Across Datasets

```bash
cognee-admin search "crafting loop" --datasets gamedb-atoms,design-notes --top-k 10 --verbose
```

### Ontology Upload

```bash
cognee-admin ontology upload --key game-design ./ontology/game-design.owl
cognee-admin ontology list
```

## Settings And LLM Configuration

Cognee settings are updated through `config set` or the `/settings` page in the web dashboard.

The settings API accepts:

```json
{
  "llm": {
    "provider": "openai|anthropic|ollama|gemini|mistral",
    "model": "string",
    "api_key": "string"
  }
}
```

Recommended proxy-backed setup:

```bash
cognee-admin config set '{"llm":{"provider":"openai","model":"claude-sonnet-4-6","api_key":"<proxy-key>"}}'
```

Set `LLM_ENDPOINT=https://api.xiaomao.chat/v1` on the Cognee server. The endpoint is not stored in Cognee settings.

## Web Dashboard

Production URL: `https://cognee.xiaomao.chat`

Login uses a `ca_xxx` token.

Current key pages:

| Route | Description |
|-------|-------------|
| `/` | Dashboard with health and activity summary |
| `/graph` | Interactive knowledge graph view |
| `/datasets` | Dataset overview and actions |
| `/search` | Interactive search UI |
| `/logs` | Request logs and pagination |
| `/pipelines` | Pipeline history |
| `/settings` | Runtime settings management |
| `/tokens` | Admin token CRUD |
| `/login` | Token-based login page |

## JSON Output Contract

Default output is a JSON envelope:

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

Use `--human` for human-readable formatting.

Exit codes:

- `0` success
- `1` command/input error
- `2` system error
- `3` external service error

## Related Docs

- Project overview: [`../README.md`](../README.md)
- Design blueprint: [`BLUEPRINT.md`](./BLUEPRINT.md)
- Dev instructions: [`AGENTS.md`](./AGENTS.md)
- Agent skill: [`../skill/SKILL.md`](../skill/SKILL.md)
