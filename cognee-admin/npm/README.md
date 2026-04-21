# @doufunao123/cognee-admin

Lightweight npm CLI client for the Cognee knowledge engine API.

## Install

```bash
npm install -g @doufunao123/cognee-admin
```

Or run directly:

```bash
npx @doufunao123/cognee-admin health
```

## Quick Start

```bash
# 1. Save your admin token (get it from the admin panel)
cognee-admin auth set ca_xxx

# 2. Use any command — token is auto-loaded
cognee-admin dataset list
cognee-admin search "query" --search-type CHUNKS
```

## Authentication

Uses `ca_xxx` admin tokens issued by the cognee-admin panel.

**Resolution order:**

1. `--admin-token <ca_xxx>` CLI flag
2. `COGNEE_ADMIN_TOKEN` environment variable
3. `~/.config/cognee-admin/auth.json` (saved via `auth set`)

Config file format:

```json
{
  "admin_token": "ca_...",
  "cognee_url": "https://cogneeapi.origingame.dev"
}
```

## Commands

```bash
# Auth
cognee-admin auth set <ca_xxx>       # Save token locally
cognee-admin auth status             # Show current auth state
cognee-admin auth clear              # Remove saved credentials

# Health
cognee-admin health                  # Quick health check
cognee-admin health --detailed       # Include component details
cognee-admin health --watch          # Continuous polling

# Datasets
cognee-admin dataset list
cognee-admin dataset create <name>
cognee-admin dataset delete <id>
cognee-admin dataset delete-all --yes
cognee-admin dataset status
cognee-admin dataset graph <id>

# Data
cognee-admin data add --dataset <name> "text content"
cognee-admin data add-file --dataset <name> ./file.md
cognee-admin data add-dir --dataset <name> --glob "*.md" ./dir
cognee-admin data list <datasetId>
cognee-admin data delete --dataset-id <id> --data-id <id>
cognee-admin data raw --dataset-id <id> --data-id <id>
cognee-admin data update --dataset-id <id> --data-id <id> ./new.md

# Cognify
cognee-admin cognify
cognee-admin cognify --dataset-name docs --background

# Search
cognee-admin search "query"
cognee-admin search "query" --search-type CHUNKS --top-k 10
cognee-admin search "query" --datasets docs,notes
cognee-admin search history

# Settings
cognee-admin config get
cognee-admin config set '{"llm":{"provider":"openai","model":"gpt-5.4","api_key":"sk-..."}}'

# Ontology
cognee-admin ontology list
cognee-admin ontology upload --key game ./ontology.owl

# Self-describe (JSON Schema)
cognee-admin describe
cognee-admin describe search
```

## Output

JSON by default. Use `--human` for readable output, `--fields` to filter:

```bash
cognee-admin dataset list --human
cognee-admin config get --fields "llm.provider,llm.model"
```
