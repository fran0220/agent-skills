# Quality Gates — godot-forge Development Phases

Checklist-based quality gates for each phase of game development with godot-forge. Every gate must pass before moving to the next phase.

> Run all commands from the project root. All commands output JSON by default; add `--human` for readable output.

---

## Phase 1: Project Setup (Steps 1–3)

Ensure the project is properly initialized and configured before building anything.

### Gate: Project Valid

```bash
godot-forge project info
godot-forge project validate
```

- [ ] `godot-forge project info` returns valid project metadata (name, Godot version, paths)
- [ ] `godot-forge project validate` passes with no errors
- [ ] `project.godot` exists with `config_version=5`
- [ ] Main scene path is set: check `run/main_scene` in project info

### Gate: Input Actions Registered

```bash
godot-forge project input list
```

- [ ] All gameplay input actions are registered (movement, jump, attack, etc.)
- [ ] No placeholder or unused input actions remain
- [ ] Key bindings match the game design document

### Gate: Autoloads Configured

```bash
godot-forge project autoload list
```

- [ ] Required singletons are registered (GameManager, AudioManager, etc.)
- [ ] Autoload scripts exist at their declared paths
- [ ] No stale autoloads pointing to deleted scripts

### Gate: Directory Structure

```bash
ls -la scenes/ scripts/ assets/ resources/
```

- [ ] Standard directories exist: `scenes/`, `scripts/`, `assets/`, `resources/`
- [ ] Asset subdirectories match project needs (e.g., `assets/textures/`, `assets/audio/`)

---

## Phase 2: Asset Pipeline (Steps 4–5)

All assets imported and indexed before scene construction begins.

### Gate: Assets Organized

- [ ] All textures in `assets/textures/` (or project-specific path)
- [ ] All audio files in `assets/audio/`
- [ ] All fonts in `assets/fonts/`
- [ ] No assets in project root or ad-hoc locations
- [ ] File names use snake_case (e.g., `player_idle.png`, not `Player Idle.png`)

### Gate: Engine Import

```bash
godot-forge engine import
```

- [ ] `godot-forge engine import` succeeds with exit code 0
- [ ] `.godot/imported/` directory is populated
- [ ] No import errors in output

### Gate: UIDs Assigned

```bash
godot-forge engine uid
```

- [ ] `godot-forge engine uid` shows UIDs assigned to all resources
- [ ] No resources without UIDs (unless intentionally excluded)
- [ ] `uid://` references in `.tscn`/`.tres` files resolve correctly

### Gate: No Missing References

```bash
godot-forge resource check
```

- [ ] `godot-forge resource check` finds zero missing references
- [ ] All `ext_resource` paths resolve to existing files
- [ ] All `res://` paths in scripts and scenes are valid

---

## Phase 3: Scene Construction (Steps 6–9)

Build scenes with correct node hierarchies, scripts, and signal connections.

### Gate: Scene Inventory

```bash
godot-forge scene list
```

- [ ] `godot-forge scene list` shows all expected scenes from the design document
- [ ] Main scene, UI scenes, level scenes, and prefab scenes are all present
- [ ] No orphan `.tscn` files unaccounted for

### Gate: Scene Readability

```bash
godot-forge scene read <scene_path>
```

- [ ] Each scene is parseable: `godot-forge scene read <scene>` succeeds for every scene
- [ ] No syntax errors or malformed sections
- [ ] `load_steps` values are correct

### Gate: Node Hierarchy

```bash
godot-forge node list --scene <scene_path>
```

- [ ] `godot-forge node list --scene <scene>` shows correct parent-child relationships
- [ ] Root node type matches scene purpose (Node2D for 2D levels, Control for UI, etc.)
- [ ] No deeply nested hierarchies without justification (flatter is better)
- [ ] Node names are descriptive and unique among siblings

### Gate: Signal Connections

```bash
godot-forge node connections --scene <scene_path>
```

- [ ] `godot-forge node connections --scene <scene>` shows expected signals
- [ ] All button `pressed` signals are connected
- [ ] All `body_entered`/`area_entered` signals are connected for physics interactions
- [ ] No dangling connections to removed nodes or renamed methods

### Gate: Script Validation

```bash
godot-forge script validate <script_path>
```

- [ ] `godot-forge script validate <path>` passes for every script
- [ ] All scripts attached to scenes via `ext_resource` exist on disk
- [ ] No `extends` mismatches (script extends CharacterBody2D but attached to Node2D)

---

## Phase 4: Integration (Steps 10–15)

All pieces connected and cross-validated.

### Gate: Resource Inventory

```bash
godot-forge resource list
```

- [ ] `godot-forge resource list` shows all resources categorized by type
- [ ] No uncategorized or unexpected resources
- [ ] Resource count matches expectations from design

### Gate: Reference Integrity

```bash
godot-forge resource check
```

- [ ] `godot-forge resource check` finds zero broken references
- [ ] No circular dependencies between scenes
- [ ] Instanced scenes (`instance=ExtResource`) all resolve

### Gate: Full Project Validation

```bash
godot-forge project validate
godot-forge engine validate
```

- [ ] `godot-forge project validate` still passes (re-check after all construction)
- [ ] `godot-forge engine validate` passes (lightweight engine check)
- [ ] No regressions from Phase 1 gates

### Gate: Cross-Scene Consistency

- [ ] Shared resources (themes, materials) are `.tres` files, not duplicated inline
- [ ] Autoload references in scripts match registered autoloads
- [ ] Scene instancing hierarchy has no cycles (A instances B instances A)

---

## Phase 5: Testing & Export (Steps 16–19)

Verify runtime behavior and produce export builds.

### Gate: Deep Validation

```bash
godot-forge engine validate --deep
```

- [ ] `godot-forge engine validate --deep` passes (requires Godot binary)
- [ ] All scripts compile without errors
- [ ] All resource references resolve at engine level
- [ ] No runtime warnings about missing nodes or properties

### Gate: Runtime Test

```bash
godot-forge engine run --scene <main_scene>
```

- [ ] `godot-forge engine run --scene <main_scene>` launches without errors
- [ ] No crash on startup
- [ ] Core gameplay loop is functional (player can move, interact, etc.)
- [ ] No error output in Godot console/logs

### Gate: Export Presets

```bash
godot-forge export preset list
```

- [ ] `godot-forge export preset list` shows configured presets for all target platforms
- [ ] Each preset has correct export path set
- [ ] Export templates are installed for target Godot version

### Gate: Export Build

```bash
godot-forge export build --preset <preset_name>
```

- [ ] `godot-forge export build --preset <name>` succeeds for each target platform
- [ ] Output binaries exist at expected export paths
- [ ] Build file sizes are reasonable (no accidentally embedded development assets)
- [ ] Exported build runs independently (test on clean machine if possible)

---

## Quick Reference: Full Pipeline Check

Run this sequence to validate an entire project end-to-end:

```bash
# Phase 1: Setup
godot-forge project info
godot-forge project validate
godot-forge project input list
godot-forge project autoload list

# Phase 2: Assets
godot-forge engine import
godot-forge engine uid
godot-forge resource check

# Phase 3: Scenes
godot-forge scene list
godot-forge scene read scenes/main.tscn
godot-forge node list --scene scenes/main.tscn
godot-forge node connections --scene scenes/main.tscn
godot-forge script validate scripts/player.gd

# Phase 4: Integration
godot-forge resource list
godot-forge resource check
godot-forge project validate
godot-forge engine validate

# Phase 5: Export
godot-forge engine validate --deep
godot-forge engine run --scene scenes/main.tscn
godot-forge export preset list
godot-forge export build --preset "Windows Desktop"
```

If any gate fails, fix the issue and re-run from that phase. Do not skip ahead — later phases depend on earlier gates passing.
