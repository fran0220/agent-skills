/**
 * resource command group — Resource file management.
 */

import { Command } from "commander";
import {
  readFileSync,
  writeFileSync,
  existsSync,
  readdirSync,
  statSync,
  unlinkSync,
} from "node:fs";
import { join, resolve, relative, extname } from "node:path";
import { execFileSync } from "node:child_process";
import { success, error, output, filterFields } from "../utils/output.js";
import { readInput } from "../utils/input.js";
import { findGodot } from "../utils/subprocess.js";
import type { GlobalOptions } from "../types/index.js";

interface ResourceInput {
  type: string;
  name: string;
  path: string;
  properties?: Record<string, unknown>;
}

const TEXTURE_EXTS = new Set([".png", ".jpg", ".jpeg", ".svg", ".webp", ".bmp", ".tga"]);
const AUDIO_EXTS = new Set([".wav", ".ogg", ".mp3"]);

function formatTresValue(value: unknown): string {
  if (typeof value === "string") {
    // Godot built-in types like Color(...), Vector2(...) are passed as strings
    if (/^[A-Z]\w+\(.*\)$/.test(value)) {
      return value;
    }
    return `"${value}"`;
  }
  if (typeof value === "boolean") return value ? "true" : "false";
  return String(value);
}

function generateTres(input: ResourceInput): string {
  const lines = [`[gd_resource type="${input.type}" format=3]`, "", "[resource]"];

  if (input.properties) {
    for (const [key, val] of Object.entries(input.properties)) {
      lines.push(`${key} = ${formatTresValue(val)}`);
    }
  }

  return lines.join("\n") + "\n";
}

function walkDir(dir: string, projectDir: string): string[] {
  const results: string[] = [];

  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return results;
  }

  for (const entry of entries) {
    // Skip hidden dirs and .godot import cache
    if (entry.startsWith(".")) continue;

    const fullPath = join(dir, entry);
    let stat;
    try {
      stat = statSync(fullPath);
    } catch {
      continue;
    }

    if (stat.isDirectory()) {
      results.push(...walkDir(fullPath, projectDir));
    } else {
      const rel = relative(projectDir, fullPath).replace(/\\/g, "/");
      results.push(`res://${rel}`);
    }
  }

  return results;
}

function categorizeFiles(files: string[]): Record<string, string[]> {
  const categories: Record<string, string[]> = {
    scenes: [],
    scripts: [],
    resources: [],
    textures: [],
    audio: [],
  };

  for (const file of files) {
    const ext = extname(file).toLowerCase();
    if (ext === ".tscn") {
      categories.scenes.push(file);
    } else if (ext === ".gd" || ext === ".cs") {
      categories.scripts.push(file);
    } else if (ext === ".tres") {
      categories.resources.push(file);
    } else if (TEXTURE_EXTS.has(ext)) {
      categories.textures.push(file);
    } else if (AUDIO_EXTS.has(ext)) {
      categories.audio.push(file);
    }
  }

  return categories;
}

interface TresResource {
  type: string;
  format: number;
  uid?: string;
  properties: Record<string, unknown>;
  ext_resources: Array<{ type: string; path: string; id: string; uid?: string }>;
  sub_resources: Array<{ type: string; id: string; properties: Record<string, unknown> }>;
}

function parseSectionHeader(line: string): { kind: string; attrs: Record<string, string> } | null {
  const m = line.match(/^\[(\w+)\s*(.*)\]$/);
  if (!m) return null;
  const kind = m[1];
  const raw = m[2].trim();
  const attrs: Record<string, string> = {};
  const re = /(\w+)=(\"[^\"]*\"|[^\s]+)/g;
  let match: RegExpExecArray | null;
  while ((match = re.exec(raw)) !== null) {
    let val = match[2];
    if (val.startsWith('"') && val.endsWith('"')) val = val.slice(1, -1);
    attrs[match[1]] = val;
  }
  return { kind, attrs };
}

function parseTresPropertyValue(raw: string): unknown {
  const trimmed = raw.trim();
  if (trimmed.startsWith('"') && trimmed.endsWith('"')) return trimmed.slice(1, -1);
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;
  if (trimmed === "null") return null;
  if (/^-?\d+(\.\d+)?$/.test(trimmed)) {
    return trimmed.includes(".") ? parseFloat(trimmed) : parseInt(trimmed, 10);
  }
  return trimmed;
}

function parseTres(content: string): TresResource {
  const lines = content.split("\n");
  const result: TresResource = {
    type: "",
    format: 3,
    properties: {},
    ext_resources: [],
    sub_resources: [],
  };

  let currentSection: { kind: string; attrs: Record<string, string> } | null = null;
  let currentProperties: Record<string, unknown> = {};

  function flushSection(): void {
    if (!currentSection) return;
    switch (currentSection.kind) {
      case "gd_resource":
        result.type = currentSection.attrs.type ?? "";
        if (currentSection.attrs.format) result.format = parseInt(currentSection.attrs.format, 10);
        if (currentSection.attrs.uid) result.uid = currentSection.attrs.uid;
        break;
      case "ext_resource": {
        const ext: TresResource["ext_resources"][number] = {
          type: currentSection.attrs.type ?? "",
          path: currentSection.attrs.path ?? "",
          id: currentSection.attrs.id ?? "",
        };
        if (currentSection.attrs.uid) ext.uid = currentSection.attrs.uid;
        result.ext_resources.push(ext);
        break;
      }
      case "sub_resource": {
        result.sub_resources.push({
          type: currentSection.attrs.type ?? "",
          id: currentSection.attrs.id ?? "",
          properties: { ...currentProperties },
        });
        break;
      }
      case "resource":
        Object.assign(result.properties, currentProperties);
        break;
    }
    currentProperties = {};
  }

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed === "" || trimmed.startsWith(";")) continue;

    if (trimmed.startsWith("[")) {
      flushSection();
      currentSection = parseSectionHeader(trimmed);
      continue;
    }

    const eqIdx = trimmed.indexOf("=");
    if (eqIdx > 0) {
      const key = trimmed.slice(0, eqIdx).trimEnd();
      const val = trimmed.slice(eqIdx + 1).trimStart();
      if (/^[\w/]+$/.test(key)) {
        currentProperties[key] = parseTresPropertyValue(val);
      }
    }
  }
  flushSection();
  return result;
}

function generateTresContent(res: TresResource): string {
  const lines: string[] = [];

  // Header
  const headerAttrs = [`type="${res.type}"`, `format=${res.format}`];
  if (res.uid) headerAttrs.push(`uid="${res.uid}"`);
  lines.push(`[gd_resource ${headerAttrs.join(" ")}]`);
  lines.push("");

  // External resources
  for (const ext of res.ext_resources) {
    const attrs = [`type="${ext.type}"`, `path="${ext.path}"`, `id="${ext.id}"`];
    if (ext.uid) attrs.splice(2, 0, `uid="${ext.uid}"`);
    lines.push(`[ext_resource ${attrs.join(" ")}]`);
  }
  if (res.ext_resources.length > 0) lines.push("");

  // Sub resources
  for (const sub of res.sub_resources) {
    lines.push(`[sub_resource type="${sub.type}" id="${sub.id}"]`);
    for (const [key, val] of Object.entries(sub.properties)) {
      lines.push(`${key} = ${formatTresValue(val)}`);
    }
    lines.push("");
  }

  // Resource properties
  lines.push("[resource]");
  for (const [key, val] of Object.entries(res.properties)) {
    lines.push(`${key} = ${formatTresValue(val)}`);
  }
  lines.push("");

  return lines.join("\n");
}

export function createResourceCommand(): Command {
  const cmd = new Command("resource").description("Resource file management");

  cmd
    .command("create")
    .description("Create a Godot resource file (.tres)")
    .option("--type <type>", "Resource type (e.g., Environment)")
    .option("--name <name>", "Resource name")
    .option("--path <path>", "Output file path")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "resource.create";
      const projectDir = resolve(globals.project ?? ".");

      try {
        const input = await readInput<ResourceInput>(globals.input);

        const resType = opts.type ?? input?.type;
        const resName = opts.name ?? input?.name;
        const resPath = opts.path ?? input?.path;
        const properties = input?.properties;

        if (!resType || !resName || !resPath) {
          output(
            error(
              commandName,
              "MISSING_INPUT",
              "Resource type, name, and path are required",
              'Provide --type, --name, --path or JSON input: {"type": "Environment", "name": "env", "path": "resources/env.tres"}'
            ),
            globals.human
          );
          process.exit(1);
        }

        const fullPath = resolve(projectDir, resPath);

        if (existsSync(fullPath) && !globals.force) {
          output(
            error(
              commandName,
              "FILE_EXISTS",
              `Resource file already exists at ${fullPath}`,
              "Use --force to overwrite"
            ),
            globals.human
          );
          process.exit(1);
        }

        const resourceInput: ResourceInput = {
          type: resType,
          name: resName,
          path: resPath,
          properties,
        };

        const content = generateTres(resourceInput);

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              file: fullPath,
              content,
            }),
            globals.human
          );
          return;
        }

        // Ensure parent directory exists
        const { mkdirSync } = await import("node:fs");
        const { dirname } = await import("node:path");
        mkdirSync(dirname(fullPath), { recursive: true });

        writeFileSync(fullPath, content);

        output(
          success(commandName, {
            file: fullPath,
            type: resType,
            name: resName,
            properties: properties ?? {},
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "CREATE_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  cmd
    .command("list")
    .description("List all resource files in the project")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "resource.list";
      const projectDir = resolve(globals.project ?? ".");

      if (!existsSync(join(projectDir, "project.godot"))) {
        output(
          error(
            commandName,
            "NOT_A_PROJECT",
            `No project.godot found at ${projectDir}`,
            "Run 'godot-forge project init' first or pass --project <path>"
          ),
          globals.human
        );
        process.exit(1);
      }

      try {
        const allFiles = walkDir(projectDir, projectDir);
        const categorized = categorizeFiles(allFiles);

        output(success(commandName, categorized), globals.human);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "LIST_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  cmd
    .command("import")
    .description("Trigger Godot import pipeline for resources (requires Godot)")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "resource.import";
      const projectDir = resolve(globals.project ?? ".");

      if (globals.dryRun) {
        output(
          success(commandName, {
            action: "dry-run",
            godot_command: "godot --headless --import",
          }),
          globals.human
        );
        return;
      }

      let godot: string;
      try {
        godot = findGodot();
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "GODOT_NOT_FOUND", msg), globals.human);
        process.exit(2);
      }

      try {
        const stdout = execFileSync(godot, ["--headless", "--import"], {
          cwd: projectDir,
          stdio: "pipe",
          timeout: 120_000,
        });

        output(
          success(commandName, {
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
            "IMPORT_FAILED",
            `Godot import failed with exit code ${err.status ?? "unknown"}`,
            log || "Check Godot logs for details"
          ),
          globals.human
        );
        process.exit(2);
      }
    });

  cmd
    .command("check")
    .description("Check for missing or broken resource references")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "resource.check";
      const projectDir = resolve(globals.project ?? ".");

      if (!existsSync(join(projectDir, "project.godot"))) {
        output(
          error(
            commandName,
            "NOT_A_PROJECT",
            `No project.godot found at ${projectDir}`,
            "Run 'godot-forge project init' first or pass --project <path>"
          ),
          globals.human
        );
        process.exit(1);
      }

      try {
        // Find all .tscn and .tres files
        const allFiles = walkDir(projectDir, projectDir);
        const sceneFiles = allFiles.filter(
          (f) => f.endsWith(".tscn") || f.endsWith(".tres")
        );

        const missing: Array<{ source: string; reference: string }> = [];
        const checked = 0;
        let totalRefs = 0;

        for (const resPath of sceneFiles) {
          // Convert res:// path back to filesystem path
          const relPath = resPath.replace("res://", "");
          const fullPath = join(projectDir, relPath);

          let content: string;
          try {
            content = readFileSync(fullPath, "utf-8");
          } catch {
            continue;
          }

          // Match ext_resource patterns: path="res://..." or path="..."
          const refPattern = /ext_resource\s+[^]]*?path="(res:\/\/[^"]+)"/g;
          let match;
          while ((match = refPattern.exec(content)) !== null) {
            totalRefs++;
            const refResPath = match[1];
            const refRelPath = refResPath.replace("res://", "");
            const refFullPath = join(projectDir, refRelPath);

            if (!existsSync(refFullPath)) {
              missing.push({
                source: resPath,
                reference: refResPath,
              });
            }
          }
        }

        output(
          success(commandName, {
            files_scanned: sceneFiles.length,
            references_checked: totalRefs,
            missing_count: missing.length,
            missing,
          }),
          globals.human
        );

        if (missing.length > 0) {
          process.exit(1);
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "CHECK_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── resource read ──────────────────────────────────────

  cmd
    .command("read")
    .description("Read a resource file and return as JSON")
    .argument("<path>", "Path to .tres file")
    .action(async (filePath: string, _opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "resource.read";
      const projectDir = resolve(globals.project ?? ".");

      try {
        const fullPath = resolve(projectDir, filePath);

        if (!existsSync(fullPath)) {
          output(
            error(commandName, "FILE_NOT_FOUND", `Resource file not found: ${fullPath}`, "Check the file path"),
            globals.human
          );
          process.exit(1);
        }

        const content = readFileSync(fullPath, "utf-8");
        const res = parseTres(content);

        const result = {
          path: fullPath,
          res_path: `res://${relative(projectDir, fullPath)}`,
          type: res.type,
          format: res.format,
          uid: res.uid,
          properties: res.properties,
          ext_resources: res.ext_resources,
          sub_resources: res.sub_resources,
        };

        output(
          success(commandName, globals.fields ? filterFields(result, globals.fields) : result),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "READ_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── resource update ──────────────────────────────────────

  cmd
    .command("update")
    .description("Update properties of an existing resource")
    .option("--path <path>", "Resource file path")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "resource.update";
      const projectDir = resolve(globals.project ?? ".");

      try {
        interface ResourceUpdateInput {
          path: string;
          properties: Record<string, unknown>;
        }

        const input = await readInput<ResourceUpdateInput>(globals.input);
        const filePath = opts.path ?? input?.path;
        const properties = input?.properties;

        if (!filePath) {
          output(
            error(commandName, "MISSING_PATH", "Resource file path is required", 'Pass --path <path> or provide {"path": "..."} via stdin'),
            globals.human
          );
          process.exit(1);
        }

        if (!properties || Object.keys(properties).length === 0) {
          output(
            error(commandName, "MISSING_PROPERTIES", "Properties to update are required", 'Provide {"properties": {"key": "value"}} via stdin'),
            globals.human
          );
          process.exit(1);
        }

        const fullPath = resolve(projectDir, filePath);

        if (!existsSync(fullPath)) {
          output(
            error(commandName, "FILE_NOT_FOUND", `Resource file not found: ${fullPath}`, "Check the file path"),
            globals.human
          );
          process.exit(1);
        }

        const content = readFileSync(fullPath, "utf-8");
        const res = parseTres(content);

        // Merge new properties
        const updatedKeys = Object.keys(properties);
        Object.assign(res.properties, properties);

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              path: fullPath,
              updated_keys: updatedKeys,
              properties: res.properties,
            }),
            globals.human
          );
          return;
        }

        writeFileSync(fullPath, generateTresContent(res));

        output(
          success(commandName, {
            path: fullPath,
            res_path: `res://${relative(projectDir, fullPath)}`,
            type: res.type,
            updated_keys: updatedKeys,
            properties: res.properties,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "UPDATE_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── resource delete ──────────────────────────────────────

  cmd
    .command("delete")
    .description("Delete a resource file (requires --force)")
    .argument("<path>", "Path to the resource file")
    .action(async (filePath: string, _opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "resource.delete";
      const projectDir = resolve(globals.project ?? ".");

      try {
        const fullPath = resolve(projectDir, filePath);
        const resPath = `res://${relative(projectDir, fullPath)}`;

        if (!existsSync(fullPath)) {
          output(
            error(commandName, "FILE_NOT_FOUND", `Resource file not found: ${fullPath}`, "Check the file path"),
            globals.human
          );
          process.exit(1);
        }

        if (!globals.force) {
          output(
            error(
              commandName,
              "FORCE_REQUIRED",
              "Deleting a resource requires --force. Pass --force to confirm deletion"
            ),
            globals.human
          );
          process.exit(1);
        }

        // Check how many .tscn/.tres files reference this resource
        const allFiles = walkDir(projectDir, projectDir);
        const sceneFiles = allFiles.filter((f) => f.endsWith(".tscn") || f.endsWith(".tres"));
        let referencedBy = 0;

        for (const sf of sceneFiles) {
          const relPath = sf.replace("res://", "");
          const sfFullPath = join(projectDir, relPath);
          if (sfFullPath === fullPath) continue;

          let content: string;
          try {
            content = readFileSync(sfFullPath, "utf-8");
          } catch {
            continue;
          }

          if (content.includes(resPath)) {
            referencedBy++;
          }
        }

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              path: fullPath,
              res_path: resPath,
              deleted: false,
              referenced_by: referencedBy,
            }),
            globals.human
          );
          return;
        }

        unlinkSync(fullPath);

        output(
          success(commandName, {
            path: fullPath,
            res_path: resPath,
            deleted: true,
            referenced_by: referencedBy,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "DELETE_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  return cmd;
}
