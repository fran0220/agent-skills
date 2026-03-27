# Game Development Workflow — 19-Step SOP

Standard operating procedure for building a Godot 4.x game from concept to release using godot-forge CLI.

## Layer Legend

| Badge | Layer | Godot Required? |
|-------|-------|:---:|
| 🟢 | L1 — Pure file operations | ❌ |
| 🔵 | L2 — .tscn/.tres text format | ❌ |
| 🟠 | L3a — Godot headless runtime | ✅ |
| 🟣 | L3b — Godot editor context | ✅ |

---

## Step 1: Concept Design / GDD 🟢

**Goal**: Define the game's vision, mechanics, and scope.

The Agent generates a Game Design Document as a Markdown file. No CLI commands needed yet (future: `godot-forge design gdd`).

**Deliverables**:
- `docs/gdd.md` — Game Design Document
- Core loop definition (see `knowledge/game-design.md`)
- 3-5 design pillars
- Target platform(s)

**Quality Gate**: GDD exists with clear core loop, target audience, and scope.

**Pitfalls**: Scope creep. Start with Minimum Viable Game.

---

## Step 2: Project Initialization 🟢

**Goal**: Create a valid Godot 4.x project with ForgeSync plugin.

```bash
# List available templates
godot-forge project template list

# Initialize project
godot-forge project init --name my-game --template default

# Verify
godot-forge project info
```

**What `project init` creates**:
- `project.godot` with config_version=5
- `.gitignore` (ignores .godot/, .import/, export/, .godot-forge/)
- `addons/forge_sync/` (TCP bridge plugin, auto-enabled)
- Standard directories: scenes/, scripts/, assets/

**Quality Gate**: `godot-forge project info` returns valid project data.

**Pitfalls**: Don't manually create project.godot — always use `project init`.

---

## Step 3: Input Mapping / Autoloads / Config 🟢

**Goal**: Configure input actions, global singletons, and project settings.

```bash
# Add input actions
echo '{"name":"move_left","keys":["A","Left"]}' | godot-forge project input add
echo '{"name":"move_right","keys":["D","Right"]}' | godot-forge project input add
echo '{"name":"jump","keys":["Space","W","Up"]}' | godot-forge project input add
echo '{"name":"attack","keys":["J","Z"]}' | godot-forge project input add

# Verify inputs
godot-forge project input list

# Create autoload scripts first
echo '{"name":"game_manager","extends":"Node","methods":["_ready"]}' | godot-forge script create --path scripts/autoload/game_manager.gd

# Register autoloads
echo '{"name":"GameManager","path":"res://scripts/autoload/game_manager.gd"}' | godot-forge project autoload add

# Set project config
godot-forge project config set application/run/main_scene '"res://scenes/main.tscn"'

# Verify
godot-forge project autoload list
godot-forge project config list
```

**Quality Gate**: `project input list` shows all actions, `project autoload list` shows singletons.

**Pitfalls**: Create the script files BEFORE registering autoloads.

---

## Step 4: Digital Asset Generation 🟢

**Goal**: Create or acquire game assets (textures, audio, fonts).

The Agent generates placeholder assets or downloads them. Place files in standard directories:
- `assets/textures/` — PNG/SVG images
- `assets/audio/music/` — Background music (OGG)
- `assets/audio/sfx/` — Sound effects (WAV/OGG)
- `assets/fonts/` — TTF/OTF fonts

No CLI commands for this step (future: `godot-forge generate *` for AI asset generation).

**Quality Gate**: All required assets exist in correct directories.

**Pitfalls**: Use `res://` relative paths when referencing assets. Don't put assets in .godot/.

---

## Step 5: Asset Import & Processing 🟠

**Goal**: Let Godot process raw assets into its internal format.

```bash
# Trigger import (requires Godot)
godot-forge engine import

# Scan UIDs (required before referencing assets in scenes)
godot-forge engine uid

# Check for issues
godot-forge resource check
```

**Quality Gate**: `engine import` succeeds, `engine uid` shows UIDs, `resource check` finds no missing refs.

**Pitfalls**: MUST run import before building scenes that reference assets. Never guess UIDs.

---

## Step 6: Scene Building 🔵

**Goal**: Create the game's scene hierarchy.

```bash
# Create player scene with children
echo '{
  "name": "player",
  "root_type": "CharacterBody2D",
  "children": [
    {"name": "Sprite2D", "type": "Sprite2D"},
    {"name": "CollisionShape2D", "type": "CollisionShape2D"},
    {"name": "AnimationPlayer", "type": "AnimationPlayer"}
  ]
}' | godot-forge scene create

# Create main scene
echo '{"name": "main", "root_type": "Node2D"}' | godot-forge scene create

# Instance player into main scene
echo '{"scene": "main", "name": "Player", "instance": "res://scenes/player.tscn"}' | godot-forge node add

# Add more nodes
echo '{"scene": "main", "name": "Camera2D", "type": "Camera2D", "parent": "."}' | godot-forge node add

# Verify structure
godot-forge scene list
godot-forge scene read player
godot-forge node list --scene main
```

**Quality Gate**: `scene list` shows expected scenes, `scene read` shows correct hierarchy.

**Pitfalls**: Use scene instancing (`instance` field) instead of duplicating node trees.

---

## Step 7: Script Writing 🟢

**Goal**: Implement game logic in GDScript.

```bash
# Generate player script
echo '{
  "name": "player",
  "extends": "CharacterBody2D",
  "path": "scripts/player/player.gd",
  "signals": ["died", "health_changed"],
  "exports": [
    {"name": "speed", "type": "float", "default": 300.0},
    {"name": "jump_velocity", "type": "float", "default": -500.0}
  ],
  "methods": ["_physics_process", "_ready", "take_damage"]
}' | godot-forge script create

# Attach script to scene node
echo '{"scene": "player", "path": ".", "script": "res://scripts/player/player.gd"}' | godot-forge node update

# Edit existing script (add new signals/methods)
echo '{"path": "scripts/player/player.gd", "add_signals": ["landed"], "add_methods": ["respawn"]}' | godot-forge script edit

# Validate script
godot-forge script validate scripts/player/player.gd

# List all scripts
godot-forge script list
```

**Quality Gate**: `script validate` passes for all scripts. Scripts attached to correct nodes.

**Pitfalls**: Use `script create` for scaffolding, then manually edit the function bodies. The CLI generates stubs, not full implementations.

---

## Step 8: Level Design 🔵

**Goal**: Build game levels by composing scenes and placing instances.

```bash
# Create level scene
echo '{"name": "level_01", "root_type": "Node2D", "path": "scenes/levels/level_01.tscn"}' | godot-forge scene create

# Instance existing scenes
echo '{"scene": "levels/level_01", "name": "Player", "instance": "res://scenes/player.tscn"}' | godot-forge node add
echo '{"scene": "levels/level_01", "name": "TileMapLayer", "type": "TileMapLayer"}' | godot-forge node add

# Add environment objects
echo '{"scene": "levels/level_01", "name": "Platforms", "type": "Node2D"}' | godot-forge node add
echo '{"scene": "levels/level_01", "name": "Platform1", "type": "StaticBody2D", "parent": "Platforms"}' | godot-forge node add

# Merge nodes from another scene
echo '{"target": "levels/level_01", "sources": ["scenes/ui/hud.tscn"], "parent": "."}' | godot-forge scene merge

# Set as main scene
godot-forge project config set application/run/main_scene '"res://scenes/levels/level_01.tscn"'
```

**Quality Gate**: Level scene has all instances, `node list` shows correct hierarchy.

**Pitfalls**: Don't duplicate entire scenes manually — use instancing.

---

## Step 9: UI Building 🔵

**Goal**: Create user interface scenes (HUD, menus, dialogs).

```bash
# Create HUD scene
echo '{
  "name": "hud",
  "root_type": "CanvasLayer",
  "path": "scenes/ui/hud.tscn",
  "children": [
    {"name": "HealthBar", "type": "ProgressBar"},
    {"name": "ScoreLabel", "type": "Label"},
    {"name": "PauseButton", "type": "Button"}
  ]
}' | godot-forge scene create

# Create main menu
echo '{
  "name": "main_menu",
  "root_type": "Control",
  "path": "scenes/ui/main_menu.tscn",
  "children": [
    {"name": "VBoxContainer", "type": "VBoxContainer"},
    {"name": "Title", "type": "Label"},
    {"name": "PlayButton", "type": "Button"},
    {"name": "QuitButton", "type": "Button"}
  ]
}' | godot-forge scene create

# Connect button signals
echo '{"scene": "ui/main_menu", "signal": "pressed", "from": "PlayButton", "to": ".", "method": "_on_play_pressed"}' | godot-forge node connect
```

**Quality Gate**: UI scenes have correct node types, signals connected.

**Pitfalls**: Use CanvasLayer as root for HUD (renders on top). Use Control for full-screen menus.

---

## Step 10: Audio Integration 🔵

**Goal**: Add music and sound effects to scenes.

```bash
# Add audio players
echo '{"scene": "main", "name": "BGMusic", "type": "AudioStreamPlayer"}' | godot-forge node add
echo '{"scene": "player", "name": "JumpSFX", "type": "AudioStreamPlayer2D", "parent": "."}' | godot-forge node add
echo '{"scene": "player", "name": "HitSFX", "type": "AudioStreamPlayer2D", "parent": "."}' | godot-forge node add
```

**Quality Gate**: Audio nodes exist in scenes, audio files imported.

**Pitfalls**: Use AudioStreamPlayer2D for positional audio, AudioStreamPlayer for global (music).

---

## Step 11: Animation 🔵

**Goal**: Create animations for sprites, UI, and game objects.

```bash
# Add AnimationPlayer if not present
echo '{"scene": "player", "name": "AnimationPlayer", "type": "AnimationPlayer", "parent": "."}' | godot-forge node add

# Create animation resources
echo '{"type": "Animation", "name": "idle", "path": "resources/animations/player_idle.tres"}' | godot-forge resource create
echo '{"type": "Animation", "name": "run", "path": "resources/animations/player_run.tres"}' | godot-forge resource create
```

**Quality Gate**: AnimationPlayer nodes exist, animation resources created.

**Pitfalls**: Animation track/keyframe editing requires manual .tres editing or Godot editor.

---

## Step 12: Shader / VFX 🔵

**Goal**: Add visual effects using shaders and particles.

```bash
# Create shader material
echo '{"type": "ShaderMaterial", "name": "glow", "path": "resources/shaders/glow.tres"}' | godot-forge resource create

# Add particle nodes
echo '{"scene": "player", "name": "DustParticles", "type": "GPUParticles2D", "parent": "."}' | godot-forge node add
```

**Quality Gate**: Shader resources exist, particle nodes in scenes.

**Pitfalls**: Shader code must be written in .gdshader files, then referenced by ShaderMaterial.

---

## Step 13: Physics / Collision 🔵

**Goal**: Set up collision shapes, layers, and physics bodies.

```bash
# CollisionShape2D nodes auto-create sub_resources
echo '{"scene": "player", "name": "HurtBox", "type": "Area2D", "parent": "."}' | godot-forge node add
echo '{"scene": "player", "name": "HurtBoxShape", "type": "CollisionShape2D", "parent": "HurtBox"}' | godot-forge node add
# ↑ Automatically creates RectangleShape2D sub_resource

# Add to groups for collision detection
echo '{"scene": "player", "path": ".", "groups": ["player"]}' | godot-forge node group add
echo '{"scene": "player", "path": "HurtBox", "groups": ["hurtbox"]}' | godot-forge node group add

# Connect Area2D signals
echo '{"scene": "player", "signal": "body_entered", "from": "HurtBox", "to": ".", "method": "_on_hurtbox_body_entered"}' | godot-forge node connect
```

**Quality Gate**: Collision shapes exist, groups assigned, signals connected.

**Pitfalls**: CollisionShape2D auto-creates sub_resource. Don't manually create shape resources for simple cases.

---

## Step 14: Save System 🟢

**Goal**: Implement game state persistence.

```bash
# Create save manager script
echo '{
  "name": "save_manager",
  "extends": "Node",
  "path": "scripts/autoload/save_manager.gd",
  "methods": ["save_game", "load_game", "_ready"]
}' | godot-forge script create

# Register as autoload
echo '{"name":"SaveManager","path":"res://scripts/autoload/save_manager.gd"}' | godot-forge project autoload add
```

**Quality Gate**: SaveManager autoload registered. Save/load methods stubbed.

**Pitfalls**: Use `user://` path for save files (platform-independent).

---

## Step 15: Internationalization 🟢

**Goal**: Prepare the game for multiple languages.

```bash
# Set locale config
godot-forge project config set internationalization/locale/translations '[]'
```

**Quality Gate**: i18n config set if needed.

**Pitfalls**: Use `tr()` for translatable strings in scripts, not hardcoded text.

---

## Step 16: Testing 🟠

**Goal**: Run the game and verify functionality.

```bash
# Validate project structure
godot-forge engine validate
godot-forge engine validate --deep  # Deep validation via Godot

# Run the game
godot-forge engine run
godot-forge engine run --scene res://scenes/levels/level_01.tscn

# Run headless (CI)
godot-forge engine run --headless --timeout 5000

# Check all references
godot-forge resource check
godot-forge project validate
```

**Quality Gate**: `engine validate --deep` passes, game runs without crashes.

**Pitfalls**: Always validate before running. Fix broken refs first.

---

## Step 17: Performance Profiling 🟠

**Goal**: Identify and fix performance bottlenecks.

```bash
# Take screenshot for visual verification
godot-forge engine preview --scene res://scenes/levels/level_01.tscn --output preview.png

# Run with monitoring
godot-forge engine run --headless --timeout 10000
```

**Quality Gate**: Game runs at target framerate, no visual glitches in preview.

**Pitfalls**: Profile on target hardware/platform.

---

## Step 18: Build Export 🟠

**Goal**: Create distributable game builds.

```bash
# Add export presets
echo '{"name": "Windows", "platform": "Windows Desktop", "path": "export/windows/game.exe"}' | godot-forge export preset add
echo '{"name": "Linux", "platform": "Linux", "path": "export/linux/game.x86_64"}' | godot-forge export preset add
echo '{"name": "macOS", "platform": "macOS", "path": "export/macos/game.dmg"}' | godot-forge export preset add
echo '{"name": "Web", "platform": "Web", "path": "export/web/index.html"}' | godot-forge export preset add

# List presets
godot-forge export preset list

# Build (requires Godot + export templates installed)
godot-forge export build --preset "Windows"
godot-forge export build --preset "Web"
```

**Quality Gate**: `export build` succeeds for target platforms, build artifacts exist.

**Pitfalls**: Export templates must be installed in Godot first. Check `engine validate` before building.

---

## Step 19: Release 🟢

**Goal**: Final checks and distribution.

```bash
# Final validation pass
godot-forge project validate
godot-forge resource check
godot-forge engine validate --deep

# Verify build outputs
ls -la export/
```

**Quality Gate**: All validations pass, build artifacts ready for distribution.

**Pitfalls**: Test builds on clean machines before release.

---

## Quick Pipeline Reference

Minimum viable pipeline (steps that MUST happen):

```bash
# 1. Init
godot-forge project init --name my-game

# 2. Build scenes + scripts (repeat as needed)
echo '{"name":"main","root_type":"Node2D"}' | godot-forge scene create
echo '{"name":"main","extends":"Node2D"}' | godot-forge script create

# 3. Import + validate
godot-forge engine import
godot-forge engine uid
godot-forge engine validate

# 4. Run
godot-forge engine run

# 5. Export
echo '{"name":"Web","platform":"Web","path":"export/web/index.html"}' | godot-forge export preset add
godot-forge export build --preset Web
```
