import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { Command } from "commander";
import { createContext, printError, printSuccess } from "./common.js";

function inferExtension(assetType: string): string {
  const map: Record<string, string> = { image: "png", audio: "mp3", video: "mp4", model3d: "glb", text: "txt" };
  return map[assetType] ?? "bin";
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
  const ext = inferExtension(assetType);
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
      .description("Generate a video from a text prompt")
      .requiredOption("--prompt <text>", "Video description prompt")
      .option("--provider <id>", "Provider to use")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const body: Record<string, unknown> = {
            asset_type: "video",
            prompt: options.prompt,
          };
          if (options.provider) body.provider = options.provider;

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
          if (options.type) body.audio_type = options.type;
          if (options.duration) body.duration = Number(options.duration);

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
    new Command("model")
      .description("Generate a 3D model")
      .option("--image <url>", "Reference image URL")
      .option("--prompt <text>", "Model description prompt")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const body: Record<string, unknown> = {
            asset_type: "model3d",
          };
          if (options.image) body.input_file = options.image;
          if (options.prompt) body.prompt = options.prompt;

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

  return command;
}
