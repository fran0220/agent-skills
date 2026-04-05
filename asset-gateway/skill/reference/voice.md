# Voice Commands

Manage custom voices for TTS.

## Clone a Voice

Create a custom voice from an audio sample:

```bash
asset-gateway voice clone --audio ./voice-sample.wav --name narrator_v1
```

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

After cloning or designing, use the voice name in TTS:

```bash
asset-gateway generate tts --prompt "Welcome back, adventurer." --voice narrator_v1 --output-dir ./assets
```

### System Voices

Built-in voices available without cloning: `Cherry`, `Serena`, `Ethan`, `Chelsie`.
