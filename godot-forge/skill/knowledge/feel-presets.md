# Game Feel Parameter Presets

> **Purpose**: Concrete numeric values for common game genres. When scaffolding a new
> project, the Agent should apply the matching preset as a starting point, then let the
> user tweak from there. These values are for Godot 4.x.

---

## Platformer — Tight (Super Meat Boy, Celeste)

Fast, precise, instant response. The player should feel like the character IS their
thumbstick.

```gdscript
# player.gd — Tight Platformer
const SPEED = 300.0
const JUMP_VELOCITY = -500.0
const GRAVITY = 1200.0
const ACCELERATION = 2000.0
const FRICTION = 2000.0
const AIR_ACCELERATION = 1200.0
const AIR_FRICTION = 200.0
const MAX_FALL_SPEED = 800.0

# Forgiveness mechanics
const COYOTE_TIME = 0.1        # seconds after leaving edge where jump still works
const JUMP_BUFFER = 0.1        # seconds before landing where jump input is remembered
const CORNER_CORRECTION = 4.0  # pixels of corner nudge to prevent head bonks
```

**When to use**: Precision platformers, speedrun-oriented games, action-heavy games.

---

## Platformer — Floaty (Kirby, Little Big Planet)

Slow, forgiving, gentle arcs. Players have more time to react mid-air.

```gdscript
# player.gd — Floaty Platformer
const SPEED = 200.0
const JUMP_VELOCITY = -350.0
const GRAVITY = 600.0
const ACCELERATION = 800.0
const FRICTION = 600.0
const AIR_ACCELERATION = 400.0
const AIR_FRICTION = 100.0
const MAX_FALL_SPEED = 500.0

const COYOTE_TIME = 0.15
const JUMP_BUFFER = 0.15
```

**When to use**: Casual platformers, kids' games, exploration-focused platformers.

---

## Platformer — Heavy (Castlevania, Mega Man)

Committed jumps, weighty feel. Once you jump, you're on a trajectory.

```gdscript
# player.gd — Heavy Platformer
const SPEED = 180.0
const JUMP_VELOCITY = -450.0
const GRAVITY = 1000.0
const ACCELERATION = 1500.0
const FRICTION = 1500.0
const AIR_ACCELERATION = 300.0   # Very limited air control
const AIR_FRICTION = 50.0
const MAX_FALL_SPEED = 900.0

const COYOTE_TIME = 0.06
const JUMP_BUFFER = 0.08
```

**When to use**: Retro-style platformers, deliberate/strategic movement games.

---

## Top-Down — Crisp (Hotline Miami, Nuclear Throne)

Instant stop-and-go. Character is exactly where the stick points.

```gdscript
# player.gd — Crisp Top-Down
const SPEED = 200.0
const ACCELERATION = 3000.0
const FRICTION = 3000.0
const DASH_SPEED = 600.0
const DASH_DURATION = 0.1
const DASH_COOLDOWN = 0.5
```

**When to use**: Twin-stick shooters, fast-paced action, arcade-style games.

---

## Top-Down — Momentum (Zelda-like, RPG)

Gentle acceleration, feels like the character has weight.

```gdscript
# player.gd — Momentum Top-Down
const SPEED = 300.0
const ACCELERATION = 800.0
const FRICTION = 400.0
const SPRINT_SPEED = 450.0
const SPRINT_ACCELERATION = 1200.0
```

**When to use**: Adventure games, RPGs, exploration games, Zelda-likes.

---

## Top-Down — Tank (Survival Horror, Enter the Gungeon)

Deliberate movement, slight drag. Movement itself is a resource.

```gdscript
# player.gd — Tank Top-Down
const SPEED = 150.0
const ACCELERATION = 600.0
const FRICTION = 800.0
const ROTATION_SPEED = 5.0       # For tank-rotation movement
```

**When to use**: Survival horror, tactical shooters, slower-paced games.

---

## Camera Settings

### Platformer Camera

```gdscript
# Camera2D settings
position_smoothing_enabled = true
position_smoothing_speed = 8.0      # Tight: 12.0, Floaty: 5.0
drag_horizontal_enabled = true
drag_vertical_enabled = true
drag_left_margin = 0.15
drag_right_margin = 0.15
drag_top_margin = 0.2
drag_bottom_margin = 0.3           # More room below for falling

# Look-ahead (via script)
const LOOK_AHEAD_DISTANCE = 60.0
const LOOK_AHEAD_SPEED = 3.0
```

### Top-Down Camera

```gdscript
# Camera2D settings
position_smoothing_enabled = true
position_smoothing_speed = 6.0      # Smooth follow
zoom = Vector2(2, 2)                # Adjust to taste
```

### Boss Fight Camera

```gdscript
# Zoom out to show both player and boss
const BOSS_ZOOM = Vector2(1.5, 1.5)
const ZOOM_SPEED = 2.0
# Center between player and boss
const CAMERA_WEIGHT_PLAYER = 0.6    # Favor player position
const CAMERA_WEIGHT_BOSS = 0.4
```

---

## Screen Shake

```gdscript
# screen_shake.gd — Attach to Camera2D or use as utility
var shake_intensity: float = 0.0
var shake_duration: float = 0.0
var shake_decay: float = 5.0

func shake(intensity: float, duration: float) -> void:
    shake_intensity = intensity
    shake_duration = duration

func _process(delta: float) -> void:
    if shake_duration > 0:
        shake_duration -= delta
        offset = Vector2(
            randf_range(-shake_intensity, shake_intensity),
            randf_range(-shake_intensity, shake_intensity)
        )
        shake_intensity = lerp(shake_intensity, 0.0, shake_decay * delta)
    else:
        offset = Vector2.ZERO
```

### Shake Presets

| Event | Intensity | Duration | Notes |
|-------|-----------|----------|-------|
| **Small hit** | 2.0 | 0.1 | Sword strike, small collision |
| **Medium hit** | 5.0 | 0.15 | Heavy attack, explosion nearby |
| **Big hit** | 10.0 | 0.2 | Boss slam, death |
| **Massive** | 15.0 | 0.3 | Screen-filling explosion |
| **Ambient rumble** | 1.0 | 2.0 | Earthquake, engine hum |

---

## Hit Stop / Freeze Frame

Briefly pause the game on impact to sell the weight of a hit.

```gdscript
# game_manager.gd (Autoload)
func hit_stop(duration: float) -> void:
    Engine.time_scale = 0.0
    await get_tree().create_timer(duration, true, false, true).timeout
    Engine.time_scale = 1.0
```

### Hit Stop Presets

| Event | Duration (seconds) | Notes |
|-------|-------------------|-------|
| **Light attack** | 0.03 | Barely perceptible, adds crunch |
| **Heavy attack** | 0.06 | Noticeable pause |
| **Critical hit** | 0.1 | Dramatic pause |
| **Kill shot** | 0.12 | Satisfying final blow |
| **Parry/Counter** | 0.08 | Reward timing |

---

## Knockback

```gdscript
func apply_knockback(direction: Vector2, force: float) -> void:
    velocity = direction.normalized() * force
```

### Knockback Presets

| Event | Force | Notes |
|-------|-------|-------|
| **Small push** | 150.0 | Bumping into something |
| **Hit** | 300.0 | Normal enemy attack |
| **Heavy hit** | 500.0 | Boss attack |
| **Explosion** | 700.0 | Bomb, area attack |
| **Launch** | 1000.0 | Intentional launch mechanic |

---

## Invincibility Frames (i-frames)

```gdscript
var invincible: bool = false

func start_iframes(duration: float) -> void:
    invincible = true
    # Visual feedback: flash the sprite
    var tween = create_tween()
    for i in int(duration / 0.1):
        tween.tween_property($Sprite2D, "modulate:a", 0.3, 0.05)
        tween.tween_property($Sprite2D, "modulate:a", 1.0, 0.05)
    await get_tree().create_timer(duration).timeout
    invincible = false
    $Sprite2D.modulate.a = 1.0
```

### i-frame Presets

| Context | Duration | Notes |
|---------|----------|-------|
| **After hit** | 1.0 - 1.5 | Standard damage immunity |
| **Dodge/roll** | 0.3 - 0.5 | Active frames during dodge animation |
| **Respawn** | 2.0 - 3.0 | Protection after respawning |
| **Power-up** | 5.0 - 10.0 | Star power / invincibility item |

---

## Animation Speed Multipliers

| State | Speed Scale | Notes |
|-------|------------|-------|
| **Idle** | 1.0 | Normal breathing/bobbing |
| **Walk** | 1.0 | Match footstep timing to movement |
| **Run** | 1.2 - 1.5 | Faster cycle, match to SPEED constant |
| **Attack (light)** | 1.5 | Snappy, quick |
| **Attack (heavy)** | 0.8 | Slow windup for weight |
| **Hurt** | 2.0 | Quick flinch |
| **Death** | 0.7 | Slow, dramatic fall |

---

## Combining Feel Elements

For maximum juice, layer multiple feel elements on a single event:

### Example: Enemy Death

```
1. Hit stop       → 0.08s freeze
2. Screen shake   → intensity 5.0, duration 0.15
3. Knockback      → force 400.0 away from player
4. Particles      → burst of 12 particles
5. Sound          → death sfx
6. Flash          → white flash for 1 frame
7. Slow-mo        → 0.5x time scale for 0.2s (optional, for last enemy)
```

### Example: Player Jump

```
1. Squash         → scale.y = 0.7 for 2 frames before jump
2. Stretch        → scale.y = 1.3 for 3 frames during launch
3. Particles      → dust puff at feet
4. Sound          → jump sfx
5. Trail          → afterimage for fast characters (optional)
```

### Example: Coin Pickup

```
1. Sound          → bright ding sfx
2. Particles      → sparkle burst
3. UI feedback    → counter increments with bounce animation
4. Screen shake   → intensity 1.0, duration 0.05 (subtle)
5. Coin anim      → coin flies toward UI counter (tween)
```

---

## How the Agent Should Use These Presets

1. **During `godot-forge project init`**: Apply the matching genre preset to the
   generated player script constants.
2. **When user says "it feels floaty/stiff/heavy"**: Adjust toward the named preset.
   Floaty complaint → increase ACCELERATION/FRICTION. Heavy complaint → decrease
   GRAVITY, increase AIR_ACCELERATION.
3. **Always include forgiveness mechanics**: Coyote time and jump buffer should be in
   every platformer, even if the user doesn't ask.
4. **Layer feel elements**: A hit without screen shake, hit stop, and knockback will
   feel flat. Always suggest the full stack.
5. **Values are starting points**: After applying a preset, recommend the user playtest
   and tweak ±20% to find their sweet spot.
