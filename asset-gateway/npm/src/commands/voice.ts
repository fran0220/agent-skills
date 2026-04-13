import { existsSync, readFileSync } from "node:fs";
import { extname } from "node:path";
import { Command } from "commander";
import { configError } from "../errors.js";
import { createContext, printError, printSuccess } from "./common.js";

function inferAudioMime(filePath: string): string {
  const extension = extname(filePath).toLowerCase();
  const map: Record<string, string> = {
    ".mp3": "audio/mpeg",
    ".wav": "audio/wav",
    ".pcm": "audio/pcm",
    ".opus": "audio/opus",
    ".ogg": "audio/ogg",
    ".m4a": "audio/mp4",
    ".aac": "audio/aac",
    ".flac": "audio/flac",
  };
  return map[extension] ?? "application/octet-stream";
}

function readAudioAsBase64(filePath: string): { audio_base64: string; audio_mime: string } {
  if (!existsSync(filePath)) {
    throw configError(`Audio file not found: ${filePath}`);
  }

  const bytes = readFileSync(filePath);
  return {
    audio_base64: bytes.toString("base64"),
    audio_mime: inferAudioMime(filePath),
  };
}

function withVoiceType(path: string, type?: string): string {
  if (!type) {
    return path;
  }

  const params = new URLSearchParams({ type });
  return `${path}?${params.toString()}`;
}

export function createVoiceCommand(): Command {
  const command = new Command("voice").description("Manage Qwen3-TTS custom voices");

  command.addCommand(
    new Command("clone")
      .description("Clone a voice from an audio sample")
      .requiredOption("--audio <path>", "Reference audio file path")
      .requiredOption("--name <name>", "Name for the cloned voice")
      .option("--target-model <model>", "Voice cloning model, e.g. qwen3-tts-vc-2026-01-22")
      .action(async function (options: { audio: string; name: string; targetModel?: string }) {
        try {
          const ctx = createContext(this);
          const audio = readAudioAsBase64(options.audio);
          const body: Record<string, unknown> = {
            ...audio,
            name: options.name,
          };
          if (options.targetModel) body.target_model = options.targetModel;

          const data = await ctx.client.post("/api/voice/clone", body);
          printSuccess("voice.clone", data, ctx);
        } catch (error) {
          printError("voice.clone", error);
        }
      })
  );

  command.addCommand(
    new Command("design")
      .description("Create a synthetic voice from a text description")
      .requiredOption("--prompt <text>", "Voice description prompt")
      .requiredOption("--preview-text <text>", "Preview text for the generated sample")
      .requiredOption("--name <name>", "Name for the designed voice")
      .option("--target-model <model>", "Voice design model, e.g. qwen3-tts-vd-2026-01-26")
      .action(async function (options: { prompt: string; previewText: string; name: string; targetModel?: string }) {
        try {
          const ctx = createContext(this);
          const body: Record<string, unknown> = {
            voice_prompt: options.prompt,
            preview_text: options.previewText,
            name: options.name,
          };
          if (options.targetModel) body.target_model = options.targetModel;

          const data = await ctx.client.post("/api/voice/design", body);
          printSuccess("voice.design", data, ctx);
        } catch (error) {
          printError("voice.design", error);
        }
      })
  );

  command.addCommand(
    new Command("create-custom")
      .description("Design a custom voice and register it for self-hosted TTS (DashScope design → VoiceBox clone)")
      .requiredOption("--prompt <text>", "Voice description (e.g. '年轻女性，温暖亲切，标准普通话')")
      .requiredOption("--preview-text <text>", "Sample text for voice preview")
      .requiredOption("--name <name>", "Name for the custom voice")
      .option("--language <lang>", "Language: zh, en, ja, ko, etc.", "zh")
      .action(async function (options: { prompt: string; previewText: string; name: string; language: string }) {
        try {
          const ctx = createContext(this);
          const body = {
            voice_prompt: options.prompt,
            preview_text: options.previewText,
            name: options.name,
            language: options.language,
          };
          const data = await ctx.client.post("/api/voice/create-custom", body);
          printSuccess("voice.create_custom", data, ctx);
        } catch (error) {
          printError("voice.create_custom", error);
        }
      })
  );

  command.addCommand(
    new Command("list")
      .description("List custom cloned or designed voices")
      .option("--type <type>", "Voice type: vc or vd")
      .action(async function (options: { type?: string }) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.get(withVoiceType("/api/voice/list", options.type));
          printSuccess("voice.list", data, ctx);
        } catch (error) {
          printError("voice.list", error);
        }
      })
  );

  command.addCommand(
    new Command("delete")
      .description("Delete a custom cloned or designed voice")
      .argument("<voice-id>", "Voice ID to delete")
      .option("--type <type>", "Voice type: vc or vd")
      .action(async function (voiceId: string, options: { type?: string }) {
        try {
          const ctx = createContext(this);
          const path = withVoiceType(`/api/voice/${encodeURIComponent(voiceId)}`, options.type);
          const data = await ctx.client.delete(path);
          printSuccess("voice.delete", data, ctx);
        } catch (error) {
          printError("voice.delete", error);
        }
      })
  );

  return command;
}
