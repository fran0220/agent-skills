import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, extname, join } from "node:path";
import { Command } from "commander";
import { createContext, printError, printSuccess } from "./common.js";

function inferExtension(assetType: string): string {
  const map: Record<string, string> = { image: "png", audio: "mp3", music: "mp3", tts: "mp3", video: "mp4", model3d: "glb", text: "txt", sprite: "png", world: "spz" };
  return map[assetType] ?? "bin";
}

function inferExtFromResult(result: Record<string, unknown>): string | null {
  const meta = result.metadata as Record<string, unknown> | undefined;
  if (!meta) return null;
  const ct = meta.content_type as string | undefined;
  if (!ct) return null;
  const map: Record<string, string> = { "image/gif": "gif", "image/webp": "webp", "image/png": "png", "image/jpeg": "jpg", "video/mp4": "mp4", "audio/mpeg": "mp3" };
  return map[ct] ?? null;
}

function stripDataUri(data: string): string {
  const idx = data.indexOf(";base64,");
  return idx >= 0 ? data.slice(idx + 8) : data;
}

async function saveOutput(
  result: Record<string, unknown>,
  assetType: string,
  outputDir: string
): Promise<string | null> {
  const ext = inferExtFromResult(result) ?? inferExtension(assetType);
  const timestamp = Date.now();
  const filename = `${assetType}_${timestamp}.${ext}`;
  mkdirSync(outputDir, { recursive: true });
  const filePath = join(outputDir, filename);

  if (result.output_data) {
    const raw = String(result.output_data);
    if (assetType === "text") {
      writeFileSync(filePath, raw, "utf8");
    } else {
      const base64 = stripDataUri(raw);
      writeFileSync(filePath, Buffer.from(base64, "base64"));
    }
    return filePath;
  }

  if (result.output_url) {
    const response = await fetch(String(result.output_url));
    if (!response.ok) {
      return null;
    }
    const buffer = Buffer.from(await response.arrayBuffer());
    writeFileSync(filePath, buffer);
    return filePath;
  }

  return null;
}

async function saveNamedOutput(
  result: Record<string, unknown>,
  assetType: string,
  filePath: string
): Promise<string | null> {
  mkdirSync(dirname(filePath), { recursive: true });

  if (result.output_data) {
    const raw = String(result.output_data);
    if (assetType === "text") {
      writeFileSync(filePath, raw, "utf8");
    } else {
      writeFileSync(filePath, Buffer.from(stripDataUri(raw), "base64"));
    }
    return filePath;
  }

  if (result.output_url) {
    const response = await fetch(String(result.output_url));
    if (!response.ok) {
      return null;
    }
    const buffer = Buffer.from(await response.arrayBuffer());
    writeFileSync(filePath, buffer);
    return filePath;
  }

  return null;
}

function parseFrameSize(raw: string): { frame_width: number; frame_height: number } {
  const [width, height] = raw.split("x");
  const frameWidth = Number(width);
  const frameHeight = Number(height);
  if (!Number.isFinite(frameWidth) || !Number.isFinite(frameHeight)) {
    throw new Error("frame size must be formatted as WIDTHxHEIGHT");
  }
  return { frame_width: frameWidth, frame_height: frameHeight };
}

export function createGenerateCommand(): Command {
  const command = new Command("generate").description("Generate assets via the gateway");

  command.addCommand(
    new Command("image")
      .description("Generate an image from a text prompt")
      .requiredOption("--prompt <text>", "Image description prompt")
      .option("--provider <id>", "Provider to use")
      .option("--transparent", "Request transparent background")
      .option("--model <model>", "Model to use")
      .option("--size <size>", "Image size (e.g. 1024x1024)")
      .option("--input <url>", "Input image URL for editing (Gemini/Grok)")
      .option("--ref <urls...>", "Reference image URLs for multi-image editing (repeatable)")
      .option("--edit-mode <mode>", "Edit mode: edit, inpaint, restyle, expand")
      .option("--session <id>", "Session ID for multi-turn editing")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const body: Record<string, unknown> = {
            asset_type: "image",
            prompt: options.prompt,
          };
          if (options.provider) body.provider = options.provider;
          if (options.transparent) body.transparent = true;
          if (options.model) body.model = options.model;
          if (options.size) body.size = options.size;
          if (options.input) body.input_file = options.input;
          if (options.ref && options.ref.length > 0) body.reference_images = options.ref;
          if (options.editMode) body.edit_mode = options.editMode;
          if (options.session) body.session_id = options.session;

          const data = await ctx.client.post("/api/generate", body) as Record<string, unknown>;
          const localPath = await saveOutput(data, "image", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("generate.image", data, ctx);
        } catch (error) {
          printError("generate.image", error);
        }
      })
  );

  command.addCommand(
    new Command("video")
      .description("Generate a video from a text prompt (or image-to-video with --input)")
      .requiredOption("--prompt <text>", "Video description prompt")
      .option("--provider <id>", "Provider to use")
      .option("--input <url>", "Reference image URL for image-to-video (Grok)")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const body: Record<string, unknown> = {
            asset_type: "video",
            prompt: options.prompt,
          };
          if (options.provider) body.provider = options.provider;
          if (options.input) body.input_file = options.input;

          const data = await ctx.client.post("/api/generate", body) as Record<string, unknown>;
          const localPath = await saveOutput(data, "video", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("generate.video", data, ctx);
        } catch (error) {
          printError("generate.video", error);
        }
      })
  );

  command.addCommand(
    new Command("batch")
      .description("Batch generate multiple assets with shared parameters")
      .requiredOption("--prompt <texts...>", "Multiple prompts (one per frame)")
      .option("--asset-type <type>", "Asset type", "image")
      .option("--transparent", "Request transparent background")
      .option("--size <size>", "Image size")
      .option("--ref <urls...>", "Reference image URLs")
      .option("--compose <direction>", "Auto-compose: horizontal, vertical, grid")
      .option("--columns <n>", "Grid columns for compose")
      .option("--frame-size <size>", "Frame size for compose (e.g. 64x64)")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const shared: Record<string, unknown> = {};
          if (options.transparent) shared.transparent = true;
          if (options.size) shared.size = options.size;
          if (options.ref && options.ref.length > 0) shared.reference_images = options.ref;

          const body: Record<string, unknown> = {
            asset_type: options.assetType,
            prompts: options.prompt,
            shared,
          };

          if (options.compose) {
            const compose: Record<string, unknown> = { direction: options.compose };
            if (options.columns) compose.columns = Number(options.columns);
            if (options.frameSize) Object.assign(compose, parseFrameSize(options.frameSize));
            body.compose = compose;
          }

          const data = await ctx.client.post("/api/generate/batch", body) as Record<string, unknown>;
          const assetType = String(options.assetType);
          mkdirSync(options.outputDir, { recursive: true });

          if (Array.isArray(data.frames)) {
            for (const frame of data.frames) {
              if (typeof frame !== "object" || frame === null) continue;
              const record = frame as Record<string, unknown>;
              const index = typeof record.index === "number" ? record.index : 0;
              const localPath = await saveNamedOutput(
                record,
                assetType,
                join(options.outputDir, `frame_${String(index).padStart(3, "0")}.${inferExtension(assetType)}`)
              );
              if (localPath) record.local_path = localPath;
            }
          }

          if (typeof data.spritesheet === "object" && data.spritesheet !== null) {
            const record = data.spritesheet as Record<string, unknown>;
            const localPath = await saveNamedOutput(
              record,
              "image",
              join(options.outputDir, "spritesheet.png")
            );
            if (localPath) record.local_path = localPath;
          }

          printSuccess("generate.batch", data, ctx);
        } catch (error) {
          printError("generate.batch", error);
        }
      })
  );

  command.addCommand(
    new Command("audio")
      .description("Generate audio from a text prompt")
      .requiredOption("--prompt <text>", "Audio description prompt")
      .option("--type <type>", "Audio type: bgm or sfx")
      .option("--duration <seconds>", "Duration in seconds")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const body: Record<string, unknown> = {
            asset_type: "audio",
            prompt: options.prompt,
          };
          const params: Record<string, unknown> = {};
          if (options.type) params.audio_type = options.type;
          if (options.duration) params.duration_seconds = Number(options.duration);
          if (Object.keys(params).length > 0) body.params = params;

          const data = await ctx.client.post("/api/generate", body) as Record<string, unknown>;
          const localPath = await saveOutput(data, "audio", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("generate.audio", data, ctx);
        } catch (error) {
          printError("generate.audio", error);
        }
      })
  );

  command.addCommand(
    new Command("music")
      .description("Generate music using the gateway music provider")
      .requiredOption("--prompt <text>", "Music description prompt")
      .option("--duration <seconds>", "Duration in seconds")
      .option("--force-instrumental", "Request instrumental output when supported")
      .option(
        "--output-format <fmt>",
        "Provider-specific output format override when supported"
      )
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const body: Record<string, unknown> = {
            asset_type: "music",
            prompt: options.prompt,
          };
          const params: Record<string, unknown> = {};
          if (options.duration) params.duration_seconds = Number(options.duration);
          if (options.forceInstrumental) params.force_instrumental = true;
          if (options.outputFormat) params.output_format = options.outputFormat;
          if (Object.keys(params).length > 0) body.params = params;

          const data = await ctx.client.post("/api/generate", body) as Record<string, unknown>;
          const localPath = await saveOutput(data, "music", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("generate.music", data, ctx);
        } catch (error) {
          printError("generate.music", error);
        }
      })
  );

  command.addCommand(
    new Command("tts")
      .description(
        "Text-to-speech: default Qwen3-TTS; use --provider elevenlabs --voice-id for ElevenLabs"
      )
      .requiredOption("--prompt <text>", "Text to synthesize")
      .option("--voice <name>", "Qwen voice name or custom voice id", "Cherry")
      .option(
        "--voice-id <id>",
        "ElevenLabs voice_id (use with --provider elevenlabs; routes to TTS API)"
      )
      .option("--language <lang>", "Language hint: Auto, Chinese, English, Japanese, etc.", "Auto")
      .option("--model <model>", "Model id (default: auto-detect from voice; qwen3-tts-flash for built-in voices)")
      .option("--instructions <text>", "Natural language speaking instructions (for instruct models)")
      .option("--provider <id>", "qwen_tts | elevenlabs | voicebox")
      .option("--profile-id <id>", "VoiceBox profile_id (use with --provider voicebox)")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const params: Record<string, unknown> = {
            voice: options.voice,
            language_type: options.language,
          };
          if (options.instructions) params.instructions = options.instructions;
          if (options.voiceId) params.voice_id = options.voiceId;
          if (options.profileId) params.profile_id = options.profileId;

          const body: Record<string, unknown> = {
            asset_type: "tts",
            prompt: options.prompt,
            params,
          };
          if (options.model) body.model = options.model;
          if (options.provider) body.provider = options.provider;

          const data = await ctx.client.post("/api/generate", body) as Record<string, unknown>;
          const localPath = await saveOutput(data, "tts", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("generate.tts", data, ctx);
        } catch (error) {
          printError("generate.tts", error);
        }
      })
  );

  command.addCommand(
    new Command("model")
      .description("Generate a 3D model")
      .option("--image <url>", "Reference image URL")
      .option("--prompt <text>", "Model description prompt")
      .option("--model-version <v>", "Tripo model version (e.g. P1-20260311)")
      .option("--face-limit <n>", "Max face count (48-20000)")
      .option("--pbr", "Enable PBR textures")
      .option("--texture-quality <q>", "Texture quality: standard or detailed")
      .option("--auto-size", "Auto-scale to real-world dimensions")
      .option("--negative-prompt <text>", "Negative prompt")
      .option("--multiview <urls>", "4 image URLs comma-separated: front,left,back,right")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const body: Record<string, unknown> = {
            asset_type: "model3d",
          };
          const params: Record<string, unknown> = {};
          if (options.image) body.input_file = options.image;
          if (options.prompt) body.prompt = options.prompt;
          if (options.modelVersion) params.model_version = options.modelVersion;
          if (options.faceLimit) params.face_limit = Number(options.faceLimit);
          if (options.pbr) params.pbr = true;
          if (options.textureQuality) params.texture_quality = options.textureQuality;
          if (options.autoSize) params.auto_size = true;
          if (options.negativePrompt) params.negative_prompt = options.negativePrompt;
          if (options.multiview) {
            params.multiview = String(options.multiview)
              .split(",")
              .map((value) => value.trim())
              .filter(Boolean);
          }
          if (Object.keys(params).length > 0) body.params = params;

          const data = await ctx.client.post("/api/generate", body) as Record<string, unknown>;
          const localPath = await saveOutput(data, "model3d", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("generate.model", data, ctx);
        } catch (error) {
          printError("generate.model", error);
        }
      })
  );

  command.addCommand(
    new Command("text")
      .description("Generate text via LLM")
      .requiredOption("--prompt <text>", "Text prompt")
      .option("--model <model>", "Model to use")
      .option("--max-tokens <n>", "Maximum tokens")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const body: Record<string, unknown> = {
            asset_type: "text",
            prompt: options.prompt,
          };
          if (options.model) body.model = options.model;
          if (options.maxTokens) body.max_tokens = Number(options.maxTokens);

          const data = await ctx.client.post("/api/generate", body) as Record<string, unknown>;
          const localPath = await saveOutput(data, "text", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("generate.text", data, ctx);
        } catch (error) {
          printError("generate.text", error);
        }
      })
  );

  command.addCommand(
    new Command("sprite")
      .description("Generate character animation (Veo AI video + frame extraction)")
      .requiredOption("--prompt <text>", "Character description")
      .option("--input <path>", "Reference image for character consistency (local path or URL)")
      .option("--animation-type <type>", "Animation type (idle, walk, run, attack, death, jump, cast, dance, or any custom)", "walk")
      .option("--direction <dir>", "Facing direction: front, left, right, back", "front")
      .option("--view <view>", "Camera view angle: auto, side, front, back, three-quarter, none", "auto")
      .option("--framing <framing>", "Framing: full-body, waist-up, close-up, none", "full-body")
      .option("--background <bg>", "Background: auto, white, none, or free text (e.g. 'forest clearing')", "auto")
      .option("--duration <n>", "Video duration in seconds (1-15)", "2")
      .option("--style <style>", "Visual style (e.g. pixel art, hand-drawn, chibi)")
      .option("--output-format <fmt>", "Output format: spritesheet or gif", "spritesheet")
      .option("--fps <n>", "GIF frame rate", "8")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const params: Record<string, unknown> = {
            animation_type: options.animationType,
            direction: options.direction,
            duration: Number(options.duration),
            output_format: options.outputFormat,
            fps: Number(options.fps),
          };
          if (options.view && options.view !== "auto") params.view = options.view;
          if (options.framing && options.framing !== "full-body") params.framing = options.framing;
          if (options.background && options.background !== "auto") params.background = options.background;
          if (options.style) params.style = options.style;

          let inputFile = options.input as string | undefined;
          if (inputFile && existsSync(inputFile)) {
            const ext = extname(inputFile).toLowerCase();
            const mime = ext === ".jpg" || ext === ".jpeg" ? "image/jpeg" : "image/png";
            const b64 = readFileSync(inputFile).toString("base64");
            inputFile = `data:${mime};base64,${b64}`;
          }

          const body: Record<string, unknown> = {
            asset_type: "sprite",
            prompt: options.prompt,
            params,
          };
          if (inputFile) body.input_file = inputFile;

          const data = await ctx.client.post("/api/generate", body) as Record<string, unknown>;
          const localPath = await saveOutput(data, "sprite", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("generate.sprite", data, ctx);
        } catch (error) {
          printError("generate.sprite", error);
        }
      })
  );

  command.addCommand(
    new Command("world")
      .description("Generate a 3D world/environment using WorldLabs Marble")
      .requiredOption("--prompt <text>", "Text prompt describing the environment")
      .option("--input <path>", "Input image (local path or URL) for image-to-world")
      .option("--model <model>", "Model: marble-1.0-draft, marble-1.0, marble-1.1, marble-1.1-plus", "marble-1.1")
      .option("--display-name <name>", "Display name for the generated world")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const params: Record<string, unknown> = {};
          if (options.displayName) params.display_name = options.displayName;

          // Encode local file to data URI if needed
          let inputFile: string | undefined;
          if (options.input) {
            if (existsSync(options.input)) {
              const ext = extname(options.input).toLowerCase();
              const mime = ext === ".jpg" || ext === ".jpeg" ? "image/jpeg" : "image/png";
              const b64 = readFileSync(options.input).toString("base64");
              inputFile = `data:${mime};base64,${b64}`;
            } else {
              inputFile = options.input;
            }
          }

          const body: Record<string, unknown> = {
            asset_type: "world",
            prompt: options.prompt,
            model: options.model,
            params,
          };
          if (inputFile) body.input_file = inputFile;

          const data = await ctx.client.post("/api/generate", body) as Record<string, unknown>;
          const localPath = await saveOutput(data, "world", options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("generate.world", data, ctx);
        } catch (error) {
          printError("generate.world", error);
        }
      })
  );

  return command;
}
