import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { Command } from "commander";
import { createContext, printError, printSuccess } from "./common.js";

function readInputAsBase64(input: string): string {
  if (existsSync(input)) {
    const bytes = readFileSync(input);
    return `data:image/png;base64,${bytes.toString("base64")}`;
  }
  return input;
}

function saveProcessOutput(data: Record<string, unknown>, outputDir: string): string | null {
  const outputData = data.output_data;
  if (typeof outputData !== "string" || !outputData) return null;

  mkdirSync(outputDir, { recursive: true });
  const timestamp = Date.now();
  const filePath = join(outputDir, `processed_${timestamp}.png`);
  writeFileSync(filePath, Buffer.from(outputData, "base64"));
  delete data.output_data;
  return filePath;
}

export function createProcessCommand(): Command {
  const command = new Command("process").description("Post-process images and video (crop, resize, compose, extract-frames, remove-bg)");

  command.addCommand(
    new Command("crop")
      .description("Smart crop an image (trim transparent borders)")
      .requiredOption("--input <path>", "Input image (file path or URL)")
      .option("--mode <mode>", "Crop mode: tightest or power_of2", "tightest")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.post("/api/process", {
            input: readInputAsBase64(options.input),
            operations: [{ op: "smart_crop", mode: options.mode }],
          }) as Record<string, unknown>;

          const localPath = saveProcessOutput(data, options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process.crop", data, ctx);
        } catch (error) {
          printError("process.crop", error);
        }
      })
  );

  command.addCommand(
    new Command("resize")
      .description("Resize an image to exact dimensions")
      .requiredOption("--input <path>", "Input image (file path or URL)")
      .requiredOption("--width <n>", "Target width")
      .requiredOption("--height <n>", "Target height")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.post("/api/process", {
            input: readInputAsBase64(options.input),
            operations: [{ op: "resize", width: Number(options.width), height: Number(options.height) }],
          }) as Record<string, unknown>;

          const localPath = saveProcessOutput(data, options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process.resize", data, ctx);
        } catch (error) {
          printError("process.resize", error);
        }
      })
  );

  command.addCommand(
    new Command("compose")
      .description("Compose multiple images into a sprite sheet")
      .requiredOption("--input <paths...>", "Input images (files or URLs)")
      .option("--direction <dir>", "Layout: horizontal, vertical, grid", "horizontal")
      .option("--columns <n>", "Columns for grid layout")
      .option("--padding <n>", "Padding between frames in px", "0")
      .option("--frame-width <n>", "Normalize each frame to this width")
      .option("--frame-height <n>", "Normalize each frame to this height")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const inputs = Array.isArray(options.input) ? options.input : [options.input];
          const data = await ctx.client.post("/api/process", {
            inputs: inputs.map(readInputAsBase64),
            operations: [{
              op: "compose",
              direction: options.direction,
              columns: options.columns ? Number(options.columns) : undefined,
              padding: Number(options.padding),
              frame_width: options.frameWidth ? Number(options.frameWidth) : undefined,
              frame_height: options.frameHeight ? Number(options.frameHeight) : undefined,
            }],
          }) as Record<string, unknown>;

          const localPath = saveProcessOutput(data, options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process.compose", data, ctx);
        } catch (error) {
          printError("process.compose", error);
        }
      })
  );

  command.addCommand(
    new Command("extract-frames")
      .description("Extract evenly-spaced frames from a video using ffmpeg")
      .requiredOption("--input <path>", "Input video (file path or URL)")
      .option("--count <n>", "Number of frames to extract", "8")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.post("/api/process", {
            input: readInputAsBase64(options.input),
            operations: [{ op: "extract_frames", count: Number(options.count) }],
          }) as Record<string, unknown>;

          const localPath = saveProcessOutput(data, options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process.extract_frames", data, ctx);
        } catch (error) {
          printError("process.extract_frames", error);
        }
      })
  );

  command.addCommand(
    new Command("remove-bg")
      .description("Remove background from image(s) using rembg or ImageMagick fallback")
      .requiredOption("--input <paths...>", "Input images (files or URLs)")
      .option("--bg-color <color>", "Background color hint for fallback (e.g. white, black)")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const inputs = Array.isArray(options.input) ? options.input : [options.input];
          const op: Record<string, unknown> = { op: "remove_bg" };
          if (options.bgColor) op.bg_color = options.bgColor;

          const data = await ctx.client.post("/api/process", {
            inputs: inputs.map(readInputAsBase64),
            operations: [op],
          }) as Record<string, unknown>;

          const localPath = saveProcessOutput(data, options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process.remove_bg", data, ctx);
        } catch (error) {
          printError("process.remove_bg", error);
        }
      })
  );

  return command;
}
