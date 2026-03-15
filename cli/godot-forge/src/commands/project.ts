/**
 * project command group — Project lifecycle management (L1).
 */

import { Command } from "commander";
import { mkdirSync, writeFileSync, existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { success, error, output, filterFields } from "../utils/output.js";
import { readInput } from "../utils/input.js";
import type { GlobalOptions, ProjectConfig } from "../types/index.js";
import {
  parseGodotConfig,
  generateGodotConfig,
  configGet,
  configSet,
  configUnset,
  configListSection,
  configListSections,
} from "../core/parser/godot-config.js";

// ── Templates ──────────────────────────────────────────────

const GODOT_PROJECT_FILE = `; Engine configuration file.
; It's best edited using the editor UI and not directly,
; since the parameters that go here are not all obvious.
;
; Format:
;   [section] ; section goes between []
;   param=value ; assign values to parameters

config_version=5

[application]

config/name="{name}"
config/features=PackedStringArray("4.4")

[editor_plugins]

enabled=PackedStringArray("res://addons/forge_sync/plugin.cfg")

[rendering]

renderer/rendering_method="mobile"
`;

const DEFAULT_GITIGNORE = `# Godot 4+ specific ignores
.godot/

# Godot-specific ignores
*.translation

# Imported textures and samples
.import/

# Build exports
export/

# godot-forge CLI bridge
.godot-forge/
`;

const FORGE_SYNC_PLUGIN_CFG = `[plugin]

name="ForgeSync"
description="Auto-scan filesystem for godot-forge CLI changes"
author="godot-forge"
version="0.1"
script="forge_sync.gd"
`;

const FORGE_SYNC_GD = `@tool
extends EditorPlugin

const COMMAND_FILE = "res://.godot-forge/commands.json"
var _timer: Timer
var _last_mtime: int = 0

func _enter_tree() -> void:
\tDirAccess.make_dir_recursive_absolute("res://.godot-forge")
\t_timer = Timer.new()
\t_timer.wait_time = 0.5
\t_timer.timeout.connect(_poll_commands)
\tadd_child(_timer)
\t_timer.start()
\tprint("[ForgeSync] Bridge active")

func _exit_tree() -> void:
\tif _timer:
\t\t_timer.stop()
\t\t_timer.queue_free()

func _poll_commands() -> void:
\tEditorInterface.get_resource_filesystem().scan()
\tvar path = ProjectSettings.globalize_path(COMMAND_FILE)
\tif not FileAccess.file_exists(path):
\t\treturn
\tvar mtime = FileAccess.get_modified_time(path)
\tif mtime == _last_mtime:
\t\treturn
\t_last_mtime = mtime
\tvar file = FileAccess.open(path, FileAccess.READ)
\tif not file:
\t\treturn
\tvar text = file.get_as_text()
\tfile.close()
\tvar json = JSON.new()
\tif json.parse(text) != OK:
\t\treturn
\tvar data = json.data
\tif data is Dictionary:
\t\t_execute(data)
\telif data is Array:
\t\tfor cmd in data:
\t\t\tif cmd is Dictionary:
\t\t\t\t_execute(cmd)
\tDirAccess.remove_absolute(path)

func _execute(cmd: Dictionary) -> void:
\tvar action = cmd.get("action", "")
\tprint("[ForgeSync] Executing: ", action)
\tmatch action:
\t\t"open_scene":
\t\t\tvar scene_path = cmd.get("path", "")
\t\t\tif scene_path and ResourceLoader.exists(scene_path):
\t\t\t\tEditorInterface.open_scene_from_path(scene_path)
\t\t"reload_scene":
\t\t\tvar current = EditorInterface.get_edited_scene_root()
\t\t\tif current:
\t\t\t\tvar p = current.scene_file_path
\t\t\t\tif p:
\t\t\t\t\tEditorInterface.reload_scene_from_path(p)
\t\t"select_node":
\t\t\tvar node_path_str = cmd.get("path", "")
\t\t\tvar root = EditorInterface.get_edited_scene_root()
\t\t\tif root and node_path_str:
\t\t\t\tvar node = root.get_node_or_null(NodePath(node_path_str))
\t\t\t\tif node:
\t\t\t\t\tEditorInterface.get_selection().clear()
\t\t\t\t\tEditorInterface.get_selection().add_node(node)
\t\t\t\t\tEditorInterface.edit_node(node)
\t\t"scan":
\t\t\tEditorInterface.get_resource_filesystem().scan()
\t\t"run_scene":
\t\t\tvar scene_path = cmd.get("path", "")
\t\t\tif scene_path:
\t\t\t\tEditorInterface.play_custom_scene(scene_path)
\t\t\telse:
\t\t\t\tEditorInterface.play_main_scene()
\t\t"stop":
\t\t\tEditorInterface.stop_playing_scene()
`;

// ── Key code mapping ───────────────────────────────────────

const KEY_MAP: Record<string, number> = {
  A: 65, B: 66, C: 67, D: 68, E: 69, F: 70, G: 71, H: 72, I: 73, J: 74,
  K: 75, L: 76, M: 77, N: 78, O: 79, P: 80, Q: 81, R: 82, S: 83, T: 84,
  U: 85, V: 86, W: 87, X: 88, Y: 89, Z: 90,
  "0": 48, "1": 49, "2": 50, "3": 51, "4": 52,
  "5": 53, "6": 54, "7": 55, "8": 56, "9": 57,
  Space: 32, Enter: 4194309, Escape: 4194305, Tab: 4194306, Backspace: 4194308,
  Left: 4194319, Right: 4194321, Up: 4194320, Down: 4194322,
  Shift: 4194325, Ctrl: 4194326, Alt: 4194328,
  F1: 4194332, F2: 4194333, F3: 4194334, F4: 4194335, F5: 4194336,
  F6: 4194337, F7: 4194338, F8: 4194339, F9: 4194340, F10: 4194341,
  F11: 4194342, F12: 4194343,
};

// ── Helpers ────────────────────────────────────────────────

function splitConfigKey(key: string): { section: string; subkey: string } {
  const idx = key.indexOf("/");
  if (idx === -1) return { section: "", subkey: key };
  return { section: key.slice(0, idx), subkey: key.slice(idx + 1) };
}

function requireProjectFile(projectDir: string, commandName: string, human?: boolean): string {
  const file = join(projectDir, "project.godot");
  if (!existsSync(file)) {
    output(
      error(commandName, "NOT_A_PROJECT", `No project.godot found at ${projectDir}`, "Run 'godot-forge project init' first or pass --project <path>"),
      human
    );
    process.exit(1);
  }
  return file;
}

function readProjectConfig(projectFile: string) {
  return parseGodotConfig(readFileSync(projectFile, "utf-8"));
}

function writeProjectConfig(projectFile: string, config: ReturnType<typeof parseGodotConfig>) {
  writeFileSync(projectFile, generateGodotConfig(config));
}

function formatInputEvent(keycode: number): string {
  return `Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":-1,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":0,"physical_keycode":${keycode},"key_label":0,"unicode":0)`;
}

function formatInputAction(keys: string[], deadzone: number): string {
  const events = keys
    .map((k) => KEY_MAP[k])
    .filter((code): code is number => code !== undefined)
    .map((code) => formatInputEvent(code));
  return `{\n"deadzone": ${deadzone},\n"events": [${events.join(", ")}]\n}`;
}

function parsePackedStringArray(value: string): string[] {
  const match = value.match(/^PackedStringArray\((.*)\)$/);
  if (!match) return [];
  return [...match[1].matchAll(/"([^"]*)"/g)].map((m) => m[1]);
}

function formatPackedStringArray(items: string[]): string {
  return `PackedStringArray(${items.map((i) => `"${i}"`).join(", ")})`;
}

// ── Command ────────────────────────────────────────────────

export function createProjectCommand(): Command {
  const cmd = new Command("project").description("Project lifecycle management");

  // ── project init ─────────────────────────────────────────

  cmd
    .command("init")
    .description("Initialize a new Godot project")
    .option("--name <name>", "Project name")
    .option("--template <template>", "Project template", "default")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.init";

      try {
        const input = await readInput<Partial<ProjectConfig>>(globals.input);
        const name = opts.name ?? input?.name;

        if (!name) {
          output(error(commandName, "MISSING_NAME", "Project name is required", 'Pass --name <name> or provide {"name": "..."} via stdin'), globals.human);
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? name);

        if (existsSync(join(projectDir, "project.godot")) && !globals.force) {
          output(error(commandName, "PROJECT_EXISTS", `Project already exists at ${projectDir}`, "Use --force to overwrite"), globals.human);
          process.exit(1);
        }

        const dirs = ["scenes", "scripts", "assets", "assets/sprites", "assets/audio", "assets/fonts", "addons/forge_sync"];

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              path: projectDir,
              files: ["project.godot", ".gitignore", "addons/forge_sync/plugin.cfg", "addons/forge_sync/forge_sync.gd", ...dirs.map((d) => d + "/")],
            }),
            globals.human
          );
          return;
        }

        for (const dir of dirs) {
          mkdirSync(join(projectDir, dir), { recursive: true });
        }

        writeFileSync(join(projectDir, "project.godot"), GODOT_PROJECT_FILE.replace("{name}", name));
        writeFileSync(join(projectDir, ".gitignore"), DEFAULT_GITIGNORE);
        writeFileSync(join(projectDir, "addons", "forge_sync", "plugin.cfg"), FORGE_SYNC_PLUGIN_CFG);
        writeFileSync(join(projectDir, "addons", "forge_sync", "forge_sync.gd"), FORGE_SYNC_GD);

        const result = {
          path: projectDir,
          name,
          template: opts.template,
          files_created: [
            "project.godot",
            ".gitignore",
            "addons/forge_sync/plugin.cfg",
            "addons/forge_sync/forge_sync.gd",
            ...dirs.map((d) => d + "/"),
          ],
        };

        output(success(commandName, globals.fields ? filterFields(result, globals.fields) : result), globals.human);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "INIT_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── project info ─────────────────────────────────────────

  cmd
    .command("info")
    .description("Show project information")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.info";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const content = readFileSync(projectFile, "utf-8");
      const nameMatch = content.match(/config\/name="(.+?)"/);
      const featuresMatch = content.match(/config\/features=PackedStringArray\("(.+?)"\)/);

      const result = {
        path: projectDir,
        name: nameMatch?.[1] ?? "unknown",
        godot_version: featuresMatch?.[1] ?? "unknown",
      };

      output(success(commandName, globals.fields ? filterFields(result, globals.fields) : result), globals.human);
    });

  // ── project config ───────────────────────────────────────

  const configCmd = new Command("config").description("Project settings management");

  configCmd
    .command("get")
    .description("Get a project setting")
    .argument("<key>", "Setting key (section/subkey format)")
    .action(async (key: string, _opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.config.get";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const { section, subkey } = splitConfigKey(key);
      const config = readProjectConfig(projectFile);
      const value = configGet(config, section, subkey);

      if (value === undefined) {
        output(error(commandName, "KEY_NOT_FOUND", `Setting not found: ${key}`, `Use 'project config list ${section}' to see available keys`), globals.human);
        process.exit(1);
      }

      output(success(commandName, { key, section, subkey, value }), globals.human);
    });

  configCmd
    .command("set")
    .description("Set a project setting")
    .argument("<key>", "Setting key")
    .argument("<value>", "Setting value")
    .action(async (key: string, value: string, _opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.config.set";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const { section, subkey } = splitConfigKey(key);

      if (globals.dryRun) {
        output(success(commandName, { action: "dry-run", key, section, subkey, value }), globals.human);
        return;
      }

      const config = readProjectConfig(projectFile);
      configSet(config, section, subkey, value);
      writeProjectConfig(projectFile, config);

      output(success(commandName, { key, section, subkey, value }), globals.human);
    });

  configCmd
    .command("list")
    .description("List project settings")
    .argument("[section]", "Section to list (omit for all)")
    .action(async (section: string | undefined, _opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.config.list";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const config = readProjectConfig(projectFile);

      if (section) {
        const entries = configListSection(config, section);
        if (!entries) {
          output(error(commandName, "SECTION_NOT_FOUND", `Section not found: ${section}`, `Available: ${configListSections(config).join(", ")}`), globals.human);
          process.exit(1);
        }
        const settings: Record<string, string> = {};
        for (const [k, v] of entries) settings[k] = v;
        output(success(commandName, { section, settings }), globals.human);
      } else {
        const sections: Record<string, Record<string, string>> = {};
        for (const name of configListSections(config)) {
          const entries = configListSection(config, name)!;
          sections[name] = {};
          for (const [k, v] of entries) sections[name][k] = v;
        }
        output(success(commandName, { sections }), globals.human);
      }
    });

  cmd.addCommand(configCmd);

  // ── project autoload ─────────────────────────────────────

  const autoloadCmd = new Command("autoload").description("Autoload singleton management");

  autoloadCmd
    .command("add")
    .description("Register an autoload singleton")
    .option("--name <name>", "Global singleton name")
    .option("--path <path>", "Script path (res:// format)")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.autoload.add";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const input = await readInput<{ name?: string; path?: string }>(globals.input);
      const name = opts.name ?? input?.name;
      const path = opts.path ?? input?.path;

      if (!name || !path) {
        output(error(commandName, "MISSING_INPUT", "Both name and path are required", 'Pass --name and --path, or JSON: {"name":"Global","path":"res://scripts/global.gd"}'), globals.human);
        process.exit(1);
      }

      if (globals.dryRun) {
        output(success(commandName, { action: "dry-run", name, path }), globals.human);
        return;
      }

      const config = readProjectConfig(projectFile);
      const resPath = path.startsWith("*") ? path : `*${path}`;
      configSet(config, "autoload", name, `"${resPath}"`);
      writeProjectConfig(projectFile, config);

      output(success(commandName, { name, path, enabled: true }), globals.human);
    });

  autoloadCmd
    .command("remove")
    .description("Remove an autoload singleton")
    .option("--name <name>", "Singleton name to remove")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.autoload.remove";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const input = await readInput<{ name?: string }>(globals.input);
      const name = opts.name ?? input?.name;

      if (!name) {
        output(error(commandName, "MISSING_NAME", "Autoload name is required"), globals.human);
        process.exit(1);
      }

      const config = readProjectConfig(projectFile);
      const existing = configGet(config, "autoload", name);
      if (!existing) {
        output(error(commandName, "NOT_FOUND", `Autoload '${name}' not found`), globals.human);
        process.exit(1);
      }

      if (globals.dryRun) {
        output(success(commandName, { action: "dry-run", name, removed: true }), globals.human);
        return;
      }

      configUnset(config, "autoload", name);
      writeProjectConfig(projectFile, config);

      output(success(commandName, { name, removed: true }), globals.human);
    });

  autoloadCmd
    .command("list")
    .description("List all autoload singletons")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.autoload.list";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const config = readProjectConfig(projectFile);
      const section = configListSection(config, "autoload");
      const autoloads: { name: string; path: string; enabled: boolean }[] = [];

      if (section) {
        for (const [name, rawValue] of section) {
          const value = rawValue.replace(/^"|"$/g, "");
          const enabled = value.startsWith("*");
          const path = enabled ? value.slice(1) : value;
          autoloads.push({ name, path, enabled });
        }
      }

      output(success(commandName, { count: autoloads.length, autoloads }), globals.human);
    });

  cmd.addCommand(autoloadCmd);

  // ── project input ────────────────────────────────────────

  const inputCmd = new Command("input").description("Input action management");

  inputCmd
    .command("add")
    .description("Add an input action")
    .option("--name <name>", "Action name")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.input.add";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const input = await readInput<{ name?: string; keys?: string[]; deadzone?: number }>(globals.input);
      const name = opts.name ?? input?.name;
      const keys = input?.keys ?? [];
      const deadzone = input?.deadzone ?? 0.2;

      if (!name) {
        output(error(commandName, "MISSING_NAME", "Action name is required", 'Provide {"name":"move_left","keys":["A","Left"]} via stdin'), globals.human);
        process.exit(1);
      }

      if (keys.length === 0) {
        output(error(commandName, "MISSING_KEYS", "At least one key is required", `Available keys: ${Object.keys(KEY_MAP).join(", ")}`), globals.human);
        process.exit(1);
      }

      const unknownKeys = keys.filter((k) => !(k in KEY_MAP));
      if (unknownKeys.length > 0) {
        output(error(commandName, "UNKNOWN_KEYS", `Unknown key names: ${unknownKeys.join(", ")}`, `Available: ${Object.keys(KEY_MAP).join(", ")}`), globals.human);
        process.exit(1);
      }

      if (globals.dryRun) {
        output(success(commandName, { action: "dry-run", name, keys, deadzone }), globals.human);
        return;
      }

      const config = readProjectConfig(projectFile);
      const value = formatInputAction(keys, deadzone);
      configSet(config, "input", name, value);
      writeProjectConfig(projectFile, config);

      output(success(commandName, { name, keys, deadzone }), globals.human);
    });

  inputCmd
    .command("remove")
    .description("Remove an input action")
    .option("--name <name>", "Action name to remove")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.input.remove";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const input = await readInput<{ name?: string }>(globals.input);
      const name = opts.name ?? input?.name;

      if (!name) {
        output(error(commandName, "MISSING_NAME", "Action name is required"), globals.human);
        process.exit(1);
      }

      const config = readProjectConfig(projectFile);
      if (!configGet(config, "input", name)) {
        output(error(commandName, "NOT_FOUND", `Input action '${name}' not found`), globals.human);
        process.exit(1);
      }

      if (globals.dryRun) {
        output(success(commandName, { action: "dry-run", name, removed: true }), globals.human);
        return;
      }

      configUnset(config, "input", name);
      writeProjectConfig(projectFile, config);

      output(success(commandName, { name, removed: true }), globals.human);
    });

  inputCmd
    .command("list")
    .description("List all input actions")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.input.list";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const config = readProjectConfig(projectFile);
      const section = configListSection(config, "input");
      const actions: { name: string; deadzone: number; event_count: number }[] = [];

      if (section) {
        for (const [name, rawValue] of section) {
          const dzMatch = rawValue.match(/"deadzone":\s*([\d.]+)/);
          const deadzone = dzMatch ? parseFloat(dzMatch[1]) : 0.2;
          const eventMatches = rawValue.match(/Object\(InputEvent/g);
          const eventCount = eventMatches?.length ?? 0;
          actions.push({ name, deadzone, event_count: eventCount });
        }
      }

      output(success(commandName, { count: actions.length, actions }), globals.human);
    });

  cmd.addCommand(inputCmd);

  // ── project plugin ───────────────────────────────────────

  const pluginCmd = new Command("plugin").description("Editor plugin management");

  pluginCmd
    .command("list")
    .description("List enabled editor plugins")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.plugin.list";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const config = readProjectConfig(projectFile);
      const raw = configGet(config, "editor_plugins", "enabled") ?? "";
      const plugins = parsePackedStringArray(raw);

      output(success(commandName, { count: plugins.length, plugins }), globals.human);
    });

  pluginCmd
    .command("enable")
    .description("Enable an editor plugin")
    .option("--name <path>", "Plugin path (res://addons/.../plugin.cfg)")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.plugin.enable";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const input = await readInput<{ name?: string }>(globals.input);
      const pluginPath = opts.name ?? input?.name;

      if (!pluginPath) {
        output(error(commandName, "MISSING_NAME", "Plugin path is required", 'Pass --name "res://addons/plugin_name/plugin.cfg"'), globals.human);
        process.exit(1);
      }

      const config = readProjectConfig(projectFile);
      const raw = configGet(config, "editor_plugins", "enabled") ?? "PackedStringArray()";
      const plugins = parsePackedStringArray(raw);

      if (plugins.includes(pluginPath)) {
        output(success(commandName, { plugin: pluginPath, already_enabled: true }), globals.human);
        return;
      }

      if (globals.dryRun) {
        output(success(commandName, { action: "dry-run", plugin: pluginPath }), globals.human);
        return;
      }

      plugins.push(pluginPath);
      configSet(config, "editor_plugins", "enabled", formatPackedStringArray(plugins));
      writeProjectConfig(projectFile, config);

      output(success(commandName, { plugin: pluginPath, enabled: true }), globals.human);
    });

  pluginCmd
    .command("disable")
    .description("Disable an editor plugin")
    .option("--name <path>", "Plugin path to disable")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "project.plugin.disable";
      const projectDir = resolve(globals.project ?? ".");
      const projectFile = requireProjectFile(projectDir, commandName, globals.human);

      const input = await readInput<{ name?: string }>(globals.input);
      const pluginPath = opts.name ?? input?.name;

      if (!pluginPath) {
        output(error(commandName, "MISSING_NAME", "Plugin path is required"), globals.human);
        process.exit(1);
      }

      const config = readProjectConfig(projectFile);
      const raw = configGet(config, "editor_plugins", "enabled") ?? "PackedStringArray()";
      const plugins = parsePackedStringArray(raw);

      if (!plugins.includes(pluginPath)) {
        output(error(commandName, "NOT_FOUND", `Plugin '${pluginPath}' is not enabled`), globals.human);
        process.exit(1);
      }

      if (globals.dryRun) {
        output(success(commandName, { action: "dry-run", plugin: pluginPath }), globals.human);
        return;
      }

      const filtered = plugins.filter((p) => p !== pluginPath);
      configSet(config, "editor_plugins", "enabled", formatPackedStringArray(filtered));
      writeProjectConfig(projectFile, config);

      output(success(commandName, { plugin: pluginPath, disabled: true }), globals.human);
    });

  cmd.addCommand(pluginCmd);

  return cmd;
}
