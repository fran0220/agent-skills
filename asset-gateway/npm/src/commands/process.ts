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
  const command = new Command("process").description("Post-process images (remove-bg, crop, resize, upscale)");

  command.addCommand(
    new Command("remove-bg")
      .description("Remove background from an image (AI-powered, BiRefNet)")
      .requiredOption("--input <path>", "Input image (file path or URL)")
      .option("--smart-crop", "Also crop to power-of-2 after removing background")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const ops: Record<string, unknown>[] = [{ op: "remove_bg" }];
          if (options.smartCrop) {
            ops.push({ op: "smart_crop", mode: "power_of2" });
          }

          const data = await ctx.client.post("/api/process", {
            input: readInputAsBase64(options.input),
            operations: ops,
          }) as Record<string, unknown>;

          const localPath = saveProcessOutput(data, options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process.remove-bg", data, ctx);
        } catch (error) {
          printError("process.remove-bg", error);
        }
      })
  );

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
    new Command("upscale")
      .description("AI upscale an image (2x or 4x via Real-ESRGAN)")
      .requiredOption("--input <path>", "Input image (file path or URL)")
      .option("--scale <n>", "Scale factor (2 or 4)", "4")
      .option("--output-dir <dir>", "Directory to save output", ".")
      .action(async function (options) {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.post("/api/process", {
            input: readInputAsBase64(options.input),
            operations: [{ op: "upscale", scale: Number(options.scale) }],
          }) as Record<string, unknown>;

          const localPath = saveProcessOutput(data, options.outputDir);
          if (localPath) data.local_path = localPath;
          printSuccess("process.upscale", data, ctx);
        } catch (error) {
          printError("process.upscale", error);
        }
      })
  );

  return command;
}
