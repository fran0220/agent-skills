# grok-search

Grok-powered real-time web search CLI + Agent Skill.

Uses [grok2api](https://github.com/fran0220/grok2api) as backend, defaults to `grok-4.1-expert` for highest quality search results.

## Install

```bash
# CLI
chmod +x cli/grok-search
ln -s $(pwd)/cli/grok-search ~/.local/bin/grok-search

# Skill (for Amp)
ln -s $(pwd)/skill ~/.config/amp/skills/grok-search

# Config
export GROK_API_KEY=your_api_key
```

## Usage

```bash
grok-search "latest AI news"
grok-search "对比 React 和 Vue" --model grok-4.1-fast
grok-search "query" --json | jq .data.content
```
