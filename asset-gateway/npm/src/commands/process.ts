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
  const command = new Command("process").description("Post-process images (crop, resize)");

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

  return command;
}
