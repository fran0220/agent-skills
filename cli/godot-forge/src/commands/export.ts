/**
 * export command group — Export preset management and build execution.
 */

import { Command } from "commander";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { join, resolve } from "node:path";
import { execFileSync } from "node:child_process";
import { success, error, output } from "../utils/output.js";
import { readInput } from "../utils/input.js";
import { findGodot } from "../utils/subprocess.js";
import type { GlobalOptions } from "../types/index.js";

interface PresetInput {
  name: string;
  platform: string;
  path?: string;
  runnable?: boolean;
}

interface ParsedPreset {
  index: number;
  name: string;
  platform: string;
  runnable: boolean;
  export_path: string;
}

const PRESETS_FILE = "export_presets.cfg";

function parsePresets(content: string): ParsedPreset[] {
  const presets: ParsedPreset[] = [];
  const blocks = content.split(/\[preset\.(\d+)\]/);

  // blocks: ["preamble", "0", "...content...", "1", "...content...", ...]
  for (let i = 1; i < blocks.length; i += 2) {
    const index = parseInt(blocks[i], 10);
    const body = blocks[i + 1] ?? "";

    const nameMatch = body.match(/^name="(.+?)"/m);
    const platformMatch = body.match(/^platform="(.+?)"/m);
    const runnableMatch = body.match(/^runnable=(\w+)/m);
    const pathMatch = body.match(/^export_path="(.+?)"/m);

    presets.push({
      index,
      name: nameMatch?.[1] ?? "",
      platform: platformMatch?.[1] ?? "",
      runnable: runnableMatch?.[1] === "true",
      export_path: pathMatch?.[1] ?? "",
    });
  }

  return presets;
}

function formatPresetBlock(index: number, preset: PresetInput): string {
  const lines = [
    `[preset.${index}]`,
    `name="${preset.name}"`,
    `platform="${preset.platform}"`,
    `runnable=${preset.runnable ?? false}`,
    `export_path="${preset.path ?? ""}"`,
  ];
  return lines.join("\n") + "\n";
}

export function createExportCommand(): Command {
  const cmd = new Command("export").description(
    "Export preset management and build execution"
  );

  const preset = new Command("preset").description(
    "Manage export presets in export_presets.cfg"
  );

  preset
    .command("list")
    .description("List configured export presets")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "export.preset.list";
      const projectDir = resolve(globals.project ?? ".");
      const presetsPath = join(projectDir, PRESETS_FILE);

      if (!existsSync(presetsPath)) {
        output(
          error(
            commandName,
            "NO_PRESETS",
            `No ${PRESETS_FILE} found at ${projectDir}`,
            "Run 'godot-forge export preset add' to create one"
          ),
          globals.human
        );
        process.exit(1);
      }

      try {
        const content = readFileSync(presetsPath, "utf-8");
        const presets = parsePresets(content);

        output(
          success(commandName, { count: presets.length, presets }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(
          error(commandName, "PARSE_FAILED", `Failed to parse presets: ${msg}`),
          globals.human
        );
        process.exit(1);
      }
    });

  preset
    .command("add")
    .description("Add an export preset")
    .option("--name <name>", "Preset name")
    .option("--platform <platform>", "Target platform")
    .option("--path <path>", "Export output path")
    .option("--runnable", "Mark preset as runnable")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "export.preset.add";
      const projectDir = resolve(globals.project ?? ".");
      const presetsPath = join(projectDir, PRESETS_FILE);

      try {
        const input = await readInput<PresetInput>(globals.input);

        const presetName = opts.name ?? input?.name;
        const platform = opts.platform ?? input?.platform;
        const exportPath = opts.path ?? input?.path;
        const runnable = opts.runnable ?? input?.runnable ?? false;

        if (!presetName || !platform) {
          output(
            error(
              commandName,
              "MISSING_INPUT",
              "Preset name and platform are required",
              'Provide --name and --platform, or JSON input: {"name": "Linux", "platform": "Linux"}'
            ),
            globals.human
          );
          process.exit(1);
        }

        const presetInput: PresetInput = {
          name: presetName,
          platform,
          path: exportPath,
          runnable,
        };

        // Read existing presets or start fresh
        let existingContent = "";
        let nextIndex = 0;

        if (existsSync(presetsPath)) {
          existingContent = readFileSync(presetsPath, "utf-8");
          const existing = parsePresets(existingContent);

          // Check for duplicate name
          const duplicate = existing.find((p) => p.name === presetName);
          if (duplicate && !globals.force) {
            output(
              error(
                commandName,
                "PRESET_EXISTS",
                `Preset "${presetName}" already exists at index ${duplicate.index}`,
                "Use --force to overwrite"
              ),
              globals.human
            );
            process.exit(1);
          }

          if (duplicate && globals.force) {
            // Replace existing preset block
            const newBlock = formatPresetBlock(duplicate.index, presetInput);
            const regex = new RegExp(
              `\\[preset\\.${duplicate.index}\\][\\s\\S]*?(?=\\[preset\\.|$)`
            );
            const newContent = existingContent.replace(regex, newBlock + "\n");

            if (globals.dryRun) {
              output(
                success(commandName, {
                  action: "dry-run",
                  operation: "replace",
                  index: duplicate.index,
                  preset: presetInput,
                }),
                globals.human
              );
              return;
            }

            writeFileSync(presetsPath, newContent);
            output(
              success(commandName, {
                operation: "replaced",
                index: duplicate.index,
                preset: presetInput,
                file: presetsPath,
              }),
              globals.human
            );
            return;
          }

          nextIndex =
            existing.length > 0
              ? Math.max(...existing.map((p) => p.index)) + 1
              : 0;
        }

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              operation: "add",
              index: nextIndex,
              preset: presetInput,
            }),
            globals.human
          );
          return;
        }

        const block = formatPresetBlock(nextIndex, presetInput);
        const newContent = existingContent
          ? existingContent.trimEnd() + "\n\n" + block
          : block;

        writeFileSync(presetsPath, newContent);

        output(
          success(commandName, {
            operation: "added",
            index: nextIndex,
            preset: presetInput,
            file: presetsPath,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "ADD_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  cmd.addCommand(preset);

  cmd
    .command("build")
    .description("Execute the export/build process (requires Godot)")
    .option("--preset <name>", "Export preset name")
    .option("--output <path>", "Output path (overrides preset config)")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "export.build";
      const projectDir = resolve(globals.project ?? ".");
      const presetsPath = join(projectDir, PRESETS_FILE);

      try {
        const input = await readInput<{ preset: string; output?: string }>(
          globals.input
        );
        const presetName = opts.preset ?? input?.preset;
        const outputPath = opts.output ?? input?.output;

        if (!presetName) {
          output(
            error(
              commandName,
              "MISSING_PRESET",
              "Preset name is required",
              'Provide --preset <name> or JSON input: {"preset": "Linux"}'
            ),
            globals.human
          );
          process.exit(1);
        }

        // Read preset config to get output path if not provided
        let resolvedOutput = outputPath;
        if (!resolvedOutput) {
          if (!existsSync(presetsPath)) {
            output(
              error(
                commandName,
                "NO_PRESETS",
                `No ${PRESETS_FILE} found at ${projectDir}`,
                "Create presets first or provide --output path"
              ),
              globals.human
            );
            process.exit(1);
          }

          const content = readFileSync(presetsPath, "utf-8");
          const presets = parsePresets(content);
          const found = presets.find((p) => p.name === presetName);

          if (!found) {
            output(
              error(
                commandName,
                "PRESET_NOT_FOUND",
                `Preset "${presetName}" not found`,
                `Available presets: ${presets.map((p) => p.name).join(", ") || "none"}`
              ),
              globals.human
            );
            process.exit(1);
          }

          resolvedOutput = found.export_path;
        }

        if (!resolvedOutput) {
          output(
            error(
              commandName,
              "NO_OUTPUT_PATH",
              "No output path configured for this preset",
              "Provide --output <path> or set export_path in the preset"
            ),
            globals.human
          );
          process.exit(1);
        }

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              preset: presetName,
              output: resolvedOutput,
              godot_command: `godot --headless --export-release "${presetName}" ${resolvedOutput}`,
            }),
            globals.human
          );
          return;
        }

        // Find Godot and run export
        let godot: string;
        try {
          godot = findGodot();
        } catch (e) {
          const msg = e instanceof Error ? e.message : String(e);
          output(error(commandName, "GODOT_NOT_FOUND", msg), globals.human);
          process.exit(2);
        }

        try {
          const stdout = execFileSync(
            godot,
            [
              "--headless",
              "--export-release",
              presetName,
              resolvedOutput,
            ],
            {
              cwd: projectDir,
              stdio: "pipe",
              timeout: 300_000,
            }
          );

          output(
            success(commandName, {
              preset: presetName,
              output: resolvedOutput,
              log: stdout.toString(),
            }),
            globals.human
          );
        } catch (e: unknown) {
          const err = e as {
            stdout?: Buffer;
            stderr?: Buffer;
            status?: number;
          };
          const log =
            (err.stdout?.toString() ?? "") + (err.stderr?.toString() ?? "");
          output(
            error(
              commandName,
              "BUILD_FAILED",
              `Godot export failed with exit code ${err.status ?? "unknown"}`,
              log || "Check Godot logs for details"
            ),
            globals.human
          );
          process.exit(2);
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "BUILD_ERROR", msg), globals.human);
        process.exit(1);
      }
    });

  return cmd;
}
