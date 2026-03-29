import { readdir } from "node:fs/promises";
import { basename, join, relative } from "node:path";
import { Command } from "commander";
import { configError, notFoundError } from "../errors.js";
import { createContext, printError, printSuccess } from "./common.js";

const UPLOAD_BATCH_SIZE = 10;

export function createDataCommand(): Command {
  const command = new Command("data").description("Data management within datasets");

  command
    .addCommand(
      new Command("add")
        .description("Add text data to a dataset")
        .requiredOption("--dataset <name>", "Dataset name")
        .argument("<content>", "Text content")
        .action(async function (content: string, options: { dataset: string }) {
          try {
            const ctx = createContext(this);
            printSuccess("data.add", await ctx.client.addData(options.dataset, content), ctx);
          } catch (error) {
            printError("data.add", error);
          }
        })
    )
    .addCommand(
      new Command("add-file")
        .description("Upload a single file to a dataset")
        .requiredOption("--dataset <name>", "Dataset name")
        .argument("<path>", "File path to upload")
        .action(async function (filePath: string, options: { dataset: string }) {
          try {
            const ctx = createContext(this);
            printSuccess("data.add-file", await ctx.client.uploadFile(options.dataset, filePath), ctx);
          } catch (error) {
            printError("data.add-file", error);
          }
        })
    )
    .addCommand(
      new Command("add-dir")
        .description("Upload matching files from a directory in batches of 10")
        .requiredOption("--dataset <name>", "Dataset name")
        .option("--glob <pattern>", "Glob pattern used to filter files", "*")
        .argument("<dir>", "Directory to scan recursively")
        .action(async function (dir: string, options: { dataset: string; glob: string }) {
          try {
            const ctx = createContext(this);
            const files = await collectMatchingFiles(dir, options.glob);
            if (files.length === 0) {
              throw notFoundError(`No files matched '${options.glob}' in ${dir}`);
            }

            let uploaded = 0;
            let failed = 0;
            const errors: Array<{ path?: string; batch?: string[]; error: string }> = [];
            const totalBatches = Math.ceil(files.length / UPLOAD_BATCH_SIZE);

            for (let index = 0; index < totalBatches; index += 1) {
              const start = index * UPLOAD_BATCH_SIZE;
              const batch = files.slice(start, start + UPLOAD_BATCH_SIZE);
              console.error(
                `[${index + 1}/${totalBatches}] Uploading ${batch.length} file(s) to ${options.dataset}`
              );

              try {
                await ctx.client.uploadFiles(options.dataset, batch);
                uploaded += batch.length;
                console.error(
                  `[${index + 1}/${totalBatches}] Uploaded ${uploaded}/${files.length} file(s)`
                );
              } catch (batchError) {
                console.error(
                  `[${index + 1}/${totalBatches}] Batch upload failed, retrying individually`
                );
                errors.push({ batch, error: errorMessage(batchError) });

                for (const filePath of batch) {
                  try {
                    await ctx.client.uploadFile(options.dataset, filePath);
                    uploaded += 1;
                    console.error(`  OK ${relative(dir, filePath) || basename(filePath)}`);
                  } catch (fileError) {
                    failed += 1;
                    errors.push({ path: filePath, error: errorMessage(fileError) });
                    console.error(`  FAIL ${relative(dir, filePath) || basename(filePath)}: ${errorMessage(fileError)}`);
                  }
                }
              }
            }

            const summary = {
              dataset: options.dataset,
              directory: dir,
              glob: options.glob,
              batch_size: UPLOAD_BATCH_SIZE,
              total: files.length,
              uploaded,
              failed,
              skipped: 0,
              errors,
            };

            console.error(`Completed upload: success=${uploaded} failed=${failed} total=${files.length}`);
            printSuccess("data.add-dir", summary, ctx);
          } catch (error) {
            printError("data.add-dir", error);
          }
        })
    )
    .addCommand(
      new Command("list")
        .description("List data in a dataset")
        .argument("<datasetId>", "Dataset ID")
        .action(async function (datasetId: string) {
          try {
            const ctx = createContext(this);
            printSuccess("data.list", await ctx.client.datasetData(datasetId), ctx);
          } catch (error) {
            printError("data.list", error);
          }
        })
    )
    .addCommand(
      new Command("delete")
        .description("Delete data from a dataset")
        .requiredOption("--dataset-id <id>", "Dataset ID")
        .requiredOption("--data-id <id>", "Data ID")
        .action(async function (options: { datasetId: string; dataId: string }) {
          try {
            const ctx = createContext(this);
            printSuccess(
              "data.delete",
              await ctx.client.deleteData(options.datasetId, options.dataId),
              ctx
            );
          } catch (error) {
            printError("data.delete", error);
          }
        })
    )
    .addCommand(
      new Command("raw")
        .description("Get raw data content")
        .requiredOption("--dataset-id <id>", "Dataset ID")
        .requiredOption("--data-id <id>", "Data ID")
        .action(async function (options: { datasetId: string; dataId: string }) {
          try {
            const ctx = createContext(this);
            printSuccess(
              "data.raw",
              await ctx.client.rawData(options.datasetId, options.dataId),
              ctx
            );
          } catch (error) {
            printError("data.raw", error);
          }
        })
    )
    .addCommand(
      new Command("update")
        .description("Replace an existing data item with a new file")
        .requiredOption("--dataset-id <id>", "Dataset ID")
        .requiredOption("--data-id <id>", "Data ID")
        .argument("<path>", "File path to upload")
        .action(async function (
          filePath: string,
          options: { datasetId: string; dataId: string }
        ) {
          try {
            const ctx = createContext(this);
            printSuccess(
              "data.update",
              await ctx.client.updateData(options.datasetId, options.dataId, filePath),
              ctx
            );
          } catch (error) {
            printError("data.update", error);
          }
        })
    );

  return command;
}

async function collectMatchingFiles(rootDir: string, globPattern: string): Promise<string[]> {
  const matcher = createGlobMatcher(globPattern);
  const files: string[] = [];

  await visit(rootDir, rootDir, matcher, files);
  files.sort((a, b) => a.localeCompare(b));
  return files;
}

async function visit(
  rootDir: string,
  currentDir: string,
  matcher: (relativePath: string, baseName: string) => boolean,
  files: string[]
): Promise<void> {
  let entries;
  try {
    entries = await readdir(currentDir, { withFileTypes: true });
  } catch (error) {
    throw configError(
      `Failed to read directory ${currentDir}: ${error instanceof Error ? error.message : String(error)}`
    );
  }

  for (const entry of entries) {
    const fullPath = join(currentDir, entry.name);
    if (entry.isDirectory()) {
      await visit(rootDir, fullPath, matcher, files);
      continue;
    }

    if (!entry.isFile()) {
      continue;
    }

    const relativePath = relative(rootDir, fullPath).split("\\").join("/");
    if (matcher(relativePath, entry.name)) {
      files.push(fullPath);
    }
  }
}

function createGlobMatcher(globPattern: string): (relativePath: string, baseName: string) => boolean {
  if (!globPattern || globPattern.trim() === "") {
    throw configError("Invalid glob pattern: pattern must not be empty");
  }

  const regex = globToRegExp(globPattern);
  const hasSlash = globPattern.includes("/");

  return (relativePath: string, baseName: string) => {
    if (regex.test(relativePath)) {
      return true;
    }
    return !hasSlash && regex.test(baseName);
  };
}

function globToRegExp(globPattern: string): RegExp {
  let regex = "^";

  for (let i = 0; i < globPattern.length; i += 1) {
    const char = globPattern[i];
    const next = globPattern[i + 1];

    if (char === "*") {
      if (next === "*") {
        regex += ".*";
        i += 1;
      } else {
        regex += "[^/]*";
      }
      continue;
    }

    if (char === "?") {
      regex += "[^/]";
      continue;
    }

    if (char === "/") {
      regex += "/";
      continue;
    }

    regex += escapeRegExp(char);
  }

  regex += "$";
  return new RegExp(regex);
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
