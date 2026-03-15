/**
 * script command group — GDScript file management (L1).
 */

import { Command } from "commander";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { success, error, output, filterFields } from "../utils/output.js";
import { readInput } from "../utils/input.js";
import type {
  GlobalOptions,
  ScriptSpec,
  ScriptEditSpec,
  ScriptInfo,
} from "../types/index.js";

function toPascalCase(name: string): string {
  return name
    .replace(/\.gd$/, "")
    .split(/[_\-]/)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

function formatExportDefault(value: unknown, type: string): string {
  if (value === undefined || value === null) return "";
  if (type === "String" || type === "string") return ` = "${value}"`;
  return ` = ${value}`;
}

function generateScript(spec: ScriptSpec): string {
  const lines: string[] = [];
  const className = toPascalCase(spec.name);

  lines.push(`extends ${spec.extends}`);
  lines.push(`class_name ${className}`);

  if (spec.exports && spec.exports.length > 0) {
    lines.push("");
    lines.push("## Exported properties");
    for (const exp of spec.exports) {
      const def = formatExportDefault(exp.default, exp.type);
      lines.push(`@export var ${exp.name}: ${exp.type}${def}`);
    }
  }

  if (spec.signals && spec.signals.length > 0) {
    lines.push("");
    lines.push("## Signals");
    for (const sig of spec.signals) {
      lines.push(`signal ${sig}`);
    }
  }

  if (spec.methods && spec.methods.length > 0) {
    lines.push("");
    for (const method of spec.methods) {
      if (method === "_process" || method === "_physics_process") {
        lines.push(`func ${method}(delta: float) -> void:`);
      } else if (method === "_ready") {
        lines.push(`func ${method}() -> void:`);
      } else {
        lines.push(`func ${method}() -> void:`);
      }
      lines.push("\tpass");
      lines.push("");
    }
  }

  let result = lines.join("\n");
  if (result.endsWith("\n\n")) {
    result = result.slice(0, -1);
  }
  return result + "\n";
}

function findGdFiles(dir: string): string[] {
  const results: string[] = [];
  if (!existsSync(dir)) return results;

  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === ".godot" || entry.name === ".git" || entry.name === "node_modules") continue;
      results.push(...findGdFiles(fullPath));
    } else if (entry.name.endsWith(".gd")) {
      results.push(fullPath);
    }
  }
  return results;
}

function parseScriptInfo(filePath: string, projectDir: string): ScriptInfo {
  const content = readFileSync(filePath, "utf-8");
  const extendsMatch = content.match(/^extends\s+(\S+)/m);
  const classMatch = content.match(/^class_name\s+(\S+)/m);
  const relativePath = filePath.startsWith(projectDir)
    ? filePath.slice(projectDir.length + 1)
    : filePath;
  return {
    path: relativePath,
    extends: extendsMatch?.[1] ?? null,
    class_name: classMatch?.[1] ?? null,
  };
}

export function createScriptCommand(): Command {
  const cmd = new Command("script").description("GDScript file management");

  // ── script create ──────────────────────────────────────────────────
  cmd
    .command("create")
    .description("Generate a GDScript file from a ScriptSpec")
    .option("--name <name>", "Script name (snake_case)")
    .option("--extends <type>", "Base class to extend")
    .option("--path <path>", "Output file path")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "script.create";

      try {
        const input = await readInput<Partial<ScriptSpec>>(globals.input);
        const spec: ScriptSpec = {
          name: opts.name ?? input?.name ?? "",
          extends: opts.extends ?? input?.extends ?? "Node",
          path: opts.path ?? input?.path,
          signals: input?.signals,
          methods: input?.methods,
          exports: input?.exports,
        };

        if (!spec.name) {
          output(
            error(commandName, "MISSING_NAME", "Script name is required", "Pass --name <name> or provide {\"name\": \"...\"} via stdin"),
            globals.human
          );
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scriptPath = spec.path ?? `scripts/${spec.name}.gd`;
        const fullPath = resolve(projectDir, scriptPath);

        if (existsSync(fullPath) && !globals.force) {
          output(
            error(commandName, "FILE_EXISTS", `Script already exists at ${scriptPath}`, "Use --force to overwrite"),
            globals.human
          );
          process.exit(1);
        }

        const content = generateScript(spec);

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              path: scriptPath,
              class_name: toPascalCase(spec.name),
              content,
            }),
            globals.human
          );
          return;
        }

        mkdirSync(dirname(fullPath), { recursive: true });
        writeFileSync(fullPath, content);

        const result = {
          path: scriptPath,
          full_path: fullPath,
          class_name: toPascalCase(spec.name),
          extends: spec.extends,
          signals: spec.signals ?? [],
          methods: spec.methods ?? [],
          exports: spec.exports ?? [],
        };

        output(
          success(commandName, globals.fields ? filterFields(result, globals.fields) : result),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "CREATE_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── script list ────────────────────────────────────────────────────
  cmd
    .command("list")
    .description("List all .gd files in the project")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "script.list";
      const projectDir = resolve(globals.project ?? ".");

      try {
        if (!existsSync(projectDir)) {
          output(
            error(commandName, "DIR_NOT_FOUND", `Directory not found: ${projectDir}`, "Pass --project <path> or run from the project directory"),
            globals.human
          );
          process.exit(1);
        }

        const files = findGdFiles(projectDir);
        const scripts: ScriptInfo[] = files.map((f) => parseScriptInfo(f, projectDir));

        output(
          success(commandName, { count: scripts.length, scripts }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "LIST_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── script validate ────────────────────────────────────────────────
  cmd
    .command("validate")
    .description("Check a script for basic issues")
    .argument("<path>", "Path to the .gd file to validate")
    .action(async (scriptPath: string, _opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "script.validate";
      const projectDir = resolve(globals.project ?? ".");
      const fullPath = resolve(projectDir, scriptPath);

      try {
        if (!existsSync(fullPath)) {
          output(
            error(commandName, "FILE_NOT_FOUND", `Script not found: ${scriptPath}`),
            globals.human
          );
          process.exit(1);
        }

        const content = readFileSync(fullPath, "utf-8");
        const issues: { severity: string; message: string; line?: number }[] = [];

        // Check extends declaration
        const extendsMatch = content.match(/^extends\s+(\S+)/m);
        if (!extendsMatch) {
          issues.push({ severity: "error", message: "Missing 'extends' declaration" });
        }

        // Check class_name references
        const classNameMatch = content.match(/^class_name\s+(\S+)/m);
        const fileName = basename(fullPath, ".gd");
        const expectedClassName = toPascalCase(fileName);
        if (classNameMatch && classNameMatch[1] !== expectedClassName) {
          issues.push({
            severity: "warning",
            message: `class_name '${classNameMatch[1]}' does not match expected '${expectedClassName}' from file name`,
          });
        }

        // Check for empty functions (func ... with no body beyond pass)
        const lines = content.split("\n");
        for (let i = 0; i < lines.length; i++) {
          const line = lines[i];
          // Check for tabs vs spaces mixing
          if (line.match(/^( +\t|\t+ )/)) {
            issues.push({
              severity: "warning",
              message: "Mixed tabs and spaces indentation",
              line: i + 1,
            });
          }
        }

        const valid = issues.filter((i) => i.severity === "error").length === 0;

        output(
          success(commandName, {
            path: scriptPath,
            valid,
            issues,
          }),
          globals.human
        );

        if (!valid) {
          process.exit(1);
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "VALIDATE_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── script edit ────────────────────────────────────────────────────
  cmd
    .command("edit")
    .description("Update parts of an existing script")
    .option("--path <path>", "Path to the .gd file to edit")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "script.edit";

      try {
        const input = await readInput<Partial<ScriptEditSpec>>(globals.input);
        const scriptPath = opts.path ?? input?.path;

        if (!scriptPath) {
          output(
            error(commandName, "MISSING_PATH", "Script path is required", "Pass --path <path> or provide {\"path\": \"...\"} via stdin"),
            globals.human
          );
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const fullPath = resolve(projectDir, scriptPath);

        if (!existsSync(fullPath)) {
          output(
            error(commandName, "FILE_NOT_FOUND", `Script not found: ${scriptPath}`),
            globals.human
          );
          process.exit(1);
        }

        const addSignals = input?.add_signals ?? [];
        const addMethods = input?.add_methods ?? [];
        const addExports = input?.add_exports ?? [];

        if (addSignals.length === 0 && addMethods.length === 0 && addExports.length === 0) {
          output(
            error(commandName, "NOTHING_TO_DO", "No additions specified", "Provide add_signals, add_methods, or add_exports"),
            globals.human
          );
          process.exit(1);
        }

        let content = readFileSync(fullPath, "utf-8");
        const added: { signals: string[]; methods: string[]; exports: string[] } = {
          signals: [],
          methods: [],
          exports: [],
        };

        // Add exports after existing exports or after extends/class_name
        if (addExports.length > 0) {
          const exportLines: string[] = [];
          for (const exp of addExports) {
            const def = formatExportDefault(exp.default, exp.type);
            exportLines.push(`@export var ${exp.name}: ${exp.type}${def}`);
            added.exports.push(exp.name);
          }

          const lastExportIdx = content.lastIndexOf("@export var ");
          if (lastExportIdx !== -1) {
            const lineEnd = content.indexOf("\n", lastExportIdx);
            content = content.slice(0, lineEnd + 1) + exportLines.join("\n") + "\n" + content.slice(lineEnd + 1);
          } else {
            // Insert after class_name or extends
            const classLine = content.match(/^class_name\s+.+$/m);
            const extendsLine = content.match(/^extends\s+.+$/m);
            const anchor = classLine ?? extendsLine;
            if (anchor && anchor.index !== undefined) {
              const lineEnd = content.indexOf("\n", anchor.index);
              content = content.slice(0, lineEnd + 1) + "\n## Exported properties\n" + exportLines.join("\n") + "\n" + content.slice(lineEnd + 1);
            } else {
              content = "## Exported properties\n" + exportLines.join("\n") + "\n\n" + content;
            }
          }
        }

        // Add signals after existing signals or after exports
        if (addSignals.length > 0) {
          const signalLines: string[] = [];
          for (const sig of addSignals) {
            signalLines.push(`signal ${sig}`);
            added.signals.push(sig);
          }

          const lastSignalIdx = content.lastIndexOf("signal ");
          if (lastSignalIdx !== -1) {
            const lineEnd = content.indexOf("\n", lastSignalIdx);
            content = content.slice(0, lineEnd + 1) + signalLines.join("\n") + "\n" + content.slice(lineEnd + 1);
          } else {
            // Append before first func or at end
            const funcIdx = content.search(/^func\s/m);
            if (funcIdx !== -1) {
              content = content.slice(0, funcIdx) + "## Signals\n" + signalLines.join("\n") + "\n\n" + content.slice(funcIdx);
            } else {
              content = content.trimEnd() + "\n\n## Signals\n" + signalLines.join("\n") + "\n";
            }
          }
        }

        // Add methods at the end
        if (addMethods.length > 0) {
          const methodLines: string[] = [];
          for (const method of addMethods) {
            if (method === "_process" || method === "_physics_process") {
              methodLines.push(`func ${method}(delta: float) -> void:`);
            } else {
              methodLines.push(`func ${method}() -> void:`);
            }
            methodLines.push("\tpass");
            methodLines.push("");
            added.methods.push(method);
          }
          content = content.trimEnd() + "\n\n" + methodLines.join("\n");
        }

        if (!content.endsWith("\n")) {
          content += "\n";
        }

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              path: scriptPath,
              added,
              content,
            }),
            globals.human
          );
          return;
        }

        writeFileSync(fullPath, content);

        output(
          success(commandName, {
            path: scriptPath,
            added,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "EDIT_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  return cmd;
}
