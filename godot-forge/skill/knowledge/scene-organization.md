# Scene Organization — Godot 4.x Project Structure

> **Purpose**: Standard directory layout and naming conventions for Godot projects
> created with godot-forge. The Agent should follow this structure for all new projects
> and validate it during audits.

---

## Standard Project Structure

```
project_root/
├── project.godot                # Godot project file (DO NOT hand-edit)
├── .gitignore                   # Git ignore rules
├── .godot/                      # Engine cache (NEVER touch, gitignored)
├── addons/
│   └── forge_sync/              # Auto-installed by godot-forge
│
├── scenes/                      # All .tscn files
│   ├── main.tscn                # Entry scene (set in project.godot)
│   ├── player/
│   │   └── player.tscn
│   ├── enemies/
│   │   ├── enemy_base.tscn      # Base enemy (inherited by specific enemies)
│   │   ├── slime.tscn
│   │   ├── bat.tscn
│   │   └── boss_goblin.tscn
│   ├── objects/
│   │   ├── coin.tscn
│   │   ├── door.tscn
│   │   └── chest.tscn
│   ├── ui/
│   │   ├── hud.tscn
│   │   ├── main_menu.tscn
│   │   ├── pause_menu.tscn
│   │   ├── game_over.tscn
│   │   └── settings_menu.tscn
│   ├── levels/
│   │   ├── level_01.tscn
│   │   ├── level_02.tscn
│   │   └── level_boss.tscn
│   └── effects/
│       ├── explosion.tscn
│       └── damage_number.tscn
│
├── scripts/                     # All .gd files
│   ├── player/
│   │   ├── player.gd
│   │   └── player_states/
│   │       ├── idle_state.gd
│   │       ├── run_state.gd
│   │       └── jump_state.gd
│   ├── enemies/
│   │   ├── enemy_base.gd
│   │   ├── slime.gd
│   │   └── boss_goblin.gd
│   ├── objects/
│   │   ├── coin.gd
│   │   └── chest.gd
│   ├── ui/
│   │   ├── hud.gd
│   │   ├── main_menu.gd
│   │   └── health_bar.gd
│   ├── autoload/
│   │   ├── game_manager.gd      # Game state, score, pause
│   │   ├── audio_manager.gd     # Music and SFX
│   │   ├── scene_manager.gd     # Scene transitions
│   │   └── save_manager.gd      # Save/load
│   ├── systems/
│   │   ├── state_machine.gd
│   │   ├── state.gd
│   │   └── damage_system.gd
│   └── resources/
│       ├── weapon_data.gd       # Custom Resource class
│       ├── enemy_stats.gd
│       └── level_config.gd
│
├── assets/                      # Raw creative assets
│   ├── textures/
│   │   ├── player/
│   │   │   ├── player_idle.png
│   │   │   ├── player_run.png
│   │   │   └── player_spritesheet.png
│   │   ├── enemies/
│   │   ├── tiles/
│   │   ├── ui/
│   │   └── effects/
│   ├── audio/
│   │   ├── music/
│   │   │   ├── main_theme.ogg   # Use .ogg for music
│   │   │   └── boss_battle.ogg
│   │   └── sfx/
│   │       ├── jump.wav         # Use .wav for SFX
│   │       ├── coin_pickup.wav
│   │       └── hit.wav
│   ├── fonts/
│   │   └── pixel_font.ttf
│   └── models/                  # 3D projects only
│       ├── player/
│       └── environment/
│
├── resources/                   # Godot resource files (.tres)
│   ├── themes/
│   │   └── default_theme.tres
│   ├── data/
│   │   ├── sword.tres           # WeaponData instance
│   │   ├── bow.tres
│   │   ├── slime_stats.tres     # EnemyStats instance
│   │   └── level_01_config.tres
│   └── materials/               # 3D projects
│       └── terrain.tres
│
├── design/                      # Design documents (godot-forge design output)
│   ├── gdd.md                   # Game Design Document
│   ├── art_guide.md
│   └── narrative.md
│
└── export/                      # Build outputs (gitignored)
    ├── web/
    ├── windows/
    ├── linux/
    └── mac/
```

---

## Naming Conventions

### Files and Directories

| Element | Convention | Example |
|---------|-----------|---------|
| Directories | `snake_case` | `player_states/`, `main_menu/` |
| Scene files | `snake_case.tscn` | `player.tscn`, `boss_goblin.tscn` |
| Script files | `snake_case.gd` | `player.gd`, `game_manager.gd` |
| Resource files | `snake_case.tres` | `sword.tres`, `slime_stats.tres` |
| Textures | `snake_case.png` | `player_idle.png` |
| Audio - Music | `snake_case.ogg` | `main_theme.ogg` |
| Audio - SFX | `snake_case.wav` | `coin_pickup.wav` |

### Nodes in the Scene Tree

| Element | Convention | Example |
|---------|-----------|---------|
| Node names | `PascalCase` | `Player`, `HurtBox`, `AnimationPlayer` |
| Group names | `snake_case` | `enemies`, `collectibles`, `damageable` |
| Signal names | `snake_case` | `health_changed`, `item_collected` |
| Autoload names | `PascalCase` | `GameManager`, `AudioManager` |

### Script Classes

| Element | Convention | Example |
|---------|-----------|---------|
| `class_name` | `PascalCase` | `WeaponData`, `StateMachine` |
| Functions | `snake_case` | `take_damage()`, `apply_knockback()` |
| Variables | `snake_case` | `max_health`, `move_speed` |
| Constants | `UPPER_SNAKE` | `MAX_SPEED`, `GRAVITY` |
| Enums | `PascalCase` members | `enum State { Idle, Running, Jumping }` |
| Private | `_` prefix | `_internal_timer`, `_calculate_damage()` |

---

## When to Split Scenes

### Split into a new scene when:

- ✅ The node subtree is **reused** in multiple places (enemies, UI widgets, bullets)
- ✅ The subtree has its **own script** with significant logic
- ✅ You want to **test it in isolation** (F6)
- ✅ The subtree represents a **complete game object** (player, item, obstacle)
- ✅ Multiple people/agents need to **work on it independently**

### Keep in one scene when:

- ❌ The nodes are **purely structural** (layout containers with no logic)
- ❌ It's only used **once** and has minimal logic
- ❌ Splitting would create a scene with **only 1-2 nodes**
- ❌ The nodes are **tightly coupled** to their parent (e.g., a collision shape)

---

## Scene Naming Patterns

| Pattern | Use Case | Example |
|---------|----------|---------|
| `<name>.tscn` | Standalone entity | `player.tscn`, `coin.tscn` |
| `<name>_base.tscn` | Base for inheritance | `enemy_base.tscn` |
| `level_<number>.tscn` | Sequential levels | `level_01.tscn`, `level_02.tscn` |
| `level_<name>.tscn` | Named levels | `level_forest.tscn`, `level_boss.tscn` |
| `<name>_menu.tscn` | Menu screens | `main_menu.tscn`, `pause_menu.tscn` |
| `<name>_effect.tscn` | Visual effects | `explosion_effect.tscn` |

---

## Script-to-Scene Pairing

### Convention: Mirror the directory structure

```
scenes/player/player.tscn   →  scripts/player/player.gd
scenes/enemies/slime.tscn   →  scripts/enemies/slime.gd
scenes/ui/hud.tscn           →  scripts/ui/hud.gd
```

### Rules

1. **One primary script per scene root** — `player.tscn`'s root has `player.gd`.
2. **Child nodes can have their own scripts** — `HurtBox` in `player.tscn` may have
   `hurt_box.gd`, but only if it has non-trivial logic.
3. **Shared scripts go in `scripts/systems/`** — `state_machine.gd`, `damage_system.gd`.
4. **Resource class scripts go in `scripts/resources/`** — `weapon_data.gd`.
5. **Autoload scripts go in `scripts/autoload/`** — `game_manager.gd`.

---

## godot-forge Commands for Organization

```bash
# Initialize project with standard structure
godot-forge project init my-game --template platformer-2d

# Validate project structure
godot-forge project validate

# List all scenes
godot-forge scene list

# List all scripts
godot-forge script list

# List all resources
godot-forge resource list

# Check for orphaned scripts (scripts with no scene reference)
godot-forge resource check --orphans

# Check project configuration
godot-forge project info
```

---

## .gitignore Template

```gitignore
# Godot 4.x
.godot/

# Build outputs
export/

# OS files
.DS_Store
Thumbs.db

# IDE
*.swp
*.swo
*~

# godot-forge temp files
.forge-tmp/
```
