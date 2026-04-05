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

For generating full music segments.

```bash
asset-gateway generate music --prompt "uplifting indie game theme, warm synths" --duration 30 --output-dir ./assets
asset-gateway generate music --prompt "lofi study beat with soft piano" --duration 45 --output-dir ./assets
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

## Text

Single-shot LLM text generation.

```bash
asset-gateway generate text --prompt "Write a backstory for a desert kingdom." --output-dir ./assets
asset-gateway generate text --prompt "Describe a crafting system." --model gpt-5.4 --output-dir ./assets
```
