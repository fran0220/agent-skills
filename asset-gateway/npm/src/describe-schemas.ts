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
        description: "Generate or edit an image",
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
        description: "Generate a video from text or an input image",
        params: {
          "--prompt": { type: "string", required: true },
          "--provider": { type: "string" },
          "--input": { type: "string", description: "Image URL for I2V" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      batch: {
        description: "Batch-generate assets with shared parameters and optional compose step",
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
      sfx: {
        description: "Generate sound effects (impacts, footsteps, UI sounds, ambience)",
        params: {
          "--prompt": { type: "string", required: true },
          "--duration": { type: "number", required: true, description: "Duration in seconds (1-5s for short SFX, 5-15s for ambience, max 30s)" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      music: {
        description: "Generate music using the gateway music provider",
        params: {
          "--prompt": { type: "string", required: true },
          "--duration": { type: "number", description: "Seconds" },
          "--force-instrumental": { type: "bool", description: "Request instrumental output when supported" },
          "--output-format": { type: "string", description: "Provider-specific output format override when supported" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      tts: {
        description: "Text-to-speech via Gemini 3.1 Flash TTS",
        params: {
          "--prompt": { type: "string", required: true },
          "--voice": { type: "string", description: "Prebuilt voice name (default: Kore)" },
          "--speakers": { type: "string", description: "Multi-speaker config JSON, e.g. '{\"Name1\":\"Puck\",\"Name2\":\"Kore\"}'" },
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
      sprite: {
        description: "Generate character animation spritesheet (AutoSprite)",
        params: {
          "--prompt": { type: "string", required: true, description: "Character description" },
          "--input": { type: "string", description: "Reference image path or URL" },
          "--animation-type": { type: "string", default: "walk", description: "walk, run, idle, jump, attack, death, cast, dance, wave, interact, or custom text" },
          "--style": { type: "string", description: "Art style: 16-bit, hd-pixel, isometric, retro-8bit, anime, chibi, painterly, vector" },
          "--frame-count": { type: "number", default: 8, description: "Number of animation frames" },
          "--frame-size": { type: "number", default: 256, description: "Frame size in pixels (square)" },
          "--is-humanoid": { type: "boolean", default: true },
          "--output-dir": { type: "string", default: "." },
        },
      },
      world: {
        description: "Generate a 3D world or environment",
        params: {
          "--prompt": { type: "string", required: true },
          "--input": { type: "string", description: "Input image path or URL" },
          "--model": { type: "string", default: "marble-1.1" },
          "--display-name": { type: "string" },
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
      convert: {
        description: "Convert a 3D model to another format",
        params: {
          "--task-id": { type: "string", required: true },
          "--format": { type: "string", required: true },
          "--quad": { type: "bool" },
          "--face-limit": { type: "number" },
          "--pack-uv": { type: "bool" },
          "--bake": { type: "bool" },
          "--texture-format": { type: "string" },
          "--force-symmetry": { type: "bool" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      texture: {
        description: "Apply new textures to a 3D model",
        params: {
          "--task-id": { type: "string", required: true },
          "--prompt": { type: "string" },
          "--style-image": { type: "string" },
          "--pbr": { type: "bool" },
          "--quality": { type: "string" },
          "--texture-alignment": { type: "string" },
          "--bake": { type: "bool" },
          "--texture-version": { type: "string" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      rig: {
        description: "Auto-rig a 3D model",
        params: {
          "--task-id": { type: "string", required: true },
          "--format": { type: "string", default: "glb" },
          "--spec": { type: "string", default: "mixamo" },
          "--rig-type": { type: "string" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      animate: {
        description: "Apply a preset animation to a rigged model",
        params: {
          "--task-id": { type: "string", required: true },
          "--animation": { type: "string", required: true },
          "--format": { type: "string", default: "glb" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      "render-sprites": {
        description: "Render an animated 3D model to sprite frames",
        params: {
          "--task-id": { type: "string", required: true },
          "--frame-count": { type: "number", default: 8 },
          "--resolution": { type: "number", default: 64 },
          "--camera-angle": { type: "string", default: "front" },
          "--directions": { type: "number", default: 1 },
          "--output-dir": { type: "string", default: "." },
        },
      },
      reduce: {
        description: "Reduce polygon count",
        params: {
          "--task-id": { type: "string", required: true },
          "--face-limit": { type: "number" },
          "--quad": { type: "bool" },
          "--output-dir": { type: "string", default: "." },
        },
      },
      stylize: {
        description: "Apply a stylized look to a model",
        params: {
          "--task-id": { type: "string", required: true },
          "--style": { type: "string", required: true },
          "--output-dir": { type: "string", default: "." },
        },
      },
      segment: {
        description: "Segment a mesh into logical parts",
        params: {
          "--task-id": { type: "string", required: true },
          "--output-dir": { type: "string", default: "." },
        },
      },
      prerigcheck: {
        description: "Check whether a model can be rigged",
        params: {
          "--task-id": { type: "string", required: true },
          "--output-dir": { type: "string", default: "." },
        },
      },
      refine: {
        description: "Refine a draft model to higher quality",
        params: {
          "--task-id": { type: "string", required: true },
          "--output-dir": { type: "string", default: "." },
        },
      },
      import: {
        description: "Import an external 3D model for post-processing",
        params: {
          "--file-url": { type: "string" },
          "--file-path": { type: "string" },
          "--output-dir": { type: "string", default: "." },
        },
      },
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
      health: { description: "Health check", params: { "[name]": { type: "string", required: false } } },
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
      status: { description: "Job detail", params: { "<id>": { type: "string", required: true } } },
      cancel: { description: "Cancel pending/running", params: { "<id>": { type: "string", required: true } } },
    },
  },
  describe: {
    description: "Command introspection (this output)",
    params: {
      "[command]": { type: "string", required: false, description: "Top-level group: generate, process, …" },
    },
  },
};
