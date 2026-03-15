/**
 * engine command group — Godot engine operations (L3: requires Godot binary).
 */

import { Command } from "commander";
import { existsSync, readFileSync, readdirSync, statSync, writeFileSync, unlinkSync, mkdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync, spawn } from "node:child_process";
import { success, error, output, filterFields } from "../utils/output.js";
import { readInput } from "../utils/input.js";
import { findGodot, runGodotScript } from "../utils/subprocess.js";
import type { GlobalOptions } from "../types/index.js";

function requireGodot(commandName: string, human?: boolean): string {
  try {
    return findGodot();
  } catch {
    output(
      error(
        commandName,
        "GODOT_NOT_FOUND",
        "Godot engine not found in PATH",
        "Install Godot 4.4+ and ensure 'godot' is in PATH. Engine commands require the Godot binary."
      ),
      human
    );
    process.exit(2);
  }
}

function requireProject(projectDir: string, commandName: string, human?: boolean): void {
  if (!existsSync(join(projectDir, "project.godot"))) {
    output(
      error(
        commandName,
        "NOT_A_PROJECT",
        `No project.godot found at ${projectDir}`,
        "Run 'godot-forge project init' first or pass --project <path>"
      ),
      human
    );
    process.exit(1);
  }
}

function writeTempScript(name: string, content: string): string {
  const dir = join(tmpdir(), "godot-forge");
  mkdirSync(dir, { recursive: true });
  const path = join(dir, `${name}_${Date.now()}.gd`);
  writeFileSync(path, content);
  return path;
}

function cleanupTempScript(path: string): void {
  try {
    unlinkSync(path);
  } catch {
    // ignore cleanup errors
  }
}

function collectSceneRefs(projectDir: string): string[] {
  const refs: string[] = [];
  const projectContent = readFileSync(join(projectDir, "project.godot"), "utf-8");
  const scenePattern = /="res:\/\/(.+?\.tscn)"/g;
  let match: RegExpExecArray | null;
  while ((match = scenePattern.exec(projectContent)) !== null) {
    refs.push(match[1]);
  }
  return refs;
}

function scanFiles(dir: string, ext: string, results: string[] = []): string[] {
  if (!existsSync(dir)) return results;
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (entry === ".godot" || entry === ".git") continue;
    const stat = statSync(full);
    if (stat.isDirectory()) {
      scanFiles(full, ext, results);
    } else if (entry.endsWith(ext)) {
      results.push(full);
    }
  }
  return results;
}

export function createEngineCommand(): Command {
  const cmd = new Command("engine").description(
    "Godot engine operations (requires Godot binary)"
  );

  // engine editor
  cmd
    .command("editor")
    .description("Open the project in Godot editor")
    .action((_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "engine.editor";
      const projectDir = resolve(globals.project ?? ".");

      requireProject(projectDir, commandName, globals.human);
      const godot = requireGodot(commandName, globals.human);

      const child = spawn(godot, ["--editor", "--path", projectDir], {
        detached: true,
        stdio: "ignore",
      });
      child.unref();

      output(
        success(commandName, {
          pid: child.pid,
          project: projectDir,
          message: "Godot editor launched. File changes from CLI will auto-reload.",
        }),
        globals.human
      );
    });

  // engine validate
  cmd
    .command("validate")
    .description("Validate project structure and resources")
    .option("--deep", "Run deep validation via Godot engine")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "engine.validate";
      const projectDir = resolve(globals.project ?? ".");

      requireProject(projectDir, commandName, globals.human);

      try {
        const issues: { level: string; path: string; message: string }[] = [];

        // Basic file structure checks (no Godot needed)
        const scenesDir = join(projectDir, "scenes");
        const scriptsDir = join(projectDir, "scripts");

        if (!existsSync(scenesDir)) {
          issues.push({ level: "warning", path: "scenes/", message: "Missing scenes directory" });
        }
        if (!existsSync(scriptsDir)) {
          issues.push({ level: "warning", path: "scripts/", message: "Missing scripts directory" });
        }

        // Check scene references from project.godot
        const sceneRefs = collectSceneRefs(projectDir);
        for (const ref of sceneRefs) {
          if (!existsSync(join(projectDir, ref))) {
            issues.push({ level: "error", path: ref, message: "Referenced scene file not found" });
          }
        }

        // Check all .tscn files for broken resource refs
        const scenes = scanFiles(projectDir, ".tscn");
        for (const scene of scenes) {
          const content = readFileSync(scene, "utf-8");
          const extResPattern = /path="res:\/\/(.+?)"/g;
          let refMatch: RegExpExecArray | null;
          while ((refMatch = extResPattern.exec(content)) !== null) {
            const refPath = refMatch[1];
            if (!existsSync(join(projectDir, refPath))) {
              const relScene = scene.slice(projectDir.length + 1);
              issues.push({
                level: "error",
                path: relScene,
                message: `Broken resource reference: res://${refPath}`,
              });
            }
          }
        }

        // Deep validation with Godot
        let deepResult: { import_errors: string[] } | undefined;
        if (opts.deep) {
          const godot = requireGodot(commandName, globals.human);

          const script = `extends SceneTree

func _init():
	var errors := []
	var dir := DirAccess.open("res://")
	if dir:
		_scan_dir(dir, "", errors)
	var result := {"import_errors": errors}
	print("GODOT_FORGE_JSON:" + JSON.stringify(result))
	quit()

func _scan_dir(dir: DirAccess, path: String, errors: Array) -> void:
	dir.list_dir_begin()
	var file_name := dir.get_next()
	while file_name != "":
		if not dir.current_is_dir():
			var full_path := path + file_name
			if full_path.ends_with(".tscn") or full_path.ends_with(".tres"):
				var res = load("res://" + full_path)
				if res == null:
					errors.append(full_path)
		else:
			if file_name != ".godot" and file_name != ".git":
				var sub_dir := DirAccess.open("res://" + path + file_name)
				if sub_dir:
					_scan_dir(sub_dir, path + file_name + "/", errors)
		file_name = dir.get_next()
	dir.list_dir_end()
`;
          const scriptPath = writeTempScript("validate", script);
          try {
            const res = runGodotScript(scriptPath, projectDir);
            const jsonMatch = res.stdout.match(/GODOT_FORGE_JSON:(.+)/);
            if (jsonMatch) {
              deepResult = JSON.parse(jsonMatch[1]);
              if (deepResult?.import_errors) {
                for (const importErr of deepResult.import_errors) {
                  issues.push({ level: "error", path: importErr, message: "Failed to load resource" });
                }
              }
            }
            if (res.exitCode !== 0 && !jsonMatch) {
              issues.push({ level: "warning", path: "godot", message: "Deep validation exited with errors" });
            }
          } finally {
            cleanupTempScript(scriptPath);
          }
        }

        const result = {
          project: projectDir,
          valid: issues.filter((i) => i.level === "error").length === 0,
          issues,
          scenes_checked: scenes.length,
          deep: opts.deep ?? false,
        };

        output(
          success(commandName, globals.fields ? filterFields(result, globals.fields) : result),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "VALIDATE_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // engine run
  cmd
    .command("run")
    .description("Run the game project via Godot")
    .option("--scene <path>", "Run a specific scene (res:// path)")
    .option("--headless", "Run in headless mode")
    .option("--timeout <ms>", "Execution timeout in milliseconds", "30000")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "engine.run";
      const projectDir = resolve(globals.project ?? ".");

      requireProject(projectDir, commandName, globals.human);
      const godot = requireGodot(commandName, globals.human);

      try {
        const args = ["--path", projectDir];

        if (opts.headless) {
          args.push("--headless");
        }

        if (opts.scene) {
          args.push("--scene", opts.scene);
        }

        const timeout = parseInt(opts.timeout, 10);

        const res = spawnSync(godot, args, {
          cwd: projectDir,
          stdio: "pipe",
          timeout,
        });

        const stdout = res.stdout?.toString() ?? "";
        const stderr = res.stderr?.toString() ?? "";

        if (res.status !== 0 && res.status !== null) {
          output(
            error(
              commandName,
              "RUN_FAILED",
              `Godot exited with code ${res.status}`,
              stderr || undefined
            ),
            globals.human
          );
          process.exit(2);
        }

        if (res.error) {
          const errMsg = res.error.message.includes("ETIMEDOUT")
            ? `Execution timed out after ${timeout}ms`
            : res.error.message;
          output(
            error(commandName, "RUN_ERROR", errMsg),
            globals.human
          );
          process.exit(2);
        }

        const result = {
          project: projectDir,
          scene: opts.scene ?? "main",
          headless: opts.headless ?? false,
          exit_code: res.status ?? 0,
          stdout: stdout.trim() || undefined,
        };

        output(
          success(commandName, globals.fields ? filterFields(result, globals.fields) : result),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "RUN_FAILED", msg), globals.human);
        process.exit(2);
      }
    });

  // engine import
  cmd
    .command("import")
    .description("Trigger Godot resource import pipeline")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "engine.import";
      const projectDir = resolve(globals.project ?? ".");

      requireProject(projectDir, commandName, globals.human);
      const godot = requireGodot(commandName, globals.human);

      try {
        const res = spawnSync(godot, ["--headless", "--import", "--path", projectDir], {
          cwd: projectDir,
          stdio: "pipe",
          timeout: 120_000,
        });

        const stdout = res.stdout?.toString() ?? "";
        const stderr = res.stderr?.toString() ?? "";

        if (res.error) {
          output(
            error(commandName, "IMPORT_ERROR", res.error.message),
            globals.human
          );
          process.exit(2);
        }

        if (res.status !== 0 && res.status !== null) {
          output(
            error(
              commandName,
              "IMPORT_FAILED",
              `Godot import exited with code ${res.status}`,
              stderr || undefined
            ),
            globals.human
          );
          process.exit(2);
        }

        const importedDir = join(projectDir, ".godot", "imported");
        const hasImported = existsSync(importedDir);

        const result = {
          project: projectDir,
          success: true,
          imported_cache: hasImported,
          stdout: stdout.trim() || undefined,
        };

        output(
          success(commandName, globals.fields ? filterFields(result, globals.fields) : result),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "IMPORT_FAILED", msg), globals.human);
        process.exit(2);
      }
    });

  // engine uid
  cmd
    .command("uid")
    .description("Scan and report resource UID assignments")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "engine.uid";
      const projectDir = resolve(globals.project ?? ".");

      requireProject(projectDir, commandName, globals.human);
      requireGodot(commandName, globals.human);

      const script = `extends SceneTree

func _init():
	var uid_map := {}
	var dir := DirAccess.open("res://")
	if dir:
		_scan_dir(dir, "", uid_map)
	var result := {"uids": uid_map, "count": uid_map.size()}
	print("GODOT_FORGE_JSON:" + JSON.stringify(result))
	quit()

func _scan_dir(dir: DirAccess, path: String, uid_map: Dictionary) -> void:
	dir.list_dir_begin()
	var file_name := dir.get_next()
	while file_name != "":
		if not dir.current_is_dir():
			var full_path := "res://" + path + file_name
			if file_name.ends_with(".tscn") or file_name.ends_with(".tres") or file_name.ends_with(".gd"):
				var uid := ResourceLoader.get_resource_uid(full_path)
				if uid != -1:
					uid_map[path + file_name] = uid
		else:
			if file_name != ".godot" and file_name != ".git":
				var sub_dir := DirAccess.open("res://" + path + file_name)
				if sub_dir:
					_scan_dir(sub_dir, path + file_name + "/", uid_map)
		file_name = dir.get_next()
	dir.list_dir_end()
`;

      const scriptPath = writeTempScript("uid_scan", script);
      try {
        const res = runGodotScript(scriptPath, projectDir);
        const jsonMatch = res.stdout.match(/GODOT_FORGE_JSON:(.+)/);

        if (!jsonMatch) {
          output(
            error(
              commandName,
              "UID_SCAN_FAILED",
              "Failed to parse UID scan output from Godot",
              res.stdout.trim() || undefined
            ),
            globals.human
          );
          process.exit(2);
        }

        const parsed = JSON.parse(jsonMatch[1]) as { uids: Record<string, number>; count: number };

        const result = {
          project: projectDir,
          uids: parsed.uids,
          count: parsed.count,
        };

        output(
          success(commandName, globals.fields ? filterFields(result, globals.fields) : result),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "UID_SCAN_FAILED", msg), globals.human);
        process.exit(2);
      } finally {
        cleanupTempScript(scriptPath);
      }
    });

  // engine preview
  cmd
    .command("preview")
    .description("Render a screenshot of a scene")
    .option("--scene <path>", "Scene path (res:// format)")
    .option("--output <path>", "Output PNG file path", "preview.png")
    .option("--width <px>", "Preview width", "1280")
    .option("--height <px>", "Preview height", "720")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "engine.preview";
      const projectDir = resolve(globals.project ?? ".");

      requireProject(projectDir, commandName, globals.human);
      requireGodot(commandName, globals.human);

      try {
        interface PreviewInput {
          scene?: string;
          output?: string;
          size?: [number, number];
        }

        const input = await readInput<PreviewInput>(globals.input);
        const scene = opts.scene ?? input?.scene;
        const outputPath = resolve(opts.output ?? input?.output ?? "preview.png");
        const width = input?.size?.[0] ?? parseInt(opts.width, 10);
        const height = input?.size?.[1] ?? parseInt(opts.height, 10);

        if (!scene) {
          output(
            error(
              commandName,
              "MISSING_SCENE",
              "Scene path is required",
              "Pass --scene <res://path/to/scene.tscn> or provide {\"scene\": \"...\"} via stdin"
            ),
            globals.human
          );
          process.exit(1);
        }

        const script = `extends SceneTree

func _init():
	var scene_res = load("${scene}")
	if scene_res == null:
		print("GODOT_FORGE_JSON:" + JSON.stringify({"error": "Failed to load scene: ${scene}"}))
		quit()
		return

	var viewport := SubViewport.new()
	viewport.size = Vector2i(${width}, ${height})
	viewport.render_target_update_mode = SubViewport.UPDATE_ONCE
	viewport.transparent_bg = false

	root.add_child(viewport)

	var instance = scene_res.instantiate()
	viewport.add_child(instance)

	await get_tree().process_frame
	await get_tree().process_frame

	var image := viewport.get_texture().get_image()
	var err := image.save_png("${outputPath}")

	var result := {}
	if err == OK:
		result = {"output": "${outputPath}", "width": ${width}, "height": ${height}, "scene": "${scene}"}
	else:
		result = {"error": "Failed to save PNG, error code: " + str(err)}

	print("GODOT_FORGE_JSON:" + JSON.stringify(result))
	quit()
`;

        const scriptPath = writeTempScript("preview", script);
        try {
          const res = runGodotScript(scriptPath, projectDir);
          const jsonMatch = res.stdout.match(/GODOT_FORGE_JSON:(.+)/);

          if (!jsonMatch) {
            output(
              error(
                commandName,
                "PREVIEW_FAILED",
                "Failed to parse preview output from Godot",
                res.stdout.trim() || undefined
              ),
              globals.human
            );
            process.exit(2);
          }

          const parsed = JSON.parse(jsonMatch[1]) as Record<string, unknown>;

          if (parsed.error) {
            output(
              error(commandName, "PREVIEW_FAILED", parsed.error as string),
              globals.human
            );
            process.exit(2);
          }

          output(
            success(commandName, globals.fields ? filterFields(parsed, globals.fields) : parsed),
            globals.human
          );
        } finally {
          cleanupTempScript(scriptPath);
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "PREVIEW_FAILED", msg), globals.human);
        process.exit(2);
      }
    });

  return cmd;
}
