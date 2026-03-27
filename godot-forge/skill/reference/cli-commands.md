# godot-forge CLI — Complete Command Reference

52 commands across 8 groups. JSON output by default.

## Global Options

| Flag | Description |
|------|-------------|
| `--project <path>` | Project directory (default: cwd) |
| `--human` | Human-readable output |
| `--fields <fields>` | Comma-separated output field filter |
| `--force` | Force overwrite/delete |
| `--dry-run` | Preview changes without executing |
| `--input <file>` | Read JSON input from file |

## Output Envelope

All commands return:
```json
{"ok": true, "command": "<cmd>", "data": { ... }}
{"ok": false, "command": "<cmd>", "error": {"code": "ERROR_CODE", "message": "...", "suggestion": "..."}}
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Command error (invalid input, file not found) |
| 2 | Engine error (Godot headless failure) |

---

## project — Project Lifecycle (L1)

### project init
Initialize a new Godot 4.x project with ForgeSync plugin.
```bash
godot-forge project init --name my-game
godot-forge project init --name my-game --template default
```
**Input**: `{name: string, template?: string}`
**Output**: `{path, name, files_created[]}`

### project info
Show project information.
```bash
godot-forge project info
```
**Output**: `{path, name, godot_version}`

### project validate
Validate project integrity (broken refs, missing files).
```bash
godot-forge project validate
```
**Output**: `{valid, issues[], files_checked}`

### project clean
Find orphaned files (unreferenced scripts/resources).
```bash
godot-forge project clean
godot-forge project clean --force  # delete orphans
```
**Output**: `{orphaned_count, orphaned[], deleted}`

### project config get
```bash
godot-forge project config get application/config/name
```
**Output**: `{key, value, section}`

### project config set
```bash
godot-forge project config set application/config/name '"My Game"'
```
**Output**: `{key, value, section}`

### project config list
```bash
godot-forge project config list
godot-forge project config list application
```
**Output**: `{sections}`

### project autoload add
```bash
echo '{"name":"GameManager","path":"res://scripts/autoload/game_manager.gd"}' | godot-forge project autoload add
```
**Input**: `{name, path}`  **Output**: `{name, path}`

### project autoload remove
```bash
echo '{"name":"GameManager"}' | godot-forge project autoload remove
```
**Input**: `{name}`  **Output**: `{name, removed}`

### project autoload list
```bash
godot-forge project autoload list
```
**Output**: `{count, autoloads[]}`

### project input add
```bash
echo '{"name":"move_left","keys":["A","Left"],"deadzone":0.2}' | godot-forge project input add
```
**Input**: `{name, keys[], deadzone?}`  **Output**: `{name, keys}`

### project input remove
```bash
echo '{"name":"move_left"}' | godot-forge project input remove
```
**Input**: `{name}`  **Output**: `{name, removed}`

### project input list
```bash
godot-forge project input list
```
**Output**: `{count, actions[]}`

### project plugin list
```bash
godot-forge project plugin list
```
**Output**: `{count, plugins[]}`

### project plugin enable
```bash
echo '{"name":"res://addons/my_plugin/plugin.cfg"}' | godot-forge project plugin enable
```
**Input**: `{name}`  **Output**: `{plugin, enabled}`

### project plugin disable
```bash
echo '{"name":"res://addons/my_plugin/plugin.cfg"}' | godot-forge project plugin disable
```
**Input**: `{name}`  **Output**: `{plugin, disabled}`

### project template list
```bash
godot-forge project template list
```
**Output**: `{count, templates[]}`

---

## scene — Scene Operations (L2)

### scene create
```bash
echo '{"name":"player","root_type":"CharacterBody2D"}' | godot-forge scene create
echo '{"name":"player","root_type":"CharacterBody2D","children":[{"name":"Sprite","type":"Sprite2D"},{"name":"Collision","type":"CollisionShape2D"}]}' | godot-forge scene create
```
**Input**: `{name, root_type?, path?, children?[]}`  **Output**: `{path, res_path, root_type, node_count}`

### scene read
```bash
godot-forge scene read player
godot-forge scene read scenes/player
```
**Output**: `{path, scene}` (full scene structure as JSON)

### scene list
```bash
godot-forge scene list
```
**Output**: `{count, scenes[]}`

### scene delete
```bash
godot-forge scene delete player --force
```
**Output**: `{path, deleted}`

### scene update
```bash
echo '{"scene":"player","root_type":"RigidBody2D","root_name":"PlayerRoot"}' | godot-forge scene update
```
**Input**: `{scene, root_type?, root_name?, uid?}`  **Output**: `{scene, updated}`

### scene rename
```bash
echo '{"scene":"player","new_name":"hero"}' | godot-forge scene rename
```
**Input**: `{scene, new_name}`  **Output**: `{old_path, new_path, references_updated}`

### scene merge
```bash
echo '{"target":"main","sources":["scenes/ui/hud.tscn","scenes/enemies/spawner.tscn"],"parent":"."}' | godot-forge scene merge
```
**Input**: `{target, sources[], parent?}`  **Output**: `{target, sources_merged, nodes_added}`

---

## node — Node Tree Operations (L2)

### node add
```bash
# Basic node
echo '{"scene":"main","name":"Player","type":"CharacterBody2D"}' | godot-forge node add

# With parent
echo '{"scene":"main","name":"Sprite","type":"Sprite2D","parent":"Player"}' | godot-forge node add

# With script (auto ext_resource)
echo '{"scene":"main","name":"Player","type":"CharacterBody2D","script":"res://scripts/player.gd"}' | godot-forge node add

# Scene instance (auto PackedScene ext_resource)
echo '{"scene":"main","name":"Enemy1","instance":"res://scenes/enemy.tscn"}' | godot-forge node add

# With properties
echo '{"scene":"main","name":"Camera","type":"Camera2D","properties":{"zoom":"Vector2(2, 2)"}}' | godot-forge node add
```
**Input**: `{scene, name, type?, parent?, properties?, script?, instance?}`
**Output**: `{scene, node, node_count}`

Special: CollisionShape2D/3D auto-creates shape sub_resource.

### node remove
```bash
echo '{"scene":"main","path":"Player"}' | godot-forge node remove
```
**Input**: `{scene, path}`  **Output**: `{removed_path, remaining_nodes}`

### node list
```bash
echo '{"scene":"main"}' | godot-forge node list
```
**Input**: `{scene}`  **Output**: `{count, nodes[]}`

### node update
```bash
echo '{"scene":"main","path":"Player","properties":{"speed":300},"script":"res://scripts/player.gd"}' | godot-forge node update
```
**Input**: `{scene, path, properties?, script?}`  **Output**: `{node_path, updated_properties[]}`

### node move
```bash
echo '{"scene":"main","path":"Sprite","new_parent":"Player"}' | godot-forge node move
```
**Input**: `{scene, path, new_parent}`  **Output**: `{old_path, new_path}`

### node duplicate
```bash
echo '{"scene":"main","path":"Enemy1","new_name":"Enemy2"}' | godot-forge node duplicate
```
**Input**: `{scene, path, new_name?}`  **Output**: `{source_path, duplicated_name, duplicated_node_count}`

### node connect
```bash
echo '{"scene":"main","signal":"pressed","from":"PlayButton","to":".","method":"_on_play_pressed"}' | godot-forge node connect
```
**Input**: `{scene, signal, from, to, method}`  **Output**: `{connection, total_connections}`

### node disconnect
```bash
echo '{"scene":"main","signal":"pressed","from":"PlayButton","to":"."}' | godot-forge node disconnect
```
**Input**: `{scene, signal, from, to}`  **Output**: `{removed}`

### node connections
```bash
echo '{"scene":"main"}' | godot-forge node connections
```
**Input**: `{scene}`  **Output**: `{count, connections[]}`

### node group add
```bash
echo '{"scene":"player","path":".","groups":["player","damageable"]}' | godot-forge node group add
```
**Input**: `{scene, path, groups[]}`  **Output**: `{node_path, groups[]}`

### node group remove
```bash
echo '{"scene":"player","path":".","group":"damageable"}' | godot-forge node group remove
```
**Input**: `{scene, path, group}`  **Output**: `{node_path, removed}`

### node group list
```bash
echo '{"scene":"player","path":"."}' | godot-forge node group list
```
**Input**: `{scene, path}`  **Output**: `{node_path, count, groups[]}`

---

## script — GDScript Management (L1)

### script create
```bash
echo '{"name":"player","extends":"CharacterBody2D","path":"scripts/player.gd","signals":["died","health_changed"],"exports":[{"name":"speed","type":"float","default":300.0}],"methods":["_ready","_physics_process","take_damage"]}' | godot-forge script create
```
**Input**: `{name, extends?, path?, signals?[], methods?[], exports?[]}`
**Output**: `{path, class_name}`

### script read
```bash
godot-forge script read scripts/player.gd
```
**Output**: `{path, extends, class_name, signals[], methods[], exports[], line_count}`

### script list
```bash
godot-forge script list
```
**Output**: `{count, scripts[]}`

### script validate
```bash
godot-forge script validate scripts/player.gd
```
**Output**: `{path, valid, issues[]}`

### script edit
```bash
echo '{"path":"scripts/player.gd","add_signals":["landed"],"add_methods":["respawn"],"add_exports":[{"name":"max_health","type":"int","default":100}]}' | godot-forge script edit
```
**Input**: `{path, add_signals?[], add_methods?[], add_exports?[]}`
**Output**: `{path, added}`

---

## engine — Engine Operations (L3a/L3b)

### engine editor 🟣
```bash
godot-forge engine editor
```
**Output**: `{pid}`

### engine validate 🟠
```bash
godot-forge engine validate
godot-forge engine validate --deep  # via Godot headless
```
**Output**: `{valid, issues[]}`

### engine run 🟠
```bash
godot-forge engine run
godot-forge engine run --scene res://scenes/main.tscn
godot-forge engine run --headless --timeout 5000
```
**Output**: `{exit_code}`

### engine import 🟠
```bash
godot-forge engine import
```
**Output**: `{success}`

### engine uid 🟠
```bash
godot-forge engine uid
```
**Output**: `{uids, count}`

### engine preview 🟠
```bash
echo '{"scene":"res://scenes/main.tscn","output":"preview.png","size":[1920,1080]}' | godot-forge engine preview
```
**Input**: `{scene?, output?, size?[]}`  **Output**: `{output, width, height}`

---

## export — Build Export (L1/L3a)

### export preset list
```bash
godot-forge export preset list
```
**Output**: `{count, presets[]}`

### export preset add
```bash
echo '{"name":"Windows","platform":"Windows Desktop","path":"export/windows/game.exe","runnable":true}' | godot-forge export preset add
```
**Input**: `{name, platform, path?, runnable?}`  **Output**: `{operation, index}`

### export preset remove
```bash
echo '{"name":"Windows"}' | godot-forge export preset remove --force
```
**Input**: `{name}`  **Output**: `{name, removed, remaining_count}`

### export build 🟠
```bash
godot-forge export build --preset "Windows"
echo '{"preset":"Windows","output":"build/game.exe"}' | godot-forge export build
```
**Input**: `{preset, output?}`  **Output**: `{preset, output}`

---

## resource — Resource Management (L1/L2/L3a)

### resource create
```bash
echo '{"type":"RectangleShape2D","name":"player_shape","path":"resources/shapes/player_shape.tres","properties":{"size":"Vector2(32, 64)"}}' | godot-forge resource create
```
**Input**: `{type, name, path, properties?}`  **Output**: `{file, type}`

### resource read
```bash
godot-forge resource read resources/shapes/player_shape.tres
```
**Output**: `{path, type, properties}`

### resource update
```bash
echo '{"path":"resources/shapes/player_shape.tres","properties":{"size":"Vector2(48, 96)"}}' | godot-forge resource update
```
**Input**: `{path, properties}`  **Output**: `{path, updated_properties[]}`

### resource delete
```bash
godot-forge resource delete resources/shapes/player_shape.tres --force
```
**Output**: `{path, deleted, referenced_by}`

### resource list
```bash
godot-forge resource list
```
**Output**: `{scenes[], scripts[], resources[], textures[], audio[]}`

### resource import 🟠
```bash
godot-forge resource import
```
**Output**: `{log}`

### resource check
```bash
godot-forge resource check
```
**Output**: `{files_scanned, missing_count, missing[]}`

---

## describe — Schema Introspection (L1/L3a)

### describe [command]
```bash
godot-forge describe scene.create
godot-forge describe node.add
```
**Output**: Command schema with input/output/flags

### describe --all
```bash
godot-forge describe --all
```
**Output**: `{commands[], schemas}`

### describe node-types 🟠
```bash
godot-forge describe node-types
```
**Output**: `{node_types[], count}` (all Node subclasses from ClassDB)

### describe resource-types 🟠
```bash
godot-forge describe resource-types
```
**Output**: `{resource_types[], count}` (all Resource subclasses)

### describe class:\<Name\> 🟠
```bash
godot-forge describe class:CharacterBody2D
godot-forge describe class:Sprite2D
```
**Output**: `{class, parent, properties[], signals[], methods[]}`

### describe project-settings 🟠
```bash
godot-forge describe project-settings
```
**Output**: `{settings[], count}`
