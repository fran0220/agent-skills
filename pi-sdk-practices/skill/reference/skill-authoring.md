# Skill Authoring Rules

This file captures practical skill authoring rules for Pi in Origin.

## Source Of Truth

- `dist/core/skills.js`
- Official `docs/skills.md`
- `building-skills` format constraints

## Required Structure

```text
.pi/skills/<skill-name>/
  SKILL.md
  reference/
  scripts/
```

`SKILL.md` must include YAML frontmatter and body instructions.

## Frontmatter Rules

- `name` is required and should match parent directory name.
- `description` is required and must be meaningful.
- Keep `name` lowercase with hyphens.
- Missing `description` causes skill to be skipped.

Recommended template:

```md
---
name: my-skill
description: Does X for Y scenarios. Use when Z happens.
---

# My Skill

Instructions...
```

## Discovery Rules In Local 0.56.2

- In a skill root directory, direct `*.md` files are scanned.
- In nested directories, scanner recognizes `SKILL.md`.
- Name collisions load first-seen skill and emit diagnostics.
- `node_modules` and hidden dirs are skipped.
- Ignore files (`.gitignore`, `.ignore`, `.fdignore`) are respected.

## Practical Recommendation

Even though root `*.md` is accepted, use `SKILL.md` in a dedicated directory for consistency with Agent Skills spec and cross-tool compatibility.

## Nested Categories

Nested structure is supported, e.g.:

```text
.pi/skills/
  quality/
    quality-criteria/
      SKILL.md
```

In that case, frontmatter `name` must match immediate parent directory (`quality-criteria`).

## Reference Material Behavior

`reference/` files are not eagerly loaded into context. They should hold detailed docs and be opened on demand.

## Authoring Conventions For Origin

- Keep top-level `SKILL.md` under 500 lines.
- Push detailed API tables and migration details into `reference/` files.
- Avoid duplicate policy statements across multiple skills; cross-link instead.
- Treat skill descriptions as triggers for retrieval quality.

## Validation Checklist

- Directory name equals `frontmatter.name`.
- `description` explains both capability and trigger scenario.
- All relative paths in instructions resolve from skill directory.
- Example commands are executable in this repository.
