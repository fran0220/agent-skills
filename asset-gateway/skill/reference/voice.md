# Voice Commands

## Voice Design → TTS Pipeline

Design a custom voice once, then use it for unlimited free TTS generation.

### Step 1: Design a Voice

Create a voice from a text description. Outputs a preview audio file:

```bash
asset-gateway voice design \
  --prompt "温柔知性的年轻女性声音，语速适中" \
  --preview-text "你好，欢迎来到我们的世界" \
  --name gentle_girl \
  --output-dir ./voices
# → ./voices/gentle_girl_preview.wav
```

### Step 2: Generate TTS with Designed Voice

Use the preview audio as reference for voice cloning:

```bash
asset-gateway generate tts \
  --prompt "今天天气真不错，我们一起去公园散步吧" \
  --input ./voices/gentle_girl_preview.wav \
  --output-dir ./audio
```

The preview `.wav` file can be reused indefinitely for any text.

### Example: Male Announcer

```bash
# Design
asset-gateway voice design \
  --prompt "成熟稳重的男性播音员，声音低沉有磁性" \
  --preview-text "各位听众朋友们，大家晚上好" \
  --name anchor_male \
  --output-dir ./voices

# Generate multiple lines with the same voice
asset-gateway generate tts --prompt "第一段台词" --input ./voices/anchor_male_preview.wav --output-dir ./audio
asset-gateway generate tts --prompt "第二段台词" --input ./voices/anchor_male_preview.wav --output-dir ./audio
```

## Manage Voices

```bash
asset-gateway voice list
asset-gateway voice delete <voice-id>
```

## Cost

- **Voice design**: small one-time cost per design (Qwen API)
- **TTS generation**: free, unlimited (self-hosted GPU)
