# 3D Model Pipeline

Generate 3D models and process them through the Tripo3D pipeline.

## Generate

```bash
# From text
asset-gateway generate model --prompt "stylized low-poly warrior, T-pose" --face-limit 5000 --pbr --output-dir ./assets

# From image
asset-gateway generate model --image https://asset.origingame.dev/uploads/concept.png --face-limit 8000 --pbr --output-dir ./assets

# From 4-view images (front, left, back, right)
asset-gateway generate model --multiview front.png,left.png,back.png,right.png --face-limit 5000 --pbr --output-dir ./assets
```

## Post-Processing Chain

Every step takes a `--task-id` from the previous step's response (`metadata.tripo_task_id`).

### Rig (add skeleton)

```bash
asset-gateway process3d rig --task-id abc-123 --spec mixamo --output-dir ./assets
```

### Animate

```bash
asset-gateway process3d animate --task-id rig-456 --animation preset:walk --output-dir ./assets
```

### Texture (re-texture)

```bash
asset-gateway process3d texture --task-id abc-123 --prompt "bright hand-painted fantasy materials" --pbr --output-dir ./assets
```

### Convert (export format)

Formats: `GLTF`, `FBX`, `USDZ`, `OBJ`, `STL`, `3MF`.

```bash
asset-gateway process3d convert --task-id abc-123 --format FBX --output-dir ./assets
```

### Other Operations

| Command | Purpose |
|---------|---------|
| `process3d reduce` | Reduce polygon count |
| `process3d stylize` | Apply style (lego, voxel, voronoi, minecraft) |
| `process3d segment` | Split mesh into parts |
| `process3d prerigcheck` | Check if model can be rigged |
| `process3d refine` | Improve draft model quality |
| `process3d import` | Import external model into Tripo pipeline |

### Typical Chain

```bash
# Generate → Rig → Animate → Convert
asset-gateway generate model --prompt "warrior character" --pbr --output-dir ./m
# task_id from response: gen-001

asset-gateway process3d rig --task-id gen-001 --spec mixamo --output-dir ./m
# task_id: rig-002

asset-gateway process3d animate --task-id rig-002 --animation preset:walk --output-dir ./m
# task_id: anim-003

asset-gateway process3d convert --task-id anim-003 --format FBX --output-dir ./m
```
