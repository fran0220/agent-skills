# Voice Commands

Manage custom voices for TTS.

## Clone a Voice

Create a custom voice from an audio sample:

```bash
asset-gateway voice clone --audio ./voice-sample.wav --name narrator_v1
```

**Name rules**: 1–16 characters, only letters, digits, and underscores. No hyphens or spaces.

## Design a Voice

Create a voice from a text description:

```bash
asset-gateway voice design --prompt "young female narrator, bright and friendly" --preview-text "Hello, welcome to our studio." --name host_v1
```

## List Voices

```bash
asset-gateway voice list
```

## Delete a Voice

```bash
asset-gateway voice delete host_v1
```

## Using Custom Voices with TTS

After cloning or designing, use the returned voice ID in TTS. The correct model is **auto-detected** from the voice ID prefix — no need to specify `--model`:

```bash
# Clone returns a voice ID like: qwen-tts-vc-narrator_v1-voice-20260413-xxxx
asset-gateway generate tts --prompt "Welcome back, adventurer." --voice qwen-tts-vc-narrator_v1-voice-20260413-xxxx --output-dir ./assets
```

### System Voices

Built-in voices available without cloning (use directly by name): `Cherry`, `Serena`, `Ethan`, `Chelsie`.
