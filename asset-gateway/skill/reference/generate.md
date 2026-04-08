# Generate Commands

## Image

One command covers generation, editing, reference images, inpainting, restyling, and expanding.

### Basic Generation

```bash
asset-gateway generate image --prompt "isometric village, soft morning light" --size 1792x1024 --output-dir ./assets
```

### Transparent Background

Add `--transparent` for true alpha channel PNG (routes to GPT Image, RGBA output). Best for game sprites, UI icons, character art.

```bash
asset-gateway generate image --prompt "pixel art sword icon" --transparent --output-dir ./assets
asset-gateway generate image --prompt "character sprite, chibi style" --transparent --size 1024x1792 --output-dir ./assets
```

> Without `--transparent`: Gemini (fast, cheap). With `--transparent`: GPT Image (native alpha, slower).

### Editing

Supports `--input`, up to 14 `--ref` images, and `--edit-mode` (edit, inpaint, restyle, expand).

```bash
asset-gateway generate image --prompt "replace the sky with sunset clouds" --input https://example.com/scene.png --output-dir ./assets
asset-gateway generate image --prompt "put these characters in the same cafe" --ref https://example.com/a.png https://example.com/b.png --output-dir ./assets
```

### Multi-Turn Editing

First generation returns `session_id`; pass it with `--session` to continue.

```bash
asset-gateway generate image --prompt "create a product poster" --size 1792x1024 --output-dir ./assets
# Response includes session_id: ses_abc123
asset-gateway generate image --prompt "make headline larger, translate to Spanish" --session ses_abc123 --output-dir ./assets
```

### Size Control

Use `--size WxH`: `1024x1024`, `1792x1024`, `1024x1792`, `4096x2304`, etc.

```bash
asset-gateway generate image --prompt "mobile splash screen" --size 1024x1792 --output-dir ./assets
```

## Video

Image-to-video via Jimeng Seedance (ByteDance). Requires an input image — use `--input` with an image URL.

### Image-to-Video

Pass an input image with `--input` for image-to-video generation. The image serves as the reference frame. Seedance models (`seedance-2.0-fast-vip`, `seedance-2.0-vip`) require at least one image.

```bash
asset-gateway generate video --prompt "walk cycle animation, smooth movement" --input https://upload.xiaomao.chat/uploads/character.png --output-dir ./assets
```

> Seedance produces consistent character frames from the input image. Default model: `seedance-2.0-fast-vip` (faster), or use `--provider jimeng` with `--model seedance-2.0-vip` for higher quality.

## Audio

For sound effects and ambient audio, not music.

```bash
asset-gateway generate audio --prompt "sword slash impact" --type sfx --output-dir ./assets
asset-gateway generate audio --prompt "ambient medieval tavern" --type bgm --duration 30 --output-dir ./assets
```

## Music

For generating full music segments. Powered by Google Lyria 3 — 30-second high-quality clips at $0.04/clip.

```bash
asset-gateway generate music --prompt "uplifting indie game theme, warm synths" --output-dir ./assets
asset-gateway generate music --prompt "lofi study beat with soft piano, instrumental only" --output-dir ./assets
asset-gateway generate music --prompt "8-bit chiptune battle theme in C minor" --output-dir ./assets
```

## TTS

### Standard TTS

```bash
asset-gateway generate tts --prompt "欢迎来到今天的产品演示。" --voice Cherry --language Chinese --output-dir ./assets
asset-gateway generate tts --prompt "Welcome to the launch event." --voice Ethan --language English --output-dir ./assets
```

### Instructed TTS

```bash
asset-gateway generate tts --prompt "This is the emergency broadcast." --instructions "calm, authoritative, slow pacing" --output-dir ./assets
```

## World

Generate photorealistic 3D environments (Gaussian Splat scenes) using WorldLabs Marble API. Outputs `.spz` file with collider mesh and panorama URLs in metadata.

### Text to World

```bash
asset-gateway generate world --prompt "a medieval tavern with wooden beams and a roaring fireplace" --output-dir ./assets
```

### Image to World

```bash
asset-gateway generate world --prompt "convert this reference into a 3D scene" --input ./reference.jpg --output-dir ./assets
asset-gateway generate world --prompt "photorealistic interior" --input https://example.com/photo.jpg --output-dir ./assets
```

### Model Selection

| Model | Speed | Quality | Use Case |
|-------|-------|---------|----------|
| `marble-1.0-draft` | Fast | Draft | Previews, iteration |
| `marble-1.0` | Medium | Good | Standard generation |
| `marble-1.1` (default) | Medium | High | Production quality |
| `marble-1.1-plus` | Slow | Highest | Largest worlds |

```bash
asset-gateway generate world --prompt "sprawling cityscape" --model marble-1.1-plus --output-dir ./assets
```

> Generation takes ~3-8 minutes. The response metadata includes download URLs for all resolution tiers (100k/500k/full_res SPZ), collider GLB, and panorama image.

## Text

Single-shot LLM text generation.

```bash
asset-gateway generate text --prompt "Write a backstory for a desert kingdom." --output-dir ./assets
asset-gateway generate text --prompt "Describe a crafting system." --model gpt-5.4 --output-dir ./assets
```
