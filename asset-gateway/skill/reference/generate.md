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

> Primary provider: ChatGPT2API (GPT Image). Fallback: Gemini. Both support `--transparent`.

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

Generate video from text prompt or image-to-video with `--input`. Multiple providers available.

### Text-to-Video

```bash
asset-gateway generate video --prompt "sweeping aerial shot of a Ming dynasty palace at golden hour" --output-dir ./assets
```

### Image-to-Video (recommended for consistency)

Pass a reference image with `--input` to ground the video in an existing visual. Upload local files first with `asset-gateway upload file`.

```bash
asset-gateway generate video --prompt "slow camera pan across the palace courtyard, morning mist" --input https://asset.origingame.dev/uploads/bg_court.png --output-dir ./assets
```

### Provider Selection

| Provider | Flag | Strengths |
|----------|------|-----------|
| **Veo** (default) | `--provider veo` | Highest quality, cinematic, good text-to-video and image-to-video |
| **Grok** | `--provider grok_image` | Fast, supports `--input` for image-to-video |
| **Jimeng/Seedance** | `--provider jimeng` | Character animation, requires `--input` image |

> **Best practice**: Use `--input` with a relevant background/CG image to maintain visual consistency across video beats. Upload the reference image first, then pass its URL.

## Sound Effects

`generate sfx` — short audio clips: impacts, footsteps, UI sounds, ambient loops. **Not** for music (use `generate music`) or speech (use `generate tts`).

`--duration` is **required** — without it the API guesses and produces overly long audio.

| Type | Duration | Examples |
|------|----------|----------|
| Short SFX | 1–3s | sword hit, button click, footstep, explosion |
| Medium SFX | 3–8s | door creak, thunder roll, spell cast |
| Ambience | 8–15s | tavern ambience, forest wind, rain |

```bash
asset-gateway generate sfx --prompt "sword slash impact" --duration 2 --output-dir ./assets
asset-gateway generate sfx --prompt "heavy door creaking open" --duration 4 --output-dir ./assets
asset-gateway generate sfx --prompt "ambient medieval tavern with chatter and fire" --duration 12 --output-dir ./assets
```

## Music

For generating full music segments. Powered by Google Lyria 3 — 30-second high-quality clips at $0.04/clip.

```bash
asset-gateway generate music --prompt "uplifting indie game theme, warm synths" --output-dir ./assets
asset-gateway generate music --prompt "lofi study beat with soft piano, instrumental only" --output-dir ./assets
asset-gateway generate music --prompt "8-bit chiptune battle theme in C minor" --output-dir ./assets
```

## TTS

Text-to-speech powered by Gemini 3.1 Flash TTS. 21 prebuilt voices, multi-speaker support, natural language style control.

### Basic TTS

```bash
asset-gateway generate tts --prompt "欢迎来到今天的产品演示。" --output-dir ./assets
asset-gateway generate tts --prompt "Say cheerfully: Have a wonderful day!" --voice Puck --output-dir ./assets
```

### Voice Selection

21 prebuilt voices available for Chinese and other languages:

| Voice | Style |
|-------|-------|
| Kore | Female, warm and clear |
| Puck | Male, lively and upbeat |
| Zephyr | Neutral, soft |
| Charon | Male, deep and steady |
| Fenrir | Male, powerful |
| Aoede | Female, elegant |
| Leda | Female, natural |
| Enceladus | Male, breathy |
| Sulafat | Female, calm |
| Vindemiatrix | Female, mature |

Full list: Zephyr, Puck, Charon, Kore, Fenrir, Aoede, Leda, Orus, Callirrhoe, Autonoe, Enceladus, Iapetus, Umbriel, Algieba, Despina, Erinome, Gacrux, Laomedeia, Sadachbia, Sulafat, Vindemiatrix.

```bash
asset-gateway generate tts --prompt "你好世界" --voice Kore --output-dir ./assets
asset-gateway generate tts --prompt "Hello world" --voice Charon --output-dir ./assets
```

### Style Control

Use natural language in the prompt to control tone, pace, emotion:

```bash
asset-gateway generate tts --prompt "用兴奋的语气说：太棒了！我们赢了！" --voice Puck --output-dir ./assets
asset-gateway generate tts --prompt "[whisper] 嘘，别出声... [normal] 好了，可以了！" --voice Kore --output-dir ./assets
```

### Multi-Speaker

Up to 2 speakers with independent voice assignment:

```bash
asset-gateway generate tts \
  --prompt "小明：今天去哪玩？\n小红：去公园吧！" \
  --speakers '{"小明":"Puck","小红":"Kore"}' \
  --output-dir ./assets
```

### Supported Languages

Auto-detected. Supports 30+ languages including Chinese, English, Japanese, Korean, French, German, Spanish, etc.

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
