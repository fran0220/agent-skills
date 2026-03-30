---
name: ai-search
description: "Runs multi-source web search through the AI Search Gateway. Use when the user wants fresh web results, research across sources, AI summaries, or MCP-based search integration."
---

# AI Search Gateway

`ai-search` is the gateway-first CLI for web research. Prefer it over calling Grok, Exa, or Tavily directly.

## Prerequisites

```bash
npm install -g @doufunao123/ai-search
ai-search auth set <gateway_token>
```

Gateway: `https://search.xiaomao.chat` (default, no extra config needed)
Admin panel: `https://search.xiaomao.chat/admin`

## Quick Reference

| Mode | Providers | Use When |
|------|-----------|----------|
| `fast` | Grok | Need a quick default web search |
| `deep` | Grok + Exa + Tavily | Need broader source coverage or validation |
| `answer` | Tavily | Need an AI summary / direct answer |

## Workflow

### Quick search (fast, Grok only)
```bash
ai-search "latest AI news"
ai-search "今天有什么科技新闻" --human
```

### Deep multi-source research
```bash
ai-search "compare MCP gateway patterns for AI tools" --mode deep
```

### AI answer mode (Tavily summary)
```bash
ai-search "What changed in SvelteKit remote functions?" --mode answer
```

### Query splitting (complex research)
```bash
ai-search "全面分析 2025 年 AI 芯片市场格局" --mode deep --split 3
```

### JSON output for piping
```bash
ai-search "query" --json | jq .data.content
```

### Inspect providers and models
```bash
ai-search models
ai-search providers
ai-search providers health
```

## Provider Guide

| Provider | What It Adds |
|----------|--------------|
| Grok | Fast default search through the Xiaomao proxy |
| Exa | Semantic retrieval and source expansion for `deep` |
| Tavily | Search + answer generation for `deep` and `answer` |

## Output Contract

Every command returns the monorepo JSON envelope:

```json
{"ok":true,"command":"search","data":{"query":"latest AI news","mode":"deep","results":[]}}
```

Errors use the same envelope shape with `ok: false` and an `error` object.

## Rules

- Prefer the hosted gateway when available instead of direct provider APIs.
- Use `deep` only when the extra coverage matters; `fast` should stay the default.
- Use `answer` when the user wants a concise summary rather than raw sources.
- Check provider health before blaming a query for weak results.
- Do not hard-code provider keys in prompts, scripts, or temporary files.
