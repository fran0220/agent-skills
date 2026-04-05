/**
 * Self-describe payloads for `asset-gateway describe` — keep in sync with CLI commands.
 */
export const SCHEMAS: Record<string, object> = {
  auth: {
    description: "Credential management",
    subcommands: {
      set: {
        description: "Save token and gateway URL locally",
        params: {
          token: { type: "string", required: true, description: "API token or admin token" },
        },
      },
      status: { description: "Show current authentication status" },
      clear: { description: "Remove saved credentials" },
    },
  },
  generate: {
    description: "Generate assets via the gateway",
    subcommands: {
      image: {
        description: "Generate or edit an image (Gemini / GPT Image / Grok)",
        params: {
          "--prompt": { type: "string", required: true, description: "Image prompt" },
          "--provider": { type: "string", required: false },
          "--transparent": { type: "bool", description: "Transparent background" },
          "--model": { type: "string" },
          "--size": { type: "string", description: 'e.g. "1024x1024"' },
          "--input": { type: "string", description: "Image URL for editing" },
          "--ref": { type: "string[]", description: "Reference image URLs (repeatable)" },
          "--edit-mode": { type: "string", description: "edit | inpaint | restyle | expand" },
          "--session": { type: "string", description: "Multi-turn session id" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      video: {
        description: "Text or image-to-video (Grok)",
        params: {
          "--prompt": { type: "string", required: true },
          "--provider": { type: "string" },
          "--input": { type: "string", description: "Image URL for I2V" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      batch: {
        description: "Batch image (etc.) generation; optional sprite compose",
        params: {
          "--prompt": { type: "string[]", required: true, description: "One prompt per frame" },
          "--asset-type": { type: "string", default: "image" },
          "--transparent": { type: "bool" },
          "--size": { type: "string" },
          "--ref": { type: "string[]" },
          "--compose": { type: "string", description: "horizontal | vertical | grid" },
          "--columns": { type: "number" },
          "--frame-size": { type: "string", description: "e.g. 64x64" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      audio: {
        description: "BGM/SFX via ElevenLabs sound-generation",
        params: {
          "--prompt": { type: "string", required: true },
          "--type": { type: "string", description: "bgm | sfx" },
          "--duration": { type: "number", description: "Seconds" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      music: {
        description: "Music via ElevenLabs POST /v1/music (model music_v1)",
        params: {
          "--prompt": { type: "string", required: true },
          "--duration": { type: "number", description: "Seconds; mapped to music_length_ms (3s–600s)" },
          "--force-instrumental": { type: "bool", description: "Optional; passed to provider" },
          "--output-format": { type: "string", description: "Optional; e.g. mp3_44100_128" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      tts: {
        description:
          "TTS: default Qwen3-TTS (voice/language/instructions). ElevenLabs: --provider elevenlabs --voice-id <id>",
        params: {
          "--prompt": { type: "string", required: true },
          "--voice": { type: "string", description: "Qwen voice name or custom id", default: "Cherry" },
          "--voice-id": { type: "string", description: "ElevenLabs voice id (with --provider elevenlabs)" },
          "--language": { type: "string", default: "Auto" },
          "--model": { type: "string", default: "qwen3-tts-flash" },
          "--instructions": { type: "string", description: "Instruct-model style control" },
          "--provider": { type: "string", description: "qwen_tts | elevenlabs" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      model: {
        description: "3D model generation (Tripo)",
        params: {
          "--image": { type: "string", description: "Reference image URL" },
          "--prompt": { type: "string" },
          "--model-version": { type: "string" },
          "--face-limit": { type: "number" },
          "--pbr": { type: "bool" },
          "--texture-quality": { type: "string" },
          "--auto-size": { type: "bool" },
          "--negative-prompt": { type: "string" },
          "--multiview": { type: "string", description: "Comma-separated 4 view URLs" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      text: {
        description: "LLM text via proxy",
        params: {
          "--prompt": { type: "string", required: true },
          "--model": { type: "string" },
          "--max-tokens": { type: "number" },
          "--output-dir": { type: "string", default: "." },
        },
      },
    },
  },
  process: {
    description: "Image/video post-process on gateway (ImageMagick/ffmpeg/rembg)",
    subcommands: {
      crop: {
        description: "Smart crop (trim / power_of2)",
        params: {
          "--input": { type: "string", required: true, description: "File path or URL" },
          "--mode": { type: "string", default: "tightest" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      resize: {
        description: "Resize to exact width/height",
        params: {
          "--input": { type: "string", required: true },
          "--width": { type: "number", required: true },
          "--height": { type: "number", required: true },
          "--output-dir": { type: "string", default: "." },
        },
      },
      compose: {
        description: "Sprite sheet from multiple images",
        params: {
          "--input": { type: "string[]", required: true },
          "--direction": { type: "string", default: "horizontal" },
          "--columns": { type: "number" },
          "--padding": { type: "string" },
          "--frame-width": { type: "number" },
          "--frame-height": { type: "number" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      "extract-frames": {
        description: "Sample frames from video (ffmpeg)",
        params: {
          "--input": { type: "string", required: true },
          "--count": { type: "string", default: "8" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      "remove-bg": {
        description: "Remove background (rembg / fallback)",
        params: {
          "--input": { type: "string[]", required: true },
          "--bg-color": { type: "string" },
          "--output-dir": { type: "string", default: "." },
        },
      },
    },
  },
  process3d: {
    description: "Tripo 3D follow-up operations (chain on tripo_task_id)",
    subcommands: {
      convert: { description: "Export format (FBX/GLTF/…)", params: { "--task-id": { required: true }, "--format": { required: true } } },
      texture: { description: "Re-texture", params: { "--task-id": { required: true } } },
      rig: { description: "Auto-rig", params: { "--task-id": { required: true } } },
      animate: { description: "Retarget animation", params: { "--task-id": { required: true } } },
      "render-sprites": { description: "Blender render to 2D frames", params: { "--task-id": { required: true } } },
      reduce: { description: "Low-poly", params: { "--task-id": { required: true } } },
      stylize: { description: "Style transfer", params: { "--task-id": { required: true } } },
      segment: { description: "Mesh segmentation", params: { "--task-id": { required: true } } },
      prerigcheck: { description: "Rig eligibility", params: { "--task-id": { required: true } } },
      refine: { description: "Refine quality", params: { "--task-id": { required: true } } },
      import: { description: "Import external model", params: { "--file-url": { type: "string" }, "--file-path": { type: "string" } } },
    },
  },
  voice: {
    description: "Qwen3-TTS custom voices (clone / design / list / delete)",
    subcommands: {
      clone: {
        description: "Clone from audio sample",
        params: { "--audio": { required: true }, "--name": { required: true } },
      },
      design: {
        description: "Design voice from text",
        params: { "--prompt": { required: true }, "--name": { required: true } },
      },
      list: { description: "List custom voices", params: { "--type": { type: "string" } } },
      delete: { description: "Delete by voice id", params: { "<voice-id>": { required: true } } },
    },
  },
  upload: {
    description: "Upload and list gateway assets",
    subcommands: {
      file: { description: "Upload file → URL", params: { "<path>": { required: true } } },
      list: { description: "List uploads" },
      delete: { description: "Delete by filename (admin)", params: { "<filename>": { required: true } } },
    },
  },
  provider: {
    description: "Provider discovery and health",
    subcommands: {
      list: { description: "List providers" },
      health: { description: "Health check", params: { name: { type: "string", required: false } } },
    },
  },
  job: {
    description: "Async job history",
    subcommands: {
      list: {
        description: "List jobs",
        params: {
          "--status": { type: "string" },
          "--limit": { type: "number" },
        },
      },
      status: { description: "Job detail", params: { id: { type: "string", required: true } } },
      cancel: { description: "Cancel pending/running", params: { id: { type: "string", required: true } } },
    },
  },
  describe: {
    description: "Command introspection (this output)",
    params: {
      command: { type: "string", required: false, description: "Top-level group: generate, process, …" },
    },
  },
};
