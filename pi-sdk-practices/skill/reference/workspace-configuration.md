# Workspace Configuration (Pi Native)

This document captures workspace discovery behavior verified from local `0.56.2` source.

## Source Of Truth

- `dist/config.js`
- `dist/core/package-manager.js`
- `dist/core/resource-loader.js`
- `dist/core/skills.js`
- `dist/core/extensions/loader.js`

## Key Constants

- `CONFIG_DIR_NAME` resolves to `.pi` (`dist/config.js`, `pkg.piConfig.configDir || ".pi"`).
- Global agent dir defaults to `~/.pi/agent` (`getAgentDir()`).

## Effective Directory Layout

```text
{workspace}/.pi/
  extensions/
  skills/
  prompts/
  themes/
  settings.json
  SYSTEM.md
  APPEND_SYSTEM.md
```

## What Pi Auto-Discovers

### Extensions

Auto paths from project and global scope:

- Project: `{cwd}/.pi/extensions`
- Global: `~/.pi/agent/extensions`

Discovery behavior:

- Direct files: `*.ts` / `*.js`
- One-level subdir entries: `*/index.ts`, `*/index.js`, or `*/package.json` with `pi.extensions`
- Loading runtime uses `jiti` and expects `export default function(api)`

### Skills

Auto paths include:

- Project: `{cwd}/.pi/skills`
- Global: `~/.pi/agent/skills`
- Also `.agents/skills` ancestor chain support exists in package manager logic

Discovery behavior (`skills.js`):

- Root-level direct `*.md` files are accepted in a skills directory.
- Nested discovery uses `SKILL.md` under subdirectories.
- Skill frontmatter `name` is validated against parent dir name.
- Missing `description` prevents loading.
- `reference/` content is not auto-loaded; it is read on demand by the agent.

### Prompt Templates

Auto paths:

- Project: `{cwd}/.pi/prompts/*.md`
- Global: `~/.pi/agent/prompts/*.md`

### Context Instructions

`resource-loader` walks up from `cwd` to filesystem root and loads first-match candidates per directory:

- `AGENTS.md`
- `CLAUDE.md`

Global context file is also loaded from `agentDir`.

### System Prompt Files

Discovered in order:

1. `{cwd}/.pi/SYSTEM.md`
2. `~/.pi/agent/SYSTEM.md`

Append prompt file similarly:

1. `{cwd}/.pi/APPEND_SYSTEM.md`
2. `~/.pi/agent/APPEND_SYSTEM.md`

## Settings Merge Model

Settings files:

- Global: `~/.pi/agent/settings.json`
- Project: `{cwd}/.pi/settings.json`

Project settings override global values; nested objects are merged.

## Important Clarification For Origin

In local `0.56.2`, AGENTS/CLAUDE context loading is explicit in source; `IDENTITY.md` is not part of default context-file candidate list in `resource-loader`.
If Origin wants `IDENTITY.md` semantics, it must route them through AGENTS or custom resource hooks.

## Recommended Project Pattern

```text
/Users/fan/Origin/.pi/
  extensions/
    memory.ts
    review.ts
    worker.ts
    image-gen.ts
    web-search.ts
    visual.ts
    providers.ts
  skills/
    pi-sdk-practices/
      SKILL.md
      reference/
  settings.json
```
