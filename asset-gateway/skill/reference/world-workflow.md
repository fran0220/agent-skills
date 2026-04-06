# 3D World Generation Workflow

WorldLabs Marble API generates photorealistic 3D environments from text prompts or reference images. Output includes Gaussian Splat scenes (.spz), collider meshes (.glb), and panorama images.

## Quick Start

### Text to World

```bash
asset-gateway generate world \
  --prompt "a medieval tavern with wooden beams and a roaring fireplace" \
  --output-dir ./worlds
```

### Image to World (Recommended)

Image input produces more faithful results — the AI matches visual style, layout, lighting.

```bash
asset-gateway generate world \
  --prompt "convert this reference into a 3D scene" \
  --input ./reference-photo.jpg \
  --output-dir ./worlds
```

## Model Selection

| Model | Speed | Quality | Best For |
|-------|-------|---------|----------|
| `marble-1.0-draft` | ~2 min | Draft | Quick previews, iteration |
| `marble-1.0` | ~4 min | Good | Standard generation |
| `marble-1.1` (default) | ~5 min | High | Production quality |
| `marble-1.1-plus` | ~8 min | Highest | Largest, most detailed worlds |

```bash
# Quick preview
asset-gateway generate world \
  --prompt "dark forest clearing" \
  --model marble-1.0-draft \
  --output-dir ./worlds

# Production quality
asset-gateway generate world \
  --prompt "cyberpunk alley with neon signs" \
  --model marble-1.1-plus \
  --output-dir ./worlds
```

## Output Assets

The response metadata contains download URLs for all assets:

| Asset | Format | Description |
|-------|--------|-------------|
| Gaussian Splat (100k) | `.spz` | Low-res, mobile/preview |
| Gaussian Splat (500k) | `.spz` | Medium quality, desktop games |
| Gaussian Splat (full_res) | `.spz` | Full quality (auto-downloaded) |
| Collider Mesh | `.glb` | Invisible physics mesh for raycasting |
| Panorama | `.jpg`/`.png` | Equirectangular skybox image |

The primary `.spz` file (full_res) is saved locally. Other assets are available via URLs in `metadata.assets`.

### Accessing Additional Assets

```bash
# The response metadata includes all asset URLs:
# metadata.assets.gaussian_splat_full_res → full quality SPZ
# metadata.assets.gaussian_splat_500k → medium SPZ
# metadata.assets.gaussian_splat_100k → low SPZ
# metadata.assets.collider_full_res → physics mesh GLB
# metadata.assets.panorama_full_res → skybox image
```

## Integration with Three.js

WorldLabs outputs render via SparkJS (`@sparkjsdev/spark`) — a Gaussian Splat renderer for Three.js.

```bash
npm install @sparkjsdev/spark
```

```js
import { SplatMesh } from '@sparkjsdev/spark';

// Load the splat — works like any Three.js object
const splat = new SplatMesh({ url: 'assets/worlds/tavern.spz' });
scene.add(splat);

// Standard renderer — no extra pass needed
renderer.render(scene, camera);
```

## Combining with 3D Models

WorldLabs environments + Tripo3D models = complete 3D scenes:

1. **Generate environment**: `asset-gateway generate world --prompt "medieval dungeon" --output-dir ./worlds`
2. **Generate character**: `asset-gateway generate model --prompt "medieval knight" --output-dir ./models`
3. **Rig & animate**: `asset-gateway process3d rig --task-id <id>` → `process3d animate --task-id <id>`
4. **Combine in Three.js**: Splat for visuals, collider mesh for physics, GLB models for characters

## Timing & Cost

- Generation takes **3–8 minutes** depending on model and server load
- Estimated cost: ~$0.50 per generation
- Poll interval: 10 seconds (automatic)
- Timeout: 10 minutes max

## Tips

- **Image input preferred** — produces more accurate, predictable results than text-only
- **Reference photos work best** — real photos of interiors/exteriors generate the most realistic worlds
- **Concept art also works** — stylized concept art produces stylized environments
- **Display name** — use `--display-name` to label worlds for organization in WorldLabs dashboard
- **Draft for iteration** — use `marble-1.0-draft` when experimenting, switch to `marble-1.1` for final

## More Examples

### Interior Scene

```bash
asset-gateway generate world \
  --prompt "Japanese zen garden room with tatami mats, sliding paper doors, and a small rock garden visible through the window" \
  --model marble-1.1 \
  --output-dir ./worlds
```

### From Concept Art

```bash
asset-gateway generate world \
  --prompt "bring this concept art to life as a 3D environment" \
  --input ./concept-art-ruins.jpg \
  --model marble-1.1-plus \
  --display-name "Ancient Ruins" \
  --output-dir ./worlds
```

### Quick Draft Iteration

```bash
# Iterate fast with drafts
asset-gateway generate world \
  --prompt "underwater coral reef with sunlight rays" \
  --model marble-1.0-draft \
  --output-dir ./worlds

# Happy with the concept? Regenerate at full quality
asset-gateway generate world \
  --prompt "underwater coral reef with sunlight rays filtering through the surface, bioluminescent coral, schools of tropical fish" \
  --model marble-1.1 \
  --output-dir ./worlds
```
