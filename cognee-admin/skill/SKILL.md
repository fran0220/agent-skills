---
name: cognee-admin
description: "Cognee knowledge engine management. Use when the user wants to add data to Cognee, build knowledge graphs (cognify), search knowledge bases, manage datasets, or administer the Cognee instance."
---

# Cognee Admin

Use `cognee-admin` to ingest documents into Cognee, trigger knowledge construction, search datasets, inspect operations, and manage the web dashboard.

## Prerequisites

### npm CLI（推荐，轻量客户端）

```bash
npx @doufunao123/cognee-admin@0.4.0 <command>
# 或全局安装
npm install -g @doufunao123/cognee-admin
```

认证 token 保存后免传参：

```bash
cognee-admin auth set ca_xxx_your_token  # 只需一次
```

详细命令参考：`reference/npm-cli.md`

### Rust CLI（完整版，含 Web 面板）

```bash
cd /path/to/agent-skills/cognee-admin/cli
cargo install --path .
```

环境变量：

| Variable | When to use it |
|----------|----------------|
| `COGNEE_URL` | Cognee API base URL. Default is `https://cogneeapi.origingame.dev`. |
| `COGNEE_JWT` | Required for Cognee API commands such as `health`, `dataset`, `data`, `cognify`, `search`, `config`, and `ontology`. |
| `COGNEE_ADMIN_TOKEN` | `ca_xxx` token for the web dashboard and Nginx-protected admin surface. |
| `COGNEE_ADMIN_DB` | PostgreSQL connection string for log, pipeline, token, and panel-backed features. |

For `serve`, also provide `COGNEE_SERVICE_JWT` so the dashboard can call Cognee upstream.

### 两套 CLI 的区别

npm CLI 只做 API 客户端。`serve`、`log`、`pipeline`、`token`、`login` 仅 Rust CLI 支持。

## Authentication

`cognee-admin` uses a dual-auth model:

1. Cognee API auth uses a Cognee JWT. Obtain it with:

```bash
cognee-admin login --username alice --password '<password>'
```

This stores the JWT in `~/.config/cognee-admin/auth.json` under `cognee_jwt`.

2. Web/admin auth uses a `ca_xxx` token created by an admin. This is stored separately as `admin_token` and is used to access `https://cognee.origingame.dev`.

Legacy single-token configs are migrated automatically: old `token` values that start with `ca_` become `admin_token`; other tokens become `cognee_jwt`.

## Core Workflows

### Quick Start

Create a dataset, upload a file, build knowledge, then search it:

```bash
cognee-admin login --username alice --password '<password>'
cognee-admin dataset create gamedb-atoms
cognee-admin data add-file --dataset gamedb-atoms ./atoms/zelda-botw.md
cognee-admin cognify --dataset-name gamedb-atoms --background
cognee-admin search "open world traversal" --datasets gamedb-atoms --top-k 10
```

### Batch Import

Use `add-dir` for bulk markdown ingestion. It scans recursively, uploads in batches of 10, waits 1 second between batches, and reports success/failure counts.

```bash
cognee-admin data add-dir --dataset gamedb-atoms --glob "*.md" /path/to/atoms/md/
```

Use this for GameDB-scale imports rather than looping `add-file` yourself.

### Cognify With A Custom Prompt

Domain data usually needs a tailored extraction prompt. Prefer an explicit prompt or prompt file:

```bash
cognee-admin cognify \
  --dataset-name gamedb-atoms \
  --custom-prompt "Extract mechanics, progression systems, and player verbs. Normalize platform names." \
  --background
```

For long prompts, keep them in a file:

```bash
cognee-admin cognify \
  --dataset-name gamedb-atoms \
  --custom-prompt-file ./prompts/gamedb-cognify.txt
```

### Search Across Datasets

Filter search to specific datasets when you want narrower recall or to compare curated corpora:

```bash
cognee-admin search "crafting loop" --datasets gamedb-atoms,design-notes --top-k 10
```

Use `search history` to inspect previous queries:

```bash
cognee-admin search history
```

### Ontology Management

Upload an ontology file so it is available to Cognee:

```bash
cognee-admin ontology upload --key game-design ./ontology/game-design.owl
cognee-admin ontology list
```

If your workflow depends on ontology structure, reference the ontology key in your cognify prompt or the corresponding Cognee-side configuration, then run `cognify` for the target dataset.

## Command Quick Reference

| Command | Description |
|---------|-------------|
| `serve --host --port` | Start the web management panel. Requires DB access and `COGNEE_SERVICE_JWT`. |
| `health [--detailed] [--watch] [--interval]` | Check Cognee health and optionally record repeated snapshots. |
| `login --username --password` | Exchange credentials for a Cognee JWT and save it locally. |
| `dataset list` | List datasets. |
| `dataset create <name>` | Create a dataset. |
| `dataset delete <id>` | Delete one dataset. |
| `dataset delete-all --yes` | Delete every dataset. Destructive. |
| `dataset status` | Show dataset processing status. |
| `dataset graph <id>` | Fetch a dataset knowledge graph. |
| `data add --dataset <name> <content>` | Add inline text content. |
| `data add-file --dataset <name> <path>` | Upload one file. |
| `data add-dir --dataset <name> --glob "*.md" <dir>` | Upload matching files recursively in batches. |
| `data list <dataset-id>` | List dataset data items. |
| `data delete --dataset-id <id> --data-id <id>` | Delete one data item. |
| `data raw --dataset-id <id> --data-id <id>` | Fetch raw stored content. |
| `data update --dataset-id <id> --data-id <id> <path>` | Replace an existing data item with a new file. |
| `cognify ...` | Trigger knowledge construction with dataset filters and prompt overrides. |
| `search <query> [--datasets ...] [--top-k ...] [--verbose]` | Search the knowledge base. |
| `search history` | Retrieve search history. |
| `config get` | Read Cognee settings. |
| `config set '<json>'` | Update Cognee settings with a JSON payload. |
| `log list` / `log stats` | Query request logs from PostgreSQL. |
| `pipeline list` / `pipeline detail <id>` | Inspect pipeline run history from PostgreSQL. |
| `token create/list/revoke/delete` | Manage `ca_xxx` admin/user tokens. |
| `ontology upload --key <name> <file>` | Upload an OWL ontology file. |
| `ontology list` | List uploaded ontologies. |
| `describe [command]` | Emit machine-readable command schema. |

## Settings / LLM Configuration

Configure Cognee's LLM through the Settings page or `config set`.

The settings payload shape is:

```json
{
  "llm": {
    "provider": "openai|anthropic|ollama|gemini|mistral",
    "model": "string",
    "api_key": "string"
  }
}
```

Example using the Xiaomao LLM proxy through OpenAI-compatible routing:

```bash
cognee-admin config set '{"llm":{"provider":"openai","model":"claude-sonnet-4-6","api_key":"<proxy-key>"}}'
```

Important: the endpoint is not part of the Cognee settings API. Set `LLM_ENDPOINT=https://api.xiaomao.chat/v1` on the Cognee server to route these requests through the proxy. Good default models for this setup include `claude-sonnet-4-6`, `gpt-5.4`, and `gemini-3.1-pro-preview`.

## Web Dashboard

The dashboard is served at `https://cognee.origingame.dev`.

Login uses a `ca_xxx` token, not a Cognee JWT.

Key pages include:

| Route | Purpose |
|-------|---------|
| `/` | Health overview and recent operational metrics |
| `/graph` | Interactive knowledge graph view |
| `/datasets` | Dataset list and actions |
| `/search` | Interactive search UI |
| `/logs` | Request log inspection |
| `/pipelines` | Pipeline run history |
| `/settings` | Runtime config management |
| `/tokens` | Admin token management |

Use the web dashboard when a human operator needs an overview or repeated administrative actions are easier in a browser than in shell automation.

## Anti-Patterns

- Do not upload both JSON exports and equivalent markdown renderings of the same source unless you deliberately want duplicate semantic coverage.
- Do not run `cognify` on domain-specific corpora without a custom prompt; the default extraction is rarely specific enough for structured knowledge work.
- Do not use `dataset delete-all` without `--yes`, and do not automate it in unattended scripts unless the dataset lifecycle is explicitly disposable.
- Do not put upstream endpoints into `config set`; the endpoint belongs in server-side `LLM_ENDPOINT`, not in Cognee settings.
- Do not use the web-panel `ca_xxx` token as a substitute for `COGNEE_JWT` when running Cognee API commands.

## Reference Files

| File | Content |
|------|---------|
| `reference/npm-cli.md` | npm CLI 完整命令参考（安装、认证、所有命令用法） |
| `reference/datasets.md` | 数据集详情（game 数据集结构、搜索示例、导入 SOP、LLM 配置） |
