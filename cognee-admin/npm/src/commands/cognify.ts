import { readFile } from "node:fs/promises";
import { Command } from "commander";
import { configError } from "../errors.js";
import { createContext, printError, printSuccess } from "./common.js";

export function createCognifyCommand(): Command {
  return new Command("cognify")
    .description("Trigger cognification pipeline")
    .option("--dataset-id <id>", "Optional dataset ID to cognify")
    .option("--dataset-name <name>", "Optional dataset name to cognify")
    .option("--custom-prompt <prompt>", "Override the default cognify prompt")
    .option("--custom-prompt-file <path>", "Read custom prompt text from a file")
    .option("--background", "Run cognify in background mode")
    .option("--chunks-per-batch <count>", "Override chunks per batch for the pipeline")
    .action(
      async function (options: {
        datasetId?: string;
        datasetName?: string;
        customPrompt?: string;
        customPromptFile?: string;
        background?: boolean;
        chunksPerBatch?: string;
      }) {
        try {
          const ctx = createContext(this);
          if (options.datasetId && options.datasetName) {
            throw configError("use either --dataset-id or --dataset-name, not both");
          }
          if (options.customPrompt && options.customPromptFile) {
            throw configError("use either --custom-prompt or --custom-prompt-file, not both");
          }

          const datasets = options.datasetId ?? options.datasetName;
          const customPrompt = options.customPromptFile
            ? await readFile(options.customPromptFile, "utf8")
            : options.customPrompt;
          const chunksPerBatch = options.chunksPerBatch
            ? Number.parseInt(options.chunksPerBatch, 10)
            : undefined;

          if (options.chunksPerBatch && !Number.isFinite(chunksPerBatch)) {
            throw configError("--chunks-per-batch must be a valid integer");
          }

          const data = await ctx.client.cognify({
            ...(datasets ? { datasets: [datasets] } : {}),
            ...(customPrompt ? { custom_prompt: customPrompt } : {}),
            run_in_background: Boolean(options.background),
            ...(typeof chunksPerBatch === "number"
              ? { chunks_per_batch: chunksPerBatch }
              : {}),
          });

          printSuccess("cognify", data, ctx);
        } catch (error) {
          printError("cognify", error);
        }
      }
    );
}
