---
name: grok-search
description: "Grok-powered real-time web search using grok-4.1-expert. Use when the user asks to search the web with Grok, wants high-quality real-time search results, or asks to use grok-search."
---

# Grok Search

Real-time web search powered by Grok models via grok2api. Uses `grok-4.1-expert` by default for highest search quality.

## Prerequisites

- `grok-search` CLI installed: `ln -s /path/to/agent-skills/grok-search/cli/grok-search ~/.local/bin/grok-search`
- Environment: `GROK_API_KEY` set

## Model Selection

| Model | Quality | Speed | Cost (quota/req) | Use Case |
|-------|---------|-------|-------------------|----------|
| `grok-4.1-expert` | 🏆 Best | Slow | 4 | Deep research, fact-checking |
| `grok-4.1-fast` | Good | Fast | 1 | Daily search, quick lookups |
| `grok-4.20-beta` | Good | Medium | 1 | Latest features |
| `grok-4` | Good | Medium | 1 | General purpose |
| `grok-4-thinking` | Good+ | Slow | 1 | Complex reasoning |

**Default: `grok-4.1-expert`** — auto-corrects dates, returns most accurate results.

## Workflow

### Basic Search

```bash
grok-search "query here"
```

### Specify Model (save quota)

```bash
grok-search "query" --model grok-4.1-fast
```

### JSON Output (for Agent parsing)

```bash
grok-search "query" --json
```

### From stdin (batch/pipeline)

```bash
echo '{"query":"latest AI news","model":"grok-4.1-fast","num":3}' | grok-search --stdin
```

### List Models

```bash
grok-search --models
```

## Agent Integration

When using grok-search from an agent:

1. Use `--json` flag for structured output
2. Parse the `data.content` field from the JSON response
3. Use `grok-4.1-fast` for routine searches to save quota (1 vs 4 per request)
4. Use `grok-4.1-expert` only for important research requiring high accuracy

## Configuration

| Env Variable | Description | Default |
|-------------|-------------|---------|
| `GROK_API_URL` | grok2api endpoint | `https://grok.xiaomao.chat` |
| `GROK_API_KEY` | API bearer token | (required) |
