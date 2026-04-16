import { Command } from "commander";
import { createContext, printError, printSuccess } from "./common.js";

function withVoiceType(path: string, type?: string): string {
  if (!type) {
    return path;
  }

  const params = new URLSearchParams({ type });
  return `${path}?${params.toString()}`;
}

export function createVoiceCommand(): Command {
  const command = new Command("voice").description("Manage Qwen3-TTS voice designs");

  command.addCommand(
    new Command("design")
      .description("Create a synthetic voice and save the preview audio file")
      .requiredOption("--prompt <text>", "Voice description prompt")
      .requiredOption("--preview-text <text>", "Preview text for the generated sample")
      .requiredOption("--name <name>", "Name for the designed voice")
      .option("--target-model <model>", "Voice design model, e.g. qwen3-tts-vd-2026-01-26")
      .option("--output-dir <dir>", "Directory to save preview audio", ".")
      .action(async function (options: { prompt: string; previewText: string; name: string; targetModel?: string; outputDir: string }) {
        try {
          const ctx = createContext(this);
          const body: Record<string, unknown> = {
            voice_prompt: options.prompt,
            preview_text: options.previewText,
            name: options.name,
          };
          if (options.targetModel) body.target_model = options.targetModel;

          const data = await ctx.client.post("/api/voice/design", body) as Record<string, unknown>;

          // Save preview audio to file for use as TTS reference
          const designData = data?.data as Record<string, unknown> | undefined;
          const b64Audio = designData?.preview_audio_data as string | undefined;
          if (b64Audio) {
            const { mkdirSync, writeFileSync } = await import("fs");
            const { join } = await import("path");
            mkdirSync(options.outputDir, { recursive: true });
            const buf = Buffer.from(b64Audio, "base64");
            const outPath = join(options.outputDir, `${options.name}_preview.wav`);
            writeFileSync(outPath, buf);
            (data as Record<string, unknown>).preview_audio_path = outPath;
          }

          printSuccess("voice.design", data, ctx);
        } catch (error) {
          printError("voice.design", error);
        }
      })
  );

  command.addCommand(
    new Command("synthesize")
      .description("Synthesize speech using a designed voice")
      .requiredOption("--voice <name>", "Voice name from voice design")
      .requiredOption("--text <text>", "Text to synthesize")
      .option("--model <model>", "TTS model (default: qwen3-tts-vd-realtime-2025-12-16)")
      .option("--language <lang>", "Language: zh, en, ja, etc. (default: Auto)")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options: { voice: string; text: string; model?: string; language?: string; outputDir: string }) {
        try {
          const ctx = createContext(this);
          const body: Record<string, unknown> = {
            voice: options.voice,
            text: options.text,
          };
          if (options.model) body.model = options.model;
          if (options.language) body.language = options.language;

          const data = await ctx.client.post("/api/voice/synthesize", body) as Record<string, unknown>;

          // Save audio if url is returned (DashScope: output.audio.url)
          const result = (data as Record<string, unknown>).data as Record<string, unknown> | undefined;
          const output = result?.output as Record<string, unknown> | undefined;
          const audio = output?.audio as Record<string, unknown> | undefined;
          const audioUrl = audio?.url as string | undefined
            ?? output?.url as string | undefined;
          if (audioUrl && options.outputDir) {
            const { mkdirSync, writeFileSync } = await import("fs");
            const { join } = await import("path");
            mkdirSync(options.outputDir, { recursive: true });
            const resp = await fetch(audioUrl);
            if (resp.ok) {
              const buf = Buffer.from(await resp.arrayBuffer());
              const outPath = join(options.outputDir, `tts_${Date.now()}.wav`);
              writeFileSync(outPath, buf);
              (data as Record<string, unknown>).local_path = outPath;
            }
          }

          printSuccess("voice.synthesize", data, ctx);
        } catch (error) {
          printError("voice.synthesize", error);
        }
      })
  );

  command.addCommand(
    new Command("list")
      .description("List custom designed voices")
      .option("--type <type>", "Voice type: vd")
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
      .description("Delete a custom designed voice")
      .argument("<voice-id>", "Voice ID to delete")
      .option("--type <type>", "Voice type: vd")
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
