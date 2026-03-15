# Godot 4.x Design Patterns & Best Practices

> **Purpose**: Canonical patterns for structuring Godot projects. The Agent should
> default to these patterns unless the user explicitly requests something different.

---

## 1. Scene Composition

**Everything in Godot is a scene.** Scenes are reusable, nestable, and self-contained.

### Principle: Composition Over Inheritance

Instead of deep class hierarchies, compose behavior by combining nodes:

```
Player (CharacterBody2D)
├── Sprite2D
├── CollisionShape2D
├── AnimationPlayer
├── HurtBox (Area2D)
│   └── CollisionShape2D
├── HitBox (Area2D)
│   └── CollisionShape2D
├── StateMachine (Node)
│   ├── IdleState
│   ├── RunState
│   ├── JumpState
│   └── AttackState
├── Camera2D
└── AudioStreamPlayer2D
```

### Rules

- Each scene should represent **one game object** (player, enemy, bullet, UI panel).
- A scene should be testable in isolation — press F6 to run just that scene.
- If a node group is reused across scenes, extract it into its own scene.

### godot-forge Commands

```bash
# Create a composed player scene
godot-forge scene create player --type CharacterBody2D
godot-forge node add player Sprite2D --name Sprite
godot-forge node add player CollisionShape2D --name Collision
godot-forge node add player AnimationPlayer --name Anim
godot-forge node add player Area2D --name HurtBox
godot-forge node add player/HurtBox CollisionShape2D --name HurtShape
```

---

## 2. Signal Pattern

Signals are Godot's observer pattern — decoupled, type-safe event communication.

### The Golden Rule

```
Signals go UP  (child → parent)     ← emit signal
Calls go DOWN  (parent → child)     ← call method directly
```

A child should **never** know about its parent's API. A parent **can** know about its
children's API (it instantiated them).

### Common Signal Definitions

```gdscript
# Player signals
signal died
signal health_changed(new_health: int)
signal stamina_changed(new_stamina: float)

# Item signals
signal collected(item_type: String, value: int)

# Level signals
signal level_completed
signal checkpoint_reached(checkpoint_id: int)

# UI signals (from UI to game logic)
signal start_pressed
signal option_changed(key: String, value: Variant)
```

### Connection Patterns

```gdscript
# In parent's _ready() — connect child signal to parent method
$Player.died.connect(_on_player_died)
$Player.health_changed.connect(_on_player_health_changed)

# Or use the editor's connection panel (generates _on_NodeName_signal_name)
```

### godot-forge Commands

```bash
# Add a signal to a script
godot-forge script edit player.gd --add-signal "health_changed(new_health: int)"

# Validate signal connections
godot-forge engine validate
```

---

## 3. State Machine Pattern

Use state machines for anything with distinct behavioral modes (player, enemies, AI, UI).

### GDScript Implementation

```gdscript
# state_machine.gd — Attach to a Node called StateMachine
class_name StateMachine
extends Node

@export var initial_state: State

var current_state: State
var states: Dictionary = {}

func _ready() -> void:
    for child in get_children():
        if child is State:
            states[child.name.to_lower()] = child
            child.transitioned.connect(_on_child_transitioned)
    if initial_state:
        initial_state.enter()
        current_state = initial_state

func _process(delta: float) -> void:
    if current_state:
        current_state.update(delta)

func _physics_process(delta: float) -> void:
    if current_state:
        current_state.physics_update(delta)

func _on_child_transitioned(state: State, new_state_name: String) -> void:
    if state != current_state:
        return
    var new_state = states.get(new_state_name.to_lower())
    if not new_state:
        return
    current_state.exit()
    new_state.enter()
    current_state = new_state
```

```gdscript
# state.gd — Base class for all states
class_name State
extends Node

signal transitioned(state: State, new_state_name: String)

func enter() -> void:
    pass

func exit() -> void:
    pass

func update(_delta: float) -> void:
    pass

func physics_update(_delta: float) -> void:
    pass
```

### Common Player States

| State | Enter | Update | Exit |
|-------|-------|--------|------|
| **Idle** | Stop animation | Check input → Run/Jump | — |
| **Run** | Play run anim | Apply velocity; no input → Idle | — |
| **Jump** | Apply jump velocity, play jump anim | Apply gravity; landed → Idle/Run | — |
| **Fall** | Play fall anim | Apply gravity; landed → Idle/Run | — |
| **Attack** | Play attack anim, enable hitbox | Wait for anim end → Idle | Disable hitbox |
| **Hurt** | Play hurt anim, flash sprite | Timer → Idle or Dead | Reset sprite |
| **Dead** | Play death anim, emit `died` | — | — |

### godot-forge Commands

```bash
# Generate state machine scaffold
godot-forge script create state_machine.gd --class StateMachine
godot-forge script create state.gd --class State

# Add state nodes
godot-forge node add player/StateMachine Node --name IdleState
godot-forge node add player/StateMachine Node --name RunState
godot-forge node add player/StateMachine Node --name JumpState
```

---

## 4. Autoload Singletons

Globally accessible managers registered in Project Settings.

### Standard Manager Set

| Singleton | Responsibility | Example API |
|-----------|---------------|-------------|
| **GameManager** | Game state, score, lives, pause | `GameManager.add_score(100)`, `GameManager.pause()` |
| **AudioManager** | Play music/SFX, volume control | `AudioManager.play_sfx("jump")`, `AudioManager.set_music("level1")` |
| **SceneManager** | Scene transitions, loading screens | `SceneManager.change_scene("res://scenes/levels/level_02.tscn")` |
| **SaveManager** | Save/load game data | `SaveManager.save_game()`, `SaveManager.load_game()` |
| **EventBus** | Global signal hub (use sparingly) | `EventBus.player_died.emit()` |

### When to Use Autoloads

✅ **Use** for:
- Truly global state (score, settings, current level)
- Cross-scene persistence (music that plays across scene changes)
- Global event bus (when signal UP/DOWN isn't possible)

❌ **Avoid** for:
- Data that belongs to a single scene
- Logic that only one scene cares about
- Anything that could be a regular node in the tree

### godot-forge Commands

```bash
# Create and register an autoload
godot-forge script create autoload/game_manager.gd --autoload GameManager
godot-forge script create autoload/audio_manager.gd --autoload AudioManager
```

---

## 5. Resource Pattern

Custom Resources are data containers — editable in the Inspector, saveable as `.tres`.

### Define a Custom Resource

```gdscript
# weapon_data.gd
class_name WeaponData
extends Resource

@export var name: String = ""
@export var damage: int = 10
@export var attack_speed: float = 1.0
@export var range: float = 50.0
@export var sprite: Texture2D
@export var sound: AudioStream
```

### Use in Scripts

```gdscript
# player.gd
@export var weapon: WeaponData

func attack() -> void:
    deal_damage(weapon.damage)
    play_sound(weapon.sound)
```

### Create Data Files

```
resources/data/
├── sword.tres      # WeaponData: damage=15, speed=1.2
├── bow.tres        # WeaponData: damage=8, speed=0.8, range=200
└── staff.tres      # WeaponData: damage=25, speed=0.5, range=150
```

### Common Resource Types

| Resource | Properties |
|----------|-----------|
| **WeaponData** | name, damage, speed, range, sprite, sound |
| **EnemyStats** | health, speed, damage, xp_reward, drop_table |
| **LevelConfig** | difficulty, enemy_count, time_limit, music |
| **DialogueLine** | speaker, text, portrait, choices[] |
| **LootTable** | entries[{item, weight, min_count, max_count}] |

### godot-forge Commands

```bash
# Create resource class and data file
godot-forge script create resources/weapon_data.gd --class WeaponData --extends Resource
godot-forge resource create data/sword.tres --type WeaponData

# List all resources of a type
godot-forge resource list --type WeaponData
```

---

## 6. Node Communication Hierarchy

The definitive guide to "how should Node A talk to Node B?"

```
┌─────────────────────────────────────────────────┐
│  RELATIONSHIP          METHOD                   │
├─────────────────────────────────────────────────┤
│  Parent → Child        Direct call              │
│                        $Child.method()           │
│                                                  │
│  Child → Parent        Signal                   │
│                        signal something          │
│                        emit something.emit()     │
│                                                  │
│  Sibling → Sibling     Via shared parent         │
│                        Parent connects signals   │
│                        between children           │
│                                                  │
│  Unrelated nodes       Signal bus (Autoload)     │
│                        EventBus.event.emit()     │
│                                                  │
│  Any → Any (data)      Resource (shared ref)     │
│                        Both read same .tres      │
└─────────────────────────────────────────────────┘
```

### Decision Flowchart

1. Is it a parent calling a child? → **Direct call**.
2. Is it a child notifying a parent? → **Signal**.
3. Are they siblings? → **Parent mediates** (connects signals to calls).
4. Are they in different scene trees? → **Autoload signal bus**.
5. Do they share data but not events? → **Shared Resource**.

---

## 7. Group Pattern

Groups are tags. Any node can belong to any number of groups.

### Common Groups

| Group | Nodes | Use Case |
|-------|-------|----------|
| `enemies` | All enemy instances | Damage all enemies, count remaining |
| `collectibles` | Coins, gems, power-ups | Reset on death, count total |
| `interactables` | Doors, levers, NPCs | Highlight nearest, show prompt |
| `persistent` | Nodes that survive scene change | Don't free on transition |
| `damageable` | Anything that can take damage | Generic damage system |

### Usage

```gdscript
# Add to group (usually in _ready or editor)
add_to_group("enemies")

# Batch operations
for enemy in get_tree().get_nodes_in_group("enemies"):
    enemy.take_damage(50)

# Count
var remaining = get_tree().get_nodes_in_group("enemies").size()

# Call method on all group members
get_tree().call_group("enemies", "freeze")
```

### godot-forge Commands

```bash
# Add a node to a group
godot-forge node update enemies/slime --add-group enemies
godot-forge node update enemies/slime --add-group damageable
```

---

## 8. Owner vs Parent

Two different hierarchies that serve different purposes.

| Concept | What It Is | Set By |
|---------|-----------|--------|
| **Parent** | The node directly above in the tree | `add_child()` or editor tree |
| **Owner** | The root of the scene this node was saved with | Scene system (automatic) |

### Why It Matters

- When you **save a scene**, only nodes whose `owner` equals the scene root are saved.
- Nodes added at runtime via `add_child()` have **no owner** by default — they won't
  be saved with the scene.
- If you add a node at runtime and want it to persist in a saved scene:
  ```gdscript
  var bullet = bullet_scene.instantiate()
  add_child(bullet)
  bullet.owner = get_tree().edited_scene_root  # Only in tool scripts
  ```

### Practical Rules

- Don't manually set `owner` in game code — the scene system handles it.
- If nodes are "disappearing" from scenes, check if their `owner` is correct.
- Runtime-spawned nodes (bullets, particles, effects) intentionally have no owner — they
  shouldn't be saved.
