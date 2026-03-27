---
name: godotforge
description: "AI-driven Godot 4.x game development via CLI. Use when user asks to create a game, build a Godot project, design game mechanics, generate scenes, test gameplay, or export builds. Requires: godot-forge CLI (npm install -g @doufunao123/godot-forge) + Godot 4.4+ in PATH."
---

# GodotForge — AI Game Development Playbook for Godot 4.x

Build complete Godot 4.x games through structured CLI commands. The CLI is your hands; this Skill is your brain.

## Prerequisites

1. **godot-forge CLI**: `npm install -g @doufunao123/godot-forge`
2. **Godot 4.4+** binary in PATH (as `godot`) — required only for L3 commands
3. **Node.js 20+**
4. Verify installation:

```bash
godot-forge --version   # CLI version
godot --version          # Godot engine (optional for L1/L2 only)
```

5. Discover any command's input/output schema:

```bash
godot-forge describe scene.create    # Single command schema
godot-forge describe --all           # Full CLI schema
```

## Required Workflow For Any Game Dev Task

1. **Check project state** before making changes:
   ```bash
   godot-forge project info
   godot-forge scene list
   ```
2. **Use `describe`** when unsure about a command's input format:
   ```bash
   godot-forge describe node.add
   ```
3. **Prefer stdin JSON** for complex inputs over flags
4. **Validate after mutations**: `godot-forge project validate` → `godot-forge engine validate`
5. **Never hand-edit `.godot/`** — it's engine cache
6. **Never guess UIDs** — use `godot-forge engine uid` to let the engine assign them
7. **Read `reference/` files** for detailed specs when this document is insufficient

## Four-Layer Architecture

```text
┌─────────────────────────────────────────────────────────┐
│ L1: Pure File Operations                   (No Godot)   │
│   project init/info/config/validate/clean               │
│   project autoload add/remove/list                      │
│   project input add/remove/list                         │
│   project plugin list/enable/disable                    │
│   project template list                                 │
│   script create/read/list/validate/edit                 │
│   export preset list/add/remove                         │
├─────────────────────────────────────────────────────────┤
│ L2: Godot Text Format (.tscn/.tres)        (No Godot)   │
│   scene create/read/list/delete/update/rename/merge     │
│   node add/remove/list/update/move/duplicate            │
│   node connect/disconnect/connections                   │
│   node group add/remove/list                            │
│   resource create/read/update/delete/list               │
├─────────────────────────────────────────────────────────┤
│ L3a: Godot Headless                     (Needs godot)   │
│   engine validate/run/import/uid/preview                │
│   export build                                          │
│   resource import/check                                 │
│   describe node-types/resource-types/class:*/           │
│           project-settings                              │
├─────────────────────────────────────────────────────────┤
│ L3b: Editor Context (ForgeSync TCP)     (Needs editor)  │
│   Auto-triggered: scene create → open_scene             │
│                   node add/remove/update → reload_scene  │
│   Falls back to .godot-forge/commands.json if no editor │
└─────────────────────────────────────────────────────────┘
```

**Decision rule:** Always prefer the lowest layer that accomplishes the task. L1/L2 are fast and need no engine binary. Only escalate to L3 for import, validation, runtime, UID assignment, introspection of engine types, or export builds.

## CLI Quick Reference — 52 Commands in 8 Groups

| Group | Subcommands | Layer |
|-------|-------------|-------|
| `project` | `init`, `info`, `validate`, `clean` | L1 |
| `project config` | `get <key>`, `set <key> <value>`, `list [section]` | L1 |
| `project autoload` | `add`, `remove`, `list` | L1 |
| `project input` | `add`, `remove`, `list` | L1 |
| `project plugin` | `list`, `enable`, `disable` | L1 |
| `project template` | `list` | L1 |
| `scene` | `create`, `read`, `list`, `delete`, `update`, `rename`, `merge` | L2 |
| `node` | `add`, `remove`, `list`, `update`, `move`, `duplicate` | L2 |
| `node` (signals) | `connect`, `disconnect`, `connections` | L2 |
| `node group` | `add`, `remove`, `list` | L2 |
| `script` | `create`, `read`, `list`, `validate`, `edit` | L1 |
| `engine` | `editor`, `validate`, `run`, `import`, `uid`, `preview` | L3a/L3b |
| `export` | `preset list`, `preset add`, `preset remove`, `build` | L1/L3a |
| `resource` | `create`, `read`, `update`, `delete`, `list`, `import`, `check` | L1/L2/L3a |
| `describe` | All 52 command schemas + engine introspection (`node-types`, `resource-types`, `class:<ClassName>`, `project-settings`) | L1/L3a |

### Global Flags

| Flag | Effect |
|------|--------|
| `--project <path>` | Override project directory (default: cwd) |
| `--human` | Human-readable output instead of JSON |
| `--fields <a,b,c>` | Filter output to specific fields |
| `--force` | Allow overwriting existing files / destructive ops |
| `--dry-run` | Preview changes without executing |
| `--input <file>` | Read JSON input from file instead of stdin |

## Input/Output Conventions

### Output Envelope (JSON default)

Every command returns:

```json
{"ok": true, "command": "scene.create", "data": {"path": "...", "node_count": 3}}
```

On error:

```json
{"ok": false, "command": "scene.create", "error": {"code": "SCENE_EXISTS", "message": "...", "suggestion": "Use --force"}}
```

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Command error (invalid input, missing file, validation failure) |
| `2` | Engine error (Godot binary not found, headless execution failed) |

### Input Methods

**Flags** — for simple operations:
```bash
godot-forge scene create --name player --root-type CharacterBody2D
```

**Stdin JSON** — for complex operations (preferred):
```bash
echo '{"name":"player","root_type":"CharacterBody2D","children":[{"name":"Sprite","type":"Sprite2D"}]}' | godot-forge scene create
```

**File input** — for reusable specs:
```bash
godot-forge scene create --input specs/player.json
```

### Scene Path Resolution

The CLI resolves scene names flexibly:
- `"player"` → `scenes/player.tscn`
- `"player.tscn"` → `player.tscn` (relative to project)
- `"res://scenes/player.tscn"` → resolved from project root
- `"/absolute/path.tscn"` → used as-is

## ForgeSync — Editor Bridge

TCP server on `127.0.0.1:23685` (override via `GODOT_FORGE_PORT` env).

**Auto-installed** by `project init` as `addons/forge_sync/`.

**Auto-triggered actions:**
- `scene create` → `open_scene` (editor opens new scene)
- `node add/remove/update/move` → `reload_scene` (editor refreshes)
- `scene rename` → `scan` (editor re-scans filesystem)

**Available commands:** `open_scene`, `reload_scene`, `select_node`, `scan`, `run_scene`, `stop`

**Fallback:** If editor not running, writes to `.godot-forge/commands.json` for file-based polling. No error thrown — L2 operations always succeed regardless of editor state.

## node add — Enhanced Capabilities

`node add` handles several advanced patterns automatically:

| Feature | Example | Auto-behavior |
|---------|---------|---------------|
| Script attachment | `{"script": "res://scripts/player.gd"}` | Auto-registers `ext_resource` for Script |
| Scene instancing | `{"instance": "res://scenes/bullet.tscn"}` | Auto-registers `ext_resource` for PackedScene, removes `type` |
| Auto sub_resource | `{"type": "CollisionShape2D"}` | Auto-creates `RectangleShape2D` sub_resource + assigns `shape` |
| Auto sub_resource | `{"type": "CollisionShape3D"}` | Auto-creates `BoxShape3D` sub_resource + assigns `shape` |

After any `node add`, `load_steps` in the scene header is auto-recalculated.

## Game Development Workflow (Condensed SOP)

The full 19-step SOP is in `reference/workflow.md`. Here is the condensed version:

### Phase 1: Foundation (L1)
```bash
# 1. Initialize project
godot-forge project init --name my-game --template default

# 2. Configure input actions
echo '{"name":"move_left","keys":["A","Left"]}' | godot-forge project input add
echo '{"name":"move_right","keys":["D","Right"]}' | godot-forge project input add
echo '{"name":"jump","keys":["Space","W","Up"]}' | godot-forge project input add

# 3. Set up autoloads (global singletons)
echo '{"name":"GameManager","path":"res://scripts/game_manager.gd"}' | godot-forge project autoload add
```

### Phase 2: Content Creation (L1/L2)
```bash
# 4. Create scripts
echo '{"name":"player","extends":"CharacterBody2D","signals":["hit","died"],"functions":[{"name":"_physics_process","params":[{"name":"delta","type":"float"}],"body":"velocity += get_gravity() * delta\\n\\tif Input.is_action_just_pressed(\"jump\") and is_on_floor():\\n\\t\\tvelocity.y = JUMP_VELOCITY\\n\\tmove_and_slide()"}]}' | godot-forge script create

# 5. Create scenes with nested children
echo '{"name":"player","root_type":"CharacterBody2D","children":[{"name":"Sprite","type":"Sprite2D"},{"name":"CollisionShape","type":"CollisionShape2D"},{"name":"Camera","type":"Camera2D"}]}' | godot-forge scene create

# 6. Attach scripts and wire nodes
echo '{"scene":"player","name":"Player","path":".","properties":{},"script":"res://scripts/player.gd"}' | godot-forge node update

# 7. Create level scenes, UI scenes, etc.
# 8. Connect signals
echo '{"scene":"main","from":"Player","signal":"hit","to":".","method":"_on_player_hit"}' | godot-forge node connect
```

### Phase 3: Engine Integration (L3)
```bash
# 9. Import assets (textures, audio)
godot-forge engine import

# 10. Assign UIDs
godot-forge engine uid

# 11. Validate project integrity
godot-forge engine validate

# 12. Test run
godot-forge engine run
godot-forge engine run --scene res://scenes/player.tscn  # Run specific scene
```

### Phase 4: Export (L3)
```bash
# 13. Add export preset
echo '{"name":"Windows","platform":"Windows Desktop","path":"builds/my-game.exe"}' | godot-forge export preset add

# 14. Build
godot-forge export build --preset Windows
godot-forge export build --preset Windows --dry-run  # Preview first
```

## Common Recipes

### Recipe 1: Platformer Player Character

```bash
# Create player script
echo '{
  "name": "player",
  "extends": "CharacterBody2D",
  "exports": [{"name":"speed","type":"float","default":300.0},{"name":"jump_velocity","type":"float","default":-400.0}],
  "signals": ["hit", "died"],
  "functions": [
    {"name":"_physics_process","params":[{"name":"delta","type":"float"}],
     "body":"velocity += get_gravity() * delta\n\tvar direction = Input.get_axis(\"move_left\", \"move_right\")\n\tvelocity.x = direction * speed\n\tif Input.is_action_just_pressed(\"jump\") and is_on_floor():\n\t\tvelocity.y = jump_velocity\n\tmove_and_slide()"}
  ]
}' | godot-forge script create

# Create player scene with collision and sprite
echo '{
  "name": "player",
  "root_type": "CharacterBody2D",
  "children": [
    {"name":"Sprite2D","type":"Sprite2D"},
    {"name":"CollisionShape2D","type":"CollisionShape2D"},
    {"name":"Camera2D","type":"Camera2D","properties":{"position_smoothing_enabled":"true"}}
  ]
}' | godot-forge scene create

# Attach script
echo '{"scene":"player","path":".","properties":{},"script":"res://scripts/player.gd"}' | godot-forge node update
```

### Recipe 2: UI HUD Scene

```bash
echo '{
  "name": "hud",
  "root_type": "CanvasLayer",
  "children": [
    {"name":"ScoreLabel","type":"Label","properties":{"text":"\"Score: 0\"","anchors_preset":0}},
    {"name":"HealthBar","type":"ProgressBar","properties":{"value":100,"max_value":100}},
    {"name":"PauseMenu","type":"PanelContainer","properties":{"visible":"false"},"children":[
      {"name":"VBox","type":"VBoxContainer","children":[
        {"name":"ResumeButton","type":"Button","properties":{"text":"\"Resume\""}},
        {"name":"QuitButton","type":"Button","properties":{"text":"\"Quit\""}}
      ]}
    ]}
  ]
}' | godot-forge scene create
```

### Recipe 3: Enemy with Signals and Groups

```bash
# Create enemy scene
echo '{
  "name": "enemy",
  "root_type": "CharacterBody2D",
  "children": [
    {"name":"Sprite","type":"AnimatedSprite2D"},
    {"name":"CollisionShape","type":"CollisionShape2D"},
    {"name":"DetectionArea","type":"Area2D","children":[
      {"name":"DetectionShape","type":"CollisionShape2D"}
    ]}
  ]
}' | godot-forge scene create

# Add to groups
echo '{"scene":"enemy","path":".","groups":["enemies","damageable"]}' | godot-forge node group add

# Connect area signal
echo '{"scene":"enemy","from":"DetectionArea","signal":"body_entered","to":".","method":"_on_detection_area_body_entered"}' | godot-forge node connect
```

### Recipe 4: Export for Multiple Platforms

```bash
# Add presets
echo '{"name":"Windows","platform":"Windows Desktop","path":"builds/win/game.exe"}' | godot-forge export preset add
echo '{"name":"Linux","platform":"Linux","path":"builds/linux/game.x86_64"}' | godot-forge export preset add
echo '{"name":"Web","platform":"Web","path":"builds/web/index.html"}' | godot-forge export preset add

# Dry-run then build
godot-forge export build --preset Windows --dry-run
godot-forge export build --preset Windows
godot-forge export build --preset Linux
godot-forge export build --preset Web
```

## Common Mistakes

| Mistake | Why it's wrong | Fix |
|---------|----------------|-----|
| Hand-editing `.godot/` directory | Engine cache — Godot owns it | Use `engine import` to refresh |
| Guessing UIDs like `uid://abc123` | UIDs must be engine-assigned | Run `godot-forge engine uid` |
| Using `--force` without `--dry-run` first | May destroy existing content | Always `--dry-run` before `--force` |
| Creating scenes via raw file writes | Misses `load_steps`, ext_resource IDs, ForgeSync sync | Always use `godot-forge scene create` |
| Skipping `engine import` after adding assets | Engine won't recognize new textures/audio | `engine import` after adding files to project |
| Running `engine run` before `engine validate` | May crash on broken references | Always validate → then run |
| Setting node types that don't exist | e.g. `CharacterBody3D` vs `CharacterBody2D` | Use `godot-forge describe node-types` to check |
| Modifying root node via `node update --path .` without `--scene` | CLI doesn't know which scene | Always specify `--scene` |
| Forgetting `recalcLoadSteps` in manual .tscn editing | Scene won't load in editor | Let the CLI handle it — it auto-recalculates |
| Complex input via flags instead of stdin JSON | Flags can't express nested children or arrays | Use stdin JSON for anything beyond simple ops |

## Engine Introspection via `describe`

For L3a introspection (requires Godot binary):

```bash
# List all available node types
godot-forge describe node-types

# List all resource types
godot-forge describe resource-types

# Get full class info (properties, signals, methods)
godot-forge describe class:CharacterBody2D

# List all project settings with defaults
godot-forge describe project-settings
```

These are invaluable for validating node types and property names before creating scenes.

## Reference Files

| File | Content | When to read |
|------|---------|-------------|
| `reference/workflow.md` | Full 19-step SOP with quality gates | Complex multi-phase projects |
| `reference/cli-commands.md` | Complete CLI reference with all fields | Unusual command parameters |
| `reference/godot-formats.md` | `.tscn` / `.tres` format specification | Debugging scene parse issues |
| `reference/quality-gates.md` | Quality gate definitions and criteria | Before marking phases complete |

## Domain Knowledge

| File | Content | When to read |
|------|---------|-------------|
| `knowledge/game-design.md` | Core loop design, MDA framework | Game concept / mechanic design |
| `knowledge/godot-patterns.md` | Scene composition, signals, state machines | Architecture decisions |
| `knowledge/scene-organization.md` | Directory structure conventions | Project layout questions |
| `knowledge/feel-presets.md` | Game feel parameters (gravity, friction, etc.) | Tuning movement / physics |
| `knowledge/anti-patterns.md` | Common Godot development mistakes | Code review / debugging |
