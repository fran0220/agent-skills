# Anti-Patterns in Godot Game Development

> **Purpose**: Common mistakes that AI Agents must avoid when generating Godot projects.
> Each entry explains the bad pattern, why it's bad, the correct approach, and the
> godot-forge commands that enforce correctness.

---

## 1. Guessing UIDs

### ❌ Bad Pattern
```
[ext_resource type="Texture2D" uid="uid://abc123fake" path="res://assets/player.png" id="1_abc"]
```
Fabricating `uid://` strings. They look plausible but Godot will reject them.

### ✅ Correct Approach
Let the Godot engine assign UIDs. Never invent them.

```bash
# Let Godot assign the correct UID
godot-forge engine uid res://assets/player.png
```

### Why It's Bad
UIDs are engine-generated unique identifiers stored in `.godot/uid_cache`. A fabricated
UID will cause import failures, broken references, and cryptic errors at runtime.

---

## 2. Editing the .godot/ Directory

### ❌ Bad Pattern
```bash
# "Fixing" an import by editing the cache
vim .godot/imported/player.png-abc123.ctex
```

### ✅ Correct Approach
Never touch `.godot/`. It's auto-generated engine cache.

```bash
# Re-import to regenerate cache
godot-forge engine import

# Or delete .godot/ entirely — Godot will rebuild it
rm -rf .godot/
godot-forge engine import
```

### Why It's Bad
The `.godot/` directory is Godot's internal cache. Edits will be overwritten on next
import, or worse, will corrupt the cache and cause silent rendering/loading failures.

---

## 3. Hardcoding Filesystem Paths

### ❌ Bad Pattern
```gdscript
var texture = load("/home/user/projects/my_game/assets/player.png")
var config = FileAccess.open("C:\\Games\\my_game\\save.dat", FileAccess.READ)
```

### ✅ Correct Approach
Always use `res://` for project resources and `user://` for user data.

```gdscript
var texture = load("res://assets/textures/player.png")
var config = FileAccess.open("user://save.dat", FileAccess.READ)
```

### Why It's Bad
Absolute paths break on every other machine, every other OS, and every export. `res://`
maps to the project root; `user://` maps to the OS-appropriate user data directory.

---

## 4. God Scripts

### ❌ Bad Pattern
```gdscript
# player.gd — 2000 lines covering movement, combat, inventory, UI, audio, saving...
func _process(delta):
    handle_movement(delta)
    handle_combat(delta)
    handle_inventory()
    update_ui()
    play_footstep_sounds()
    check_save_trigger()
    manage_particle_effects()
    # ... 50 more functions
```

### ✅ Correct Approach
Split into focused scripts using scene composition:

```
Player (CharacterBody2D)     → player.gd (movement only)
├── StateMachine             → state_machine.gd
├── CombatHandler (Node)     → combat_handler.gd
├── Inventory (Node)         → inventory.gd
└── AudioHandler (Node)      → audio_handler.gd
```

```bash
# Create focused scripts
godot-forge script create player/player.gd
godot-forge script create player/combat_handler.gd
godot-forge script create player/inventory.gd
godot-forge node add player Node --name CombatHandler
godot-forge node add player Node --name Inventory
```

### Why It's Bad
God scripts are impossible to debug, test, or reuse. When one system breaks, you're
reading through thousands of unrelated lines. They also cause merge conflicts in teams.

---

## 5. Signal Spaghetti

### ❌ Bad Pattern
```gdscript
# enemy.gd — reaching across the tree to connect to random nodes
func _ready():
    get_node("../../UI/HUD/HealthBar").value_changed.connect(_on_player_health)
    get_node("/root/Main/Player").died.connect(_on_player_died)
    get_tree().get_nodes_in_group("doors")[0].opened.connect(_on_door_opened)
```

### ✅ Correct Approach
Follow the signal hierarchy:
- **Signals go UP** (child → parent via `signal.emit()`)
- **Calls go DOWN** (parent → child via `$Child.method()`)
- **Unrelated nodes** → use an Autoload EventBus

```gdscript
# level.gd (parent) — wires children together
func _ready():
    $Player.died.connect(_on_player_died)
    $Enemies/Slime.health_changed.connect($UI/HUD.update_enemy_health)
```

### Why It's Bad
Hardcoded paths (`../../UI/HUD`) break when you move any node. Cross-tree signal
connections create invisible dependencies that are impossible to trace or refactor.

---

## 6. Ignoring the Import Pipeline

### ❌ Bad Pattern
```bash
# Just drop a PNG into the project and reference it
cp ~/Downloads/sprite.png assets/textures/
# Then immediately try to use it in a scene
```

### ✅ Correct Approach
Always run the import pipeline after adding assets:

```bash
# Add asset, then import
cp ~/Downloads/sprite.png assets/textures/
godot-forge engine import

# Or use resource import which handles this
godot-forge resource import assets/textures/sprite.png
```

### Why It's Bad
Godot needs to process raw assets into its internal format (`.ctex`, `.sample`, etc.)
and register their UIDs. Without importing, the engine won't see the file, and any
`load()` call referencing it will return `null`.

---

## 7. Manual ext_resource IDs

### ❌ Bad Pattern
```
# Hand-writing .tscn content
[ext_resource type="Script" path="res://scripts/player.gd" id="1_abc"]
[ext_resource type="Texture2D" path="res://assets/player.png" id="2_def"]

[sub_resource type="RectangleShape2D" id="RectangleShape2D_xyz"]
shape = ...
```
Manually crafting ext_resource and sub_resource IDs.

### ✅ Correct Approach
Use godot-forge to manage scene files. Its parser handles IDs correctly.

```bash
# Create scene and add resources through CLI
godot-forge scene create player --type CharacterBody2D
godot-forge node add player Sprite2D --texture res://assets/player.png

# Validate the result
godot-forge engine validate
```

### Why It's Bad
Godot 4.x uses a specific ID format (`"TypeName_hash"` for sub_resources, and
`"N_hash"` for ext_resources). Wrong IDs cause parse failures. Duplicate IDs cause
silent overwrites. Let the tooling manage this.

---

## 8. Skipping Validation

### ❌ Bad Pattern
```bash
# Make changes and immediately run
godot-forge scene create enemy
godot-forge node add enemy Sprite2D
godot-forge engine run    # Crosses fingers
```

### ✅ Correct Approach
Always validate after changes:

```bash
godot-forge scene create enemy
godot-forge node add enemy Sprite2D
godot-forge engine import     # Import any new assets
godot-forge engine validate   # Check for errors
godot-forge engine run        # Now run with confidence
```

### Why It's Bad
Errors compound silently. A missing collision shape won't crash but will cause invisible
bugs (enemies you can walk through). Validation catches these before they become
hour-long debugging sessions.

---

## 9. Deep Node Hierarchies

### ❌ Bad Pattern
```
Main
└── World
    └── Environments
        └── Forest
            └── Section1
                └── EnemySpawner
                    └── SpawnPoint
                        └── Enemy
                            └── Visual
                                └── Sprite2D   # 9 levels deep!
```

### ✅ Correct Approach
Keep hierarchies to 3–4 levels. Flatten by using scenes.

```
Main
├── World
│   ├── Level (instanced scene)
│   ├── Player (instanced scene)
│   └── Enemies (Node — container)
│       ├── Slime (instanced scene)
│       └── Bat (instanced scene)
└── UI
    └── HUD (instanced scene)
```

### Why It's Bad
Deep hierarchies make node paths fragile, increase the cognitive load of understanding
the tree, and make `get_node()` calls long and breakable. Instanced scenes keep each
sub-tree shallow and self-contained.

---

## 10. _process vs _physics_process Confusion

### ❌ Bad Pattern
```gdscript
func _process(delta):
    # Movement in _process — frame-rate dependent!
    velocity.x = Input.get_axis("left", "right") * SPEED
    move_and_slide()
```

### ✅ Correct Approach
```gdscript
func _physics_process(delta):
    # Movement and physics in _physics_process — fixed timestep
    velocity.x = Input.get_axis("left", "right") * SPEED
    move_and_slide()

func _process(delta):
    # Visuals, UI updates, non-physics animations
    update_sprite_direction()
    update_particle_effects()
```

### Why It's Bad
`_process` runs every render frame (variable rate). `_physics_process` runs at a fixed
rate (default 60 Hz). Movement and collision in `_process` will behave differently at
30 fps vs 144 fps — characters will move faster or slower depending on frame rate.

**Rule**: Physics/movement/collision → `_physics_process`. Visuals/UI/audio → `_process`.

---

## 11. Not Using Groups

### ❌ Bad Pattern
```gdscript
# Manually iterating children to find enemies
for child in get_children():
    if child.has_method("take_damage"):
        child.take_damage(50)

# Or worse, hardcoding node paths
$Enemies/Slime1.take_damage(50)
$Enemies/Slime2.take_damage(50)
$Enemies/Bat1.take_damage(50)
```

### ✅ Correct Approach
```gdscript
# Tag nodes with groups
# In each enemy's _ready(): add_to_group("enemies")

# Batch operations
for enemy in get_tree().get_nodes_in_group("enemies"):
    enemy.take_damage(50)

# Or even simpler
get_tree().call_group("enemies", "take_damage", 50)
```

```bash
# Add groups via CLI
godot-forge node update enemies/slime --add-group enemies
godot-forge node update enemies/slime --add-group damageable
```

### Why It's Bad
Manual iteration is fragile (misses dynamically spawned enemies), verbose, and breaks
when you reorganize the tree. Groups are dynamic, order-independent, and work across
scenes.

---

## 12. Circular Scene Dependencies

### ❌ Bad Pattern
```
player.tscn instances weapon.tscn
weapon.tscn instances projectile.tscn
projectile.tscn instances player_effect.tscn
player_effect.tscn instances player.tscn   # CIRCULAR!
```

### ✅ Correct Approach
Break circular dependencies with signals or runtime instantiation:

```gdscript
# Instead of instancing player.tscn from player_effect.tscn,
# emit a signal and let the parent handle it
signal effect_completed

# Or use preload only where needed and spawn at runtime
var ProjectileScene = preload("res://scenes/objects/projectile.tscn")
func shoot():
    var bullet = ProjectileScene.instantiate()
    get_parent().add_child(bullet)  # Add to parent, not self
```

### Why It's Bad
Circular scene dependencies cause infinite recursion during loading. Godot will either
crash or produce cryptic "Can't load scene" errors. The dependency graph must be a DAG
(directed acyclic graph).

---

## 13. Forgetting --force for Destructive Operations

### ❌ Bad Pattern
```bash
# Trying to delete without --force
godot-forge scene delete player
# Error: Destructive operation requires --force flag
```

### ✅ Correct Approach
```bash
# Check references first
godot-forge resource check res://scenes/player/player.tscn

# Then delete with explicit confirmation
godot-forge scene delete player --force
```

### Why It's Bad
The `--force` flag exists to prevent accidental data loss. It's a deliberate friction
point. The Agent should always check references before deleting and include `--force`
only after confirming the operation is safe.

---

## 14. Not Checking Resource References Before Delete

### ❌ Bad Pattern
```bash
# Just delete a texture without checking what uses it
godot-forge resource delete assets/textures/player.png --force
# Now 12 scenes have broken texture references
```

### ✅ Correct Approach
```bash
# First, check what references this resource
godot-forge resource check res://assets/textures/player.png

# Output shows:
# Referenced by:
#   - res://scenes/player/player.tscn (Sprite2D.texture)
#   - res://scenes/ui/main_menu.tscn (TextureRect.texture)

# Update or remove references first, then delete
godot-forge resource delete assets/textures/player.png --force
```

### Why It's Bad
Broken resource references cause pink/magenta placeholder textures, missing sounds,
and `null` errors at runtime. Godot won't crash, but the game will look/sound broken.
Always audit dependencies before removing anything.

---

## 15. Mixing 2D and 3D Nodes Incorrectly

### ❌ Bad Pattern
```
Node3D (root)
├── MeshInstance3D
├── Sprite2D          # ← Won't render! 2D node under 3D tree
└── Control           # ← Won't work properly here either
```

### ✅ Correct Approach
```
Node (root)
├── SubViewportContainer    # If you need 2D inside 3D
│   └── SubViewport
│       └── Sprite2D
├── Node3D
│   └── MeshInstance3D
└── CanvasLayer             # UI always goes in CanvasLayer
    └── Control
        └── HUD
```

For pure 2D or pure 3D projects:
```
# 2D Project
Node2D (root)
├── TileMapLayer
├── Player (CharacterBody2D)
└── CanvasLayer
    └── HUD

# 3D Project
Node3D (root)
├── WorldEnvironment
├── DirectionalLight3D
├── Player (CharacterBody3D)
└── CanvasLayer
    └── HUD
```

### Why It's Bad
2D and 3D nodes use completely different rendering pipelines. A `Sprite2D` under a
`Node3D` won't appear in the viewport. A `Control` node outside a `CanvasLayer` in a
3D scene won't receive input correctly. UI should always be in a `CanvasLayer` to render
on top of the game regardless of 2D/3D.

---

## Quick Reference: Validation Workflow

After any change, run this sequence to catch anti-patterns:

```bash
# 1. Import any new assets
godot-forge engine import

# 2. Assign/verify UIDs
godot-forge engine uid

# 3. Validate project integrity
godot-forge engine validate

# 4. Check specific resources if needed
godot-forge resource check

# 5. Run and test
godot-forge engine run
```

If validation fails, fix the reported issues before proceeding. Never skip validation
to save time — it costs more time later.
