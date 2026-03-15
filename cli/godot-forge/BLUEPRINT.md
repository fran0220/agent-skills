# Design Blueprint — `godot-forge`

> AI Agent 驱动的 Godot 4.x 游戏开发 CLI。从零到发布的完整流水线。

## 1. Purpose

Provide a unified CLI that lets AI agents create, build, test, and export complete Godot 4.x games through structured commands — orchestrating Godot engine operations, file generation, and external asset-generation services in a single tool.

## 2. Classification

- **Primary role:** Workflow / Orchestration
- **Primary user type:** Agent-Primary
- **Primary interaction form:** Batch CLI
- **Statefulness:** Config-Stateful (project files on disk; no sessions — agent manages context)
- **Risk profile:** Mixed (file generation is low-risk; engine operations are medium-risk; export/publish is high-risk)
- **Secondary surfaces:** Capability sub-surface (scene/node/asset CRUD), Package/Build sub-surface (export/build), External Service sub-surface (asset generation APIs)
- **Confidence level:** High
- **Hybrid notes:** Subcommands span three distinct sub-roles — `scene`/`asset`/`script` are Capability-like CRUD; `project`/`design` are Workflow orchestration; `export` is Package/Build. The CLI's center of gravity is the end-to-end game creation pipeline, which makes Workflow the primary role.
- **Evolution trajectory:** v1 covers core Godot operations. External service integration (image/audio/3D generation) will expand the Workflow surface in v2+.

## 2b. Classification reasoning

**Why Workflow/Orchestration, not Capability.**
A Capability CLI manages resources at rest. `godot-forge` orchestrates a 19-step pipeline from concept design to published build. Individual scene/node operations are Capability-like, but the product's center of gravity is "build a complete game," not "CRUD game objects." If you stripped the pipeline orchestration and only kept scene CRUD, you would lose the CLI's reason to exist.

**Why Workflow/Orchestration, not Package/Build.**
Package/Build CLIs manage dependency resolution, environment lifecycle, and artifact publishing. `godot-forge` touches build/export, but its primary surface is game content creation (design → scenes → scripts → assets), not dependency management. Export is one step in a larger workflow, not the defining activity.

**Why Agent-Primary, not Balanced.**
Humans interact with `godot-forge` exclusively through AI agents. The agent interprets user intent, issues structured CLI commands, and processes JSON output. No human types `godot-forge scene create` directly. This means: JSON is the default output, schema introspection replaces help text as the primary discoverability surface, and input hardening targets agent failure modes (hallucinated paths, wrong node types) rather than human typos.

**Why Config-Stateful, not Sessionful.**
The CLI does not maintain sessions. All state lives in the project directory as files (`.tscn`, `.gd`, `.tres`, `project.godot`). The agent holds context across invocations; the CLI is a stateless executor against a stateful filesystem. This is the same model as `git` — durable side-effects, no session identity.

## 3. Primary design stance

Optimize for an AI agent that builds a complete Godot game through a sequence of structured CLI calls. Each command should be a self-contained, composable step in the game creation pipeline. The CLI is the agent's hands; the Skill (SKILL.md) is the agent's brain.

`godot-forge` is **not** trying to be:
- An interactive game editor (that's Godot's editor)
- A game engine runtime (that's Godot itself)
- A general-purpose file manipulation tool (that's the filesystem)
- A human-facing CLI with rich help and discoverability (agents don't read help)

## 4. Command structure

**Shape:** Phase-oriented command groups reflecting the game creation pipeline.

```
godot-forge project   init|info|config|validate      # Project lifecycle
godot-forge design    gdd|art|narrative|balance       # Design documents (L1: pure files)
godot-forge scene     create|read|update|delete|list  # Scene operations (L1/L2)
godot-forge node      add|remove|move|update|list     # Node tree manipulation (L1/L2)
godot-forge script    create|edit|validate|list       # GDScript management (L1)
godot-forge resource  create|import|list|check        # Resource management (L1/L2)
godot-forge engine    import|uid|validate|run|preview # Engine operations (L3: needs Godot)
godot-forge export    preset|build                    # Build & export (L3)
godot-forge generate  image|audio|model|voice         # External service calls (v2+)
godot-forge describe  <command-group>                 # Schema introspection
```

**Why this shape.** Workflow role drives phase-oriented grouping. Each group maps to one or more steps in the 19-step SOP. Within each group, subcommands are CRUD-like (Capability secondary surface) because individual operations manipulate game objects. The top-level groups reflect the pipeline; the subcommands reflect the objects.

**Three-layer execution model** (unchanged from current design):

| Layer | Method | Needs Godot? | Commands |
|-------|--------|:---:|-----------|
| L1: Pure files | Generate/modify `.md`, `.json` | ❌ | `design *`, `script create` |
| L2: Godot text | Read/write `.tscn`, `.tres`, `.gd` | ❌ | `scene *`, `node *`, `resource create` |
| L3: Engine | `godot --headless -s script.gd` | ✅ | `engine *`, `export *`, `resource import` |

## 5. Input model

**Primary:** Raw-payload-first. Agent sends structured JSON via stdin or `--input` flag.

```bash
# Primary path: raw JSON payload
echo '{"name":"player","root_type":"CharacterBody2D","children":[{"name":"Sprite","type":"Sprite2D"}]}' | godot-forge scene create

# Also accepted: JSON file reference
godot-forge scene create --input scene-spec.json

# Convenience: flags for simple operations (secondary, for debugging)
godot-forge scene create --name player --root-type CharacterBody2D
```

**Why raw-payload-first.** Agent-Primary user type means the primary consumer constructs structured data, not flag strings. Complex game objects (scene trees with nested children, script attachments, resource references) cannot be expressed cleanly in flags. JSON payloads map directly to Godot's object model.

**Project context:** All commands operate on the current directory by default. Override with `--project <path>`. The agent is responsible for `cd`-ing to the right project or passing `--project`.

## 6. Output model

**Default:** JSON. Every command outputs structured JSON to stdout.

```json
{
  "ok": true,
  "command": "scene.create",
  "data": {
    "path": "res://scenes/player.tscn",
    "root_type": "CharacterBody2D",
    "node_count": 3
  }
}
```

**Error output:**

```json
{
  "ok": false,
  "command": "scene.create",
  "error": {
    "code": "INVALID_NODE_TYPE",
    "message": "Node type 'CharacterBody3D' does not exist in Godot 4.4",
    "suggestion": "Did you mean 'CharacterBody2D'?"
  }
}
```

**Human-readable secondary:** `--human` flag for debugging. Produces formatted, colored output. This is a convenience layer, not a contract.

**Field selection:** `--fields path,root_type,node_count` to limit output. Critical for agent context-window discipline.

**Contract level:** JSON output field names are stable across minor versions. Envelope shape (`ok`, `command`, `data`/`error`) is guaranteed. Breaking changes require major version bump.

## 7. Help / discoverability / introspection

**Schema introspection (primary — Agent-Primary drives this):**

```bash
# Describe a command group's input schema
godot-forge describe scene.create
# → JSON schema of accepted input payload

# Describe all command groups
godot-forge describe --all
# → Full CLI schema (commands, input shapes, output shapes)

# List available node types (domain introspection)
godot-forge describe node-types
# → ["Node2D", "CharacterBody2D", "Sprite2D", ...]
```

**Why `describe` instead of rich `--help`.** Agents don't read help text — they consume schema definitions. `godot-forge describe` is the primary discoverability surface. It returns structured JSON that agents can use to construct valid payloads.

**Help (secondary — for human debugging):**
- `--help` exists on every command with minimal examples.
- Not the primary discoverability path.

## 8. State / session model

**No sessions.** The CLI is Config-Stateful, not Sessionful.

Project state lives entirely in the filesystem:
- `project.godot` — project configuration
- `*.tscn` — scenes
- `*.gd` — scripts
- `*.tres` — resources
- `.godot/` — engine cache (never touched by CLI)

The agent manages context (which scene is being edited, what the current task is). The CLI does not track "current scene" or "editing context" — every command receives the full specification it needs.

**State inspection:**
- `godot-forge project info` — project metadata, scene list, script list
- `godot-forge project validate` — check project integrity (missing resources, broken references)

**No init/reset/status ceremony needed** beyond `project init` and `project info`. The filesystem IS the state.

## 9. Risk / safety model

**Low-risk operations (no guardrails):**
- `project info`, `scene list`, `node list`, `describe *` — read-only inspection
- `design *` — generates markdown files, no engine mutation

**Medium-risk operations (validation before execution):**
- `scene create`, `node add`, `script create` — file creation/modification
  - Guardrail: refuse to overwrite existing files without `--force`
  - Guardrail: validate node types and resource paths against known Godot types
- `engine import`, `engine uid` — engine operations that modify `.godot/` cache
  - Guardrail: `engine validate` should run before `engine run`

**High-risk operations (explicit confirmation or flag required):**
- `export build` — produces final build artifacts
  - Guardrail: requires `--preset` to be explicitly named
  - Guardrail: `--dry-run` shows what would be built without building
- `scene delete`, `node remove` — destructive operations
  - Guardrail: `--force` required, default is to fail

**Dry-run:** `--dry-run` on mutation commands shows the planned changes as a JSON diff without executing.

**Audit:** Every mutation command includes a `changes` field in output listing affected files.

## 10. Hardening model

**Input validation (Agent-specific hardening):**
- Unknown JSON fields in payloads → rejected with error listing unknown fields
- Hallucinated node types (`CharacterBody3D` doesn't exist) → rejected with suggestion
- Path traversal (`../../etc/passwd`) → rejected
- Double-encoded JSON → detected and rejected with clear error
- Empty/null required fields → rejected with field name

**Field stability:** JSON output field names are stable. New fields may be added; existing fields never removed or renamed without major version bump.

**Exit codes:**
- `0` = success
- `1` = command error (invalid input, missing file, validation failure)
- `2` = engine error (Godot headless failed, import error)
- `3` = external service error (asset generation API failure)

**Idempotency:**
- `scene create` with identical input and `--force` produces identical output
- `node add` to a scene that already has that node → error (not silent no-op)
- `engine import` is idempotent (re-import is safe)

## 11. Secondary surface contract

**Capability sub-surface (scene/node/asset CRUD):**
- *Who:* Agents performing granular game object manipulation.
- *What:* CRUD operations on scenes, nodes, scripts, resources.
- *Contract:* Strong. Input/output schemas are documented via `describe`. Field names are stable.

**Package/Build sub-surface (`export`):**
- *Who:* Agents and CI pipelines producing game builds.
- *What:* Export presets and build execution.
- *Contract:* Strong. Exit codes distinguish build-success from build-failure. Output includes artifact paths and sizes.

**External Service sub-surface (`generate`) — v2+:**
- *Who:* Agents generating game assets via external AI services.
- *What:* Image, audio, 3D model, voice generation through configured service backends.
- *Contract:* Convenience layer in v1. Service configuration via project-level `.godotforge.json`. Output includes generated file paths and service metadata.

**Human-readable sub-surface (`--human`):**
- *Who:* Developers debugging agent behavior.
- *What:* Formatted, colored terminal output.
- *Contract:* Convenience only. No stability guarantee on format.

## 12. v1 boundaries

**v1 includes:**
- `project init|info|config|validate` — project lifecycle
- `design gdd` — GDD generation (other design docs deferred)
- `scene create|read|update|delete|list` — full scene CRUD
- `node add|remove|move|update|list` — node tree manipulation
- `script create|edit|validate|list` — GDScript management
- `resource create|import|list|check` — resource management
- `engine import|uid|validate|run|preview` — engine operations
- `export preset|build` — build and export
- `describe` — schema introspection for all commands
- `.tscn` / `.tres` parser and generator (TypeScript)
- JSON envelope contract (`ok`, `command`, `data`/`error`)
- `--fields`, `--dry-run`, `--force`, `--human` global flags

**v1 defers:**
- `generate *` — external service integration (image/audio/model/voice)
- Plugin/addon management
- Multi-project workspace support
- Incremental build / change detection
- GDExtension support
- Multiplayer/networking scaffolding
- Visual shader generation

**What would be premature abstraction:**
- Building a plugin system before the core command surface is stable — plugin boundaries cannot be drawn until real agent usage reveals where extensibility is needed.
- Designing a universal "asset pipeline" abstraction — v1 should hardcode the Godot import flow and learn from usage before abstracting.
- Adding session/state tracking in the CLI — the agent manages context; adding sessions would duplicate state and create sync bugs.
- Building a service registry for external generators — v1 should defer `generate` entirely; if prototyped, hardcode one provider per type.

## 13. Direction for implementation

**Optimize for:** Composable, self-contained commands that an agent can chain into game creation workflows. Each command should accept a complete specification and produce a complete result. The agent should never need to "remember" CLI state between calls.

**Do not optimize for:** Human ergonomics, interactive wizards, rich terminal UX, or session continuity. These are all agent responsibilities, not CLI responsibilities.

**Acceptable patterns:**
- TypeScript + Commander.js (or oclif) for CLI framework
- Direct `.tscn`/`.tres` text parsing/generation in TypeScript (no external parser dependency)
- `child_process.execSync` for Godot headless calls
- Project-level `.godotforge.json` for CLI configuration (default service endpoints, export presets)
- npm package distribution: `npm install -g @doufunao123/godot-forge` or `npx @doufunao123/godot-forge`

**What would be a category mistake:**
- Designing `godot-forge` as a REPL or interactive session — agents don't need interactive sessions; they issue discrete commands.
- Making human-readable output the default — this is Agent-Primary; JSON is the default.
- Adding `--json` as an opt-in flag — JSON IS the default; `--human` is the opt-in.
- Building rich `--help` with extensive examples as the primary discoverability surface — agents use `describe`, not `help`.
- Requiring a "project context" to be set before commands work (e.g., `godot-forge use <project>`) — this invents session semantics where none are needed.
- Wrapping Godot editor GUI operations — the CLI operates on files and headless engine, not on the visual editor.

---

## Technology Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | TypeScript | Native JSON, template literals for .tscn/.gd generation, async for external services |
| CLI Framework | Commander.js | Lightweight, mature, good subcommand support |
| Package Manager | npm | Standard TS distribution |
| .tscn/.tres Parser | Custom (in-repo) | No existing library; text format is well-documented |
| Godot Integration | `child_process` → `godot --headless` | Standard subprocess pattern |
| Testing | Vitest | Fast, TypeScript-native |
| Build | tsup / esbuild | Single-file bundle for fast startup |
| Distribution | `npm install -g` / `npx` | Zero-config for agents |
