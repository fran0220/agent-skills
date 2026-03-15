---
name: godotforge
description: "AI-driven Godot 4.x game development via CLI. Use when user asks to create a game, build a Godot project, design game mechanics, generate scenes, test gameplay, or export builds. Requires: godot-forge CLI (pip install godot-forge) + Godot 4.4+ in PATH."
---

# GodotForge — AI Game Development for Godot

Build complete Godot 4.x games through CLI commands.

## Prerequisites

1. **godot-forge CLI**: `pip install godot-forge`
2. **Godot 4.4+** binary in PATH (as `godot`)
3. Verify: `godot-forge --version && godot --version`

## CLI Commands

```bash
godot-forge design    gdd|art|narrative|balance    # 设计文档生成
godot-forge project   init|info|config             # 项目管理
godot-forge asset     import|manifest|check        # 资产管理
godot-forge scene     create|add-node|list|info    # 场景操作
godot-forge engine    import|uid|validate|run      # 引擎操作
godot-forge test      run|report                   # 测试
godot-forge export    build|preset                 # 构建导出
```

All commands support `--json` for machine-readable output.

## Three-Layer Control Strategy

| Layer | Method | Needs Godot? |
|---|---|---|
| L1: Pure files | Generate `.md/.json` | ❌ |
| L2: Godot text | Read/write `.tscn/.tres/.gd` | ❌ |
| L3: Engine | `godot --headless -s script.gd` | ✅ |

## Key Rules

1. **Real software only** — rendering/export/physics must use real Godot
2. **Never guess UIDs** — use `godot-forge engine uid` to let engine assign
3. **Never hand-edit `.godot/`** — it's engine cache
4. **After changes**: import → uid → validate → run/test

## Reference Documents

- `reference/workflow.md` — 19-step SOP
- `reference/cli-commands.md` — CLI reference
- `reference/godot-formats.md` — .tscn/.tres format spec
- `reference/quality-gates.md` — Quality gates

## Domain Knowledge

- `knowledge/game-design.md` — Core loop, MDA framework
- `knowledge/godot-patterns.md` — Scene/signal/state-machine
- `knowledge/scene-organization.md` — Project structure
- `knowledge/feel-presets.md` — Game feel parameters
- `knowledge/anti-patterns.md` — Common mistakes
