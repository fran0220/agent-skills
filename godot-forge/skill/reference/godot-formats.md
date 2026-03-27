# Godot 4.x Text File Formats

Reference for the text-based file formats that godot-forge's L2 layer parses and generates.

> **Scope**: Godot 4.x only (format version 3). Godot 3.x formats differ significantly.

---

## .tscn — Text Scene Format

Scenes are the fundamental building block of Godot projects. A `.tscn` file describes a node tree with its resources, properties, and signal connections.

### Structure

A `.tscn` file has four ordered sections:

```
1. Header          [gd_scene ...]
2. ext_resource     External file references (0 or more)
3. sub_resource     Inline resource definitions (0 or more)
4. node             Node tree (1 or more)
5. connection       Signal connections (0 or more)
```

### Full Example

```ini
[gd_scene load_steps=4 format=3 uid="uid://c1a2b3c4d5e6f"]

[ext_resource type="Script" path="res://scripts/player.gd" id="1"]
[ext_resource type="PackedScene" path="res://scenes/enemy.tscn" id="2"]
[ext_resource type="Texture2D" uid="uid://d7e8f9a0b1c2" path="res://assets/player.png" id="3"]

[sub_resource type="RectangleShape2D" id="RectangleShape2D_abc"]
size = Vector2(16, 16)

[node name="Root" type="Node2D"]

[node name="Player" type="CharacterBody2D" parent="."]
script = ExtResource("1")

[node name="Sprite" type="Sprite2D" parent="Player"]
texture = ExtResource("3")

[node name="Collision" type="CollisionShape2D" parent="Player"]
shape = SubResource("RectangleShape2D_abc")

[node name="Enemy1" parent="." instance=ExtResource("2")]

[connection signal="body_entered" from="Player" to="." method="_on_player_body_entered"]
```

### Header

```ini
[gd_scene load_steps=N format=3 uid="uid://..."]
```

| Field | Description |
|---|---|
| `load_steps` | `ext_resource count` + `sub_resource count` + `1` |
| `format` | Always `3` for Godot 4.x |
| `uid` | Optional. Engine-assigned unique ID for the scene resource itself |

⚠️ **`load_steps` must be accurate.** Godot uses it to allocate loading. If wrong, the scene may fail to load or produce warnings.

### ext_resource — External File References

```ini
[ext_resource type="Script" path="res://scripts/player.gd" id="1"]
[ext_resource type="Texture2D" uid="uid://d7e8f9a0b1c2" path="res://assets/player.png" id="3"]
```

| Attribute | Required | Description |
|---|---|---|
| `type` | Yes | Godot class name (`Script`, `PackedScene`, `Texture2D`, etc.) |
| `path` | Yes | `res://` path to the external file |
| `id` | Yes | Unique string ID within this file, referenced via `ExtResource("id")` |
| `uid` | No | Engine-assigned UID. Present if the referenced resource has one |

Rules:
- IDs must be unique within the file (can be numeric strings like `"1"` or generated like `"1_abc"`)
- Every `ExtResource("N")` reference in the file must have a matching `ext_resource` with that ID
- Unused `ext_resource` entries are harmless but wasteful
- Order does not matter, but conventionally sorted by ID

### sub_resource — Inline Resources

```ini
[sub_resource type="RectangleShape2D" id="RectangleShape2D_abc"]
size = Vector2(16, 16)
```

| Attribute | Required | Description |
|---|---|---|
| `type` | Yes | Godot class name |
| `id` | Yes | Unique string ID, referenced via `SubResource("id")` |

Rules:
- Properties follow on subsequent lines (until the next section header)
- IDs are typically `TypeName_randomsuffix` in Godot 4.x (e.g., `"RectangleShape2D_abc"`)
- Can reference other sub_resources or ext_resources in their properties

### node — Node Tree

```ini
[node name="Root" type="Node2D"]

[node name="Player" type="CharacterBody2D" parent="."]
script = ExtResource("1")
position = Vector2(100, 200)

[node name="Enemy1" parent="." instance=ExtResource("2")]
```

| Attribute | Required | Description |
|---|---|---|
| `name` | Yes | Node name (unique among siblings) |
| `type` | Yes* | Godot class name. *Omitted when using `instance` |
| `parent` | No* | Path relative to root. *Omitted only for the root node |
| `instance` | No | `ExtResource("id")` pointing to a PackedScene — instantiates that scene |

**Parent path rules:**

| Node position | `parent` value |
|---|---|
| Root node | (no `parent` attribute) |
| Direct child of root | `parent="."` |
| Child of "Player" | `parent="Player"` |
| Child of "Player/Sprite" | `parent="Player/Sprite"` |

⚠️ **Root node must be first** and must not have a `parent` attribute.

⚠️ **Scene instancing**: When `instance=ExtResource("N")` is used, do NOT set `type`. The type comes from the instanced scene. You can still override properties on the instance.

**Script attachment:**
```ini
script = ExtResource("1")
```
This is a property on the node, not an attribute of the `[node]` header.

### connection — Signal Connections

```ini
[connection signal="body_entered" from="Player" to="." method="_on_player_body_entered"]
[connection signal="timeout" from="Timer" to="." method="_on_timer_timeout" flags=3]
```

| Attribute | Required | Description |
|---|---|---|
| `signal` | Yes | Signal name |
| `from` | Yes | Path to emitting node (relative to root) |
| `to` | Yes | Path to receiving node (relative to root) |
| `method` | Yes | Method name on the receiving node |
| `flags` | No | Connection flags (default: `0`) |
| `binds` | No | Bound arguments array |

Rules:
- Connections must come after all nodes
- `from` and `to` use the same path format as `parent` (`.` = root node)

---

## .tres — Text Resource Format

Standalone resource files. Used for shapes, materials, animations, themes, custom resources, etc.

### Simple Resource (no sub-resources)

```ini
[gd_resource type="RectangleShape2D" format=3 uid="uid://a1b2c3d4e5f6"]

[resource]
size = Vector2(32, 32)
```

### Resource with Sub-resources

```ini
[gd_resource type="Animation" load_steps=2 format=3]

[sub_resource type="AnimationLibrary" id="AnimationLibrary_abc"]

[resource]
libraries = {
"": SubResource("AnimationLibrary_abc")
}
```

### Structure

```
1. Header          [gd_resource ...]
2. ext_resource     (0 or more)
3. sub_resource     (0 or more)
4. [resource]       The main resource and its properties
```

The `[resource]` section is always last and defines the top-level resource's properties.

| Header field | Description |
|---|---|
| `type` | The Godot class of the main resource |
| `format` | Always `3` for Godot 4.x |
| `load_steps` | Required if there are ext/sub resources. Same formula as .tscn |
| `uid` | Optional engine-assigned UID |

---

## project.godot — Project Configuration

INI-like format. Godot reads this to identify a project and load its settings.

### Example

```ini
config_version=5

[application]
config/name="My Game"
config/features=PackedStringArray("4.4")
run/main_scene="res://scenes/main.tscn"

[input]
move_left={
"deadzone": 0.2,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":0,"physical_keycode":65,"key_label":0,"unicode":97,"location":0,"echo":false,"script":null)]
}

[autoload]
GameManager="*res://scripts/game_manager.gd"

[editor_plugins]
enabled=PackedStringArray("res://addons/forge_sync/plugin.cfg")
```

### Key Sections

| Section | Purpose |
|---|---|
| `[application]` | Project name, main scene, features |
| `[input]` | Input action mappings |
| `[autoload]` | Singleton autoloads. `*` prefix = enabled |
| `[editor_plugins]` | Active editor plugins |
| `[display]` | Window size, stretch mode |
| `[rendering]` | Renderer settings |
| `[physics]` | Physics engine settings |

Rules:
- `config_version=5` is required for Godot 4.x (appears before any section)
- Keys use `section/subsection` format within each `[group]`
- Autoload prefix `*` means the autoload is enabled; without it, it's disabled
- Input actions use a complex serialized Object format — prefer `godot-forge project input add` over hand-editing

---

## export_presets.cfg — Export Configuration

```ini
[preset.0]
name="Windows Desktop"
platform="Windows Desktop"
export_path="export/windows/game.exe"
runnable=true
...

[preset.0.options]
binary_format/embed_pck=true
...

[preset.1]
name="Linux"
platform="Linux/X11"
...
```

Rules:
- Presets are zero-indexed: `[preset.0]`, `[preset.1]`, etc.
- Each preset has a matching `[preset.N.options]` section
- `runnable=true` marks the default export preset
- Platform names must match Godot's exact platform identifiers

---

## Godot Variant Types

Common types found in property values across all formats:

| Type | Syntax | Example |
|---|---|---|
| Vector2 | `Vector2(x, y)` | `Vector2(100, 200)` |
| Vector3 | `Vector3(x, y, z)` | `Vector3(0, 1, 0)` |
| Color | `Color(r, g, b, a)` | `Color(1, 0.5, 0, 1)` |
| Rect2 | `Rect2(x, y, w, h)` | `Rect2(0, 0, 64, 64)` |
| Transform2D | `Transform2D(xx, xy, yx, yy, ox, oy)` | `Transform2D(1, 0, 0, 1, 0, 0)` |
| Transform3D | `Transform3D(...)` | 12-float basis + origin |
| NodePath | `NodePath("path")` | `NodePath("Player/Sprite")` |
| ExtResource | `ExtResource("id")` | `ExtResource("1")` |
| SubResource | `SubResource("id")` | `SubResource("RectangleShape2D_abc")` |
| PackedStringArray | `PackedStringArray("a", "b")` | `PackedStringArray("4.4")` |
| PackedVector2Array | `PackedVector2Array(x1, y1, x2, y2)` | Flat list of floats |
| Boolean | `true` / `false` | Lowercase only |
| String | `"quoted"` | `"Hello World"` |
| Integer | `42` | No quotes |
| Float | `3.14` | No quotes, decimal point |
| Dictionary | `{ "key": value }` | JSON-like syntax |
| Array | `[a, b, c]` | Square brackets |
| null | `null` | Lowercase |

### Color value range

⚠️ Color components are **0.0–1.0 floats**, not 0–255 integers. `Color(1, 0, 0, 1)` is red, not `Color(255, 0, 0, 255)`.

---

## Common Mistakes & Warnings

### ⚠️ Never guess UIDs
UIDs (`uid://...`) are assigned by the Godot engine's resource indexing system. **Never fabricate a UID.** Use `godot-forge engine uid` to let the engine assign them. A wrong UID will cause resource loading failures.

### ⚠️ load_steps must be correct
Formula: `ext_resource count + sub_resource count + 1`. An incorrect value causes Godot to log errors and may prevent the scene from loading.

### ⚠️ Do not set `type` on instanced nodes
When using `instance=ExtResource("N")`, the node inherits its type from the instanced scene. Setting `type` alongside `instance` is invalid.

### ⚠️ Node names must be unique among siblings
Two nodes with the same `parent` cannot share the same `name`. Godot will silently rename one, breaking references.

### ⚠️ Root node has no `parent`
The first `[node]` entry must omit the `parent` attribute entirely. Adding `parent=""` or `parent="."` to the root is invalid.

### ⚠️ Ordering matters
Sections must appear in order: header → ext_resource → sub_resource → node → connection. Godot's parser expects this sequence.

### ⚠️ String IDs in Godot 4.x
Unlike Godot 3.x (which used integer IDs), Godot 4.x uses string IDs for resources. `ExtResource("1")` is a string, not `ExtResource(1)`.

### ⚠️ Do not hand-edit `.godot/` or `.import` files
The `.godot/` directory and `.import` files are engine-managed cache. Editing them directly will be overwritten or cause corruption. Use `godot-forge engine import` instead.

### ⚠️ `res://` paths are case-sensitive on Linux
While macOS/Windows may forgive case mismatches, Linux (and most CI/export targets) will not. Always match exact case in all `path` references.

### ⚠️ Input action serialization is fragile
The `[input]` section in `project.godot` uses a complex `Object(...)` serialization. Hand-editing is error-prone. Prefer `godot-forge project input add` or the Godot editor.
