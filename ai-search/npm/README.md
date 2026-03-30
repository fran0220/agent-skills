# @doufunao123/ai-search

Gateway client for AI Search Gateway. The npm CLI is a thin HTTP client that talks to `search.xiaomao.chat` instead of calling model APIs directly.

## Install

```bash
npm install -g @doufunao123/ai-search
```

## Setup

```bash
ai-search auth set your_gateway_token

# Or use environment variables
export AI_SEARCH_TOKEN=your_gateway_token
export AI_SEARCH_GATEWAY_URL=https://search.xiaomao.chat
```

## Usage

```bash
# Default fast search
ai-search "latest AI news"

# Deep multi-source search
ai-search "compare latest AI browser agents" --mode deep

# Answer mode
ai-search "what changed in Bun this month" --mode answer

# Choose model and query splitting
ai-search "comprehensive analysis of X" --model grok-4.1-fast --split 3 --num 5

# stdin JSON input
printf '%s\n' '{"query":"latest AI infra news","mode":"deep","num":5}' | ai-search search --stdin

# List gateway metadata
ai-search models
ai-search providers
ai-search providers health
ai-search health

# Config and auth
ai-search auth status
ai-search config set gateway_url https://search.xiaomao.chat
ai-search config show
```

## Configuration

Config file: `~/.config/ai-search/auth.json`

Saved format:

```json
{
  "token": "asg_...",
  "gateway_url": "https://search.xiaomao.chat"
}
```

Environment variables:
- `AI_SEARCH_TOKEN` — gateway token
- `AI_SEARCH_GATEWAY_URL` — gateway URL

Priority order:
1. CLI flags (`--gateway-url`, `--token`)
2. Environment variables
3. `~/.config/ai-search/auth.json`
4. Default gateway URL `https://search.xiaomao.chat`

## Search Modes

- `fast` — Grok only, default mode
- `deep` — Grok + Exa + Tavily in parallel
- `answer` — Tavily answer mode with AI summary
