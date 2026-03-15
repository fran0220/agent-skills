/**
 * describe command — Schema introspection for Agent discovery + engine introspection.
 */

import { Command } from "commander";
import { existsSync, mkdirSync, writeFileSync, unlinkSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { success, error, output } from "../utils/output.js";
import { findGodot, runGodotScript } from "../utils/subprocess.js";
import type { GlobalOptions } from "../types/index.js";

/* ---------- Static SCHEMAS ---------- */

const SCHEMAS: Record<string, object> = {
  "project.init": {
    command: "project init",
    description: "Initialize a new Godot project",
    input: { type: "object", properties: { name: { type: "string", required: true }, template: { type: "string", default: "default" } } },
    flags: { "--name": "Project name", "--template": "Project template" },
    output: { type: "object", properties: { path: { type: "string" }, name: { type: "string" }, files_created: { type: "string[]" } } },
  },
  "project.info": {
    command: "project info",
    description: "Show project information",
    input: null,
    flags: {},
    output: { type: "object", properties: { path: { type: "string" }, name: { type: "string" }, godot_version: { type: "string" } } },
  },
  "project.config.get": {
    command: "project config get <key>",
    description: "Get a project setting",
    input: { type: "argument", properties: { key: { type: "string", required: true, description: "section/key format" } } },
    flags: {},
    output: { type: "object", properties: { key: { type: "string" }, value: { type: "string" }, section: { type: "string" } } },
  },
  "project.config.set": {
    command: "project config set <key> <value>",
    description: "Set a project setting",
    input: { type: "arguments", properties: { key: { type: "string", required: true }, value: { type: "string", required: true } } },
    flags: {},
    output: { type: "object", properties: { key: { type: "string" }, value: { type: "string" }, section: { type: "string" } } },
  },
  "project.config.list": {
    command: "project config list [section]",
    description: "List project settings",
    input: null,
    flags: {},
    output: { type: "object", properties: { sections: { type: "object" } } },
  },
  "project.autoload.add": {
    command: "project autoload add",
    description: "Register an autoload singleton",
    input: { type: "object", properties: { name: { type: "string", required: true }, path: { type: "string", required: true } } },
    flags: { "--name": "Global name", "--path": "Script path (res://)" },
    output: { type: "object", properties: { name: { type: "string" }, path: { type: "string" } } },
  },
  "project.autoload.remove": {
    command: "project autoload remove",
    description: "Remove an autoload singleton",
    input: { type: "object", properties: { name: { type: "string", required: true } } },
    flags: { "--name": "Name to remove" },
    output: { type: "object", properties: { name: { type: "string" }, removed: { type: "boolean" } } },
  },
  "project.autoload.list": {
    command: "project autoload list",
    description: "List autoload singletons",
    input: null,
    flags: {},
    output: { type: "object", properties: { count: { type: "number" }, autoloads: { type: "array" } } },
  },
  "project.input.add": {
    command: "project input add",
    description: "Add an input action",
    input: { type: "object", properties: { name: { type: "string", required: true }, keys: { type: "string[]", required: true }, deadzone: { type: "number", default: 0.2 } } },
    flags: {},
    output: { type: "object", properties: { name: { type: "string" }, keys: { type: "string[]" } } },
  },
  "project.input.remove": {
    command: "project input remove",
    description: "Remove an input action",
    input: { type: "object", properties: { name: { type: "string", required: true } } },
    flags: { "--name": "Action name" },
    output: { type: "object", properties: { name: { type: "string" }, removed: { type: "boolean" } } },
  },
  "project.input.list": {
    command: "project input list",
    description: "List input actions",
    input: null,
    flags: {},
    output: { type: "object", properties: { count: { type: "number" }, actions: { type: "array" } } },
  },
  "scene.create": {
    command: "scene create",
    description: "Create a new .tscn scene file",
    input: { type: "object", properties: { name: { type: "string", required: true }, root_type: { type: "string", default: "Node2D" }, path: { type: "string" }, children: { type: "array" } } },
    flags: { "--name": "Scene name", "--root-type": "Root node type", "--path": "Output path" },
    output: { type: "object", properties: { path: { type: "string" }, res_path: { type: "string" }, root_type: { type: "string" }, node_count: { type: "number" } } },
  },
  "scene.read": {
    command: "scene read <scene>",
    description: "Read scene structure as JSON",
    input: { type: "argument", properties: { scene: { type: "string", required: true } } },
    flags: {},
    output: { type: "object", properties: { path: { type: "string" }, scene: { type: "object" } } },
  },
  "scene.list": {
    command: "scene list",
    description: "List all .tscn files",
    input: null,
    flags: {},
    output: { type: "object", properties: { count: { type: "number" }, scenes: { type: "array" } } },
  },
  "scene.delete": {
    command: "scene delete <scene>",
    description: "Delete a scene (requires --force)",
    input: { type: "argument", properties: { scene: { type: "string", required: true } } },
    flags: {},
    output: { type: "object", properties: { path: { type: "string" }, deleted: { type: "boolean" } } },
  },
  "scene.update": {
    command: "scene update",
    description: "Update scene metadata",
    input: { type: "object", properties: { scene: { type: "string", required: true }, root_type: { type: "string" }, root_name: { type: "string" }, uid: { type: "string" } } },
    flags: { "--scene": "Scene", "--root-type": "New root type", "--root-name": "New root name" },
    output: { type: "object", properties: { scene: { type: "string" }, updated: { type: "object" } } },
  },
  "node.add": {
    command: "node add",
    description: "Add a node to a scene",
    input: { type: "object", properties: { scene: { type: "string", required: true }, name: { type: "string", required: true }, type: { type: "string" }, parent: { type: "string", default: "." }, properties: { type: "object" }, script: { type: "string" }, instance: { type: "string" } } },
    flags: { "--scene": "Scene", "--name": "Node name", "--type": "Node type", "--parent": "Parent", "--script": "Script path", "--instance": "Scene to instance" },
    output: { type: "object", properties: { scene: { type: "string" }, node: { type: "object" }, node_count: { type: "number" } } },
  },
  "node.remove": {
    command: "node remove",
    description: "Remove a node from a scene",
    input: { type: "object", properties: { scene: { type: "string", required: true }, path: { type: "string", required: true } } },
    flags: { "--scene": "Scene", "--path": "Node path" },
    output: { type: "object", properties: { removed_path: { type: "string" }, remaining_nodes: { type: "number" } } },
  },
  "node.list": {
    command: "node list",
    description: "List all nodes in a scene",
    input: { type: "object", properties: { scene: { type: "string", required: true } } },
    flags: { "--scene": "Scene" },
    output: { type: "object", properties: { count: { type: "number" }, nodes: { type: "array" } } },
  },
  "node.update": {
    command: "node update",
    description: "Update node properties",
    input: { type: "object", properties: { scene: { type: "string", required: true }, path: { type: "string", required: true }, properties: { type: "object" }, script: { type: "string" } } },
    flags: { "--scene": "Scene", "--path": "Node path" },
    output: { type: "object", properties: { node_path: { type: "string" }, updated_properties: { type: "array" } } },
  },
  "node.move": {
    command: "node move",
    description: "Move a node to a new parent",
    input: { type: "object", properties: { scene: { type: "string", required: true }, path: { type: "string", required: true }, new_parent: { type: "string", required: true } } },
    flags: { "--scene": "Scene", "--path": "Node path", "--new-parent": "New parent" },
    output: { type: "object", properties: { old_path: { type: "string" }, new_path: { type: "string" } } },
  },
  "node.connect": {
    command: "node connect",
    description: "Add a signal connection",
    input: { type: "object", properties: { scene: { type: "string", required: true }, signal: { type: "string", required: true }, from: { type: "string", required: true }, to: { type: "string", required: true }, method: { type: "string", required: true } } },
    flags: { "--scene": "Scene", "--signal": "Signal", "--from": "Source", "--to": "Target", "--method": "Handler" },
    output: { type: "object", properties: { connection: { type: "object" }, total_connections: { type: "number" } } },
  },
  "node.disconnect": {
    command: "node disconnect",
    description: "Remove a signal connection",
    input: { type: "object", properties: { scene: { type: "string", required: true }, signal: { type: "string", required: true }, from: { type: "string", required: true }, to: { type: "string", required: true } } },
    flags: { "--scene": "Scene", "--signal": "Signal", "--from": "Source", "--to": "Target" },
    output: { type: "object", properties: { removed: { type: "boolean" } } },
  },
  "node.connections": {
    command: "node connections",
    description: "List signal connections in a scene",
    input: { type: "object", properties: { scene: { type: "string", required: true } } },
    flags: { "--scene": "Scene" },
    output: { type: "object", properties: { count: { type: "number" }, connections: { type: "array" } } },
  },
  "script.create": {
    command: "script create",
    description: "Generate a GDScript file",
    input: { type: "object", properties: { name: { type: "string", required: true }, extends: { type: "string", default: "Node" }, path: { type: "string" }, signals: { type: "string[]" }, methods: { type: "string[]" }, exports: { type: "array" } } },
    flags: { "--name": "Script name", "--extends": "Base class", "--path": "Output path" },
    output: { type: "object", properties: { path: { type: "string" }, class_name: { type: "string" } } },
  },
  "script.list": {
    command: "script list",
    description: "List all .gd files",
    input: null,
    flags: {},
    output: { type: "object", properties: { count: { type: "number" }, scripts: { type: "array" } } },
  },
  "script.validate": {
    command: "script validate <path>",
    description: "Check script for issues",
    input: { type: "argument", properties: { path: { type: "string", required: true } } },
    flags: {},
    output: { type: "object", properties: { path: { type: "string" }, valid: { type: "boolean" }, issues: { type: "array" } } },
  },
  "script.edit": {
    command: "script edit",
    description: "Update parts of an existing script",
    input: { type: "object", properties: { path: { type: "string", required: true }, add_signals: { type: "string[]" }, add_methods: { type: "string[]" }, add_exports: { type: "array" } } },
    flags: { "--path": "Script path" },
    output: { type: "object", properties: { path: { type: "string" }, added: { type: "object" } } },
  },
  "engine.editor": {
    command: "engine editor",
    description: "Open project in Godot editor",
    input: null,
    flags: {},
    output: { type: "object", properties: { pid: { type: "number" } } },
  },
  "engine.validate": {
    command: "engine validate",
    description: "Validate project structure",
    input: null,
    flags: { "--deep": "Deep validation via Godot" },
    output: { type: "object", properties: { valid: { type: "boolean" }, issues: { type: "array" } } },
  },
  "engine.run": {
    command: "engine run",
    description: "Run game via Godot",
    input: null,
    flags: { "--scene": "Scene path", "--headless": "Headless mode", "--timeout": "Timeout ms" },
    output: { type: "object", properties: { exit_code: { type: "number" } } },
  },
  "engine.import": {
    command: "engine import",
    description: "Trigger resource import",
    input: null,
    flags: {},
    output: { type: "object", properties: { success: { type: "boolean" } } },
  },
  "engine.uid": {
    command: "engine uid",
    description: "Scan resource UIDs",
    input: null,
    flags: {},
    output: { type: "object", properties: { uids: { type: "object" }, count: { type: "number" } } },
  },
  "engine.preview": {
    command: "engine preview",
    description: "Render scene screenshot",
    input: { type: "object", properties: { scene: { type: "string" }, output: { type: "string" }, size: { type: "array" } } },
    flags: { "--scene": "Scene", "--output": "PNG path", "--width": "Width", "--height": "Height" },
    output: { type: "object", properties: { output: { type: "string" }, width: { type: "number" }, height: { type: "number" } } },
  },
  "export.preset.list": {
    command: "export preset list",
    description: "List export presets",
    input: null,
    flags: {},
    output: { type: "object", properties: { count: { type: "number" }, presets: { type: "array" } } },
  },
  "export.preset.add": {
    command: "export preset add",
    description: "Add export preset",
    input: { type: "object", properties: { name: { type: "string", required: true }, platform: { type: "string", required: true }, path: { type: "string" }, runnable: { type: "boolean" } } },
    flags: { "--name": "Name", "--platform": "Platform", "--path": "Export path", "--runnable": "Runnable" },
    output: { type: "object", properties: { operation: { type: "string" }, index: { type: "number" } } },
  },
  "export.build": {
    command: "export build",
    description: "Export/build game (requires Godot)",
    input: { type: "object", properties: { preset: { type: "string", required: true }, output: { type: "string" } } },
    flags: { "--preset": "Preset name", "--output": "Output path" },
    output: { type: "object", properties: { preset: { type: "string" }, output: { type: "string" } } },
  },
  "resource.create": {
    command: "resource create",
    description: "Create a .tres resource",
    input: { type: "object", properties: { type: { type: "string", required: true }, name: { type: "string", required: true }, path: { type: "string", required: true }, properties: { type: "object" } } },
    flags: { "--type": "Resource type", "--name": "Name", "--path": "Path" },
    output: { type: "object", properties: { file: { type: "string" }, type: { type: "string" } } },
  },
  "resource.list": {
    command: "resource list",
    description: "List resources by category",
    input: null,
    flags: {},
    output: { type: "object", properties: { scenes: { type: "array" }, scripts: { type: "array" }, resources: { type: "array" }, textures: { type: "array" }, audio: { type: "array" } } },
  },
  "resource.read": {
    command: "resource read <path>",
    description: "Read a .tres file as JSON",
    input: { type: "argument", properties: { path: { type: "string", required: true } } },
    flags: {},
    output: { type: "object", properties: { path: { type: "string" }, type: { type: "string" }, properties: { type: "object" } } },
  },
  "resource.update": {
    command: "resource update",
    description: "Update .tres properties",
    input: { type: "object", properties: { path: { type: "string", required: true }, properties: { type: "object", required: true } } },
    flags: { "--path": "Resource path" },
    output: { type: "object", properties: { path: { type: "string" }, updated_properties: { type: "array" } } },
  },
  "resource.import": {
    command: "resource import",
    description: "Trigger Godot import (requires Godot)",
    input: null,
    flags: {},
    output: { type: "object", properties: { log: { type: "string" } } },
  },
  "resource.check": {
    command: "resource check",
    description: "Check for broken resource references",
    input: null,
    flags: {},
    output: { type: "object", properties: { files_scanned: { type: "number" }, missing_count: { type: "number" }, missing: { type: "array" } } },
  },
};

/* ---------- Engine introspection helpers ---------- */

const ENGINE_TARGETS = ["node-types", "resource-types", "class:<ClassName>", "project-settings"];

function writeTempScript(name: string, content: string): string {
  const dir = join(tmpdir(), "godot-forge");
  mkdirSync(dir, { recursive: true });
  const p = join(dir, `${name}_${Date.now()}.gd`);
  writeFileSync(p, content);
  return p;
}

function cleanupTempScript(p: string): void {
  try {
    unlinkSync(p);
  } catch {
    /* ignore */
  }
}

async function runEngineIntrospection(
  scriptContent: string,
  _commandName: string,
  human?: boolean,
): Promise<Record<string, unknown> | null> {
  let godot: string;
  try {
    godot = findGodot();
  } catch {
    output(
      error("describe", "GODOT_NOT_FOUND", "Godot engine required for engine introspection", "Install Godot 4.4+ and ensure it's in PATH"),
      human,
    );
    process.exit(2);
    return null;
  }

  const scriptPath = writeTempScript("describe", scriptContent);
  try {
    const res = runGodotScript(scriptPath);
    const match = res.stdout.match(/GODOT_FORGE_JSON:(.+)/);
    if (!match) {
      output(
        error("describe", "INTROSPECTION_FAILED", "Failed to parse engine output", res.stdout.trim() || undefined),
        human,
      );
      process.exit(2);
      return null;
    }
    return JSON.parse(match[1]) as Record<string, unknown>;
  } finally {
    cleanupTempScript(scriptPath);
  }
}

/* ---------- GDScript templates ---------- */

const GDSCRIPT_NODE_TYPES = `extends SceneTree
func _init():
\tvar types := []
\tfor c in ClassDB.get_class_list():
\t\tif ClassDB.is_parent_class(c, "Node") or c == "Node":
\t\t\ttypes.append(c)
\ttypes.sort()
\tprint("GODOT_FORGE_JSON:" + JSON.stringify({"node_types": types, "count": types.size()}))
\tquit()
`;

const GDSCRIPT_RESOURCE_TYPES = `extends SceneTree
func _init():
\tvar types := []
\tfor c in ClassDB.get_class_list():
\t\tif ClassDB.is_parent_class(c, "Resource") or c == "Resource":
\t\t\ttypes.append(c)
\ttypes.sort()
\tprint("GODOT_FORGE_JSON:" + JSON.stringify({"resource_types": types, "count": types.size()}))
\tquit()
`;

function gdscriptClassInfo(className: string): string {
  return `extends SceneTree
func _init():
\tvar cn := "${className}"
\tif not ClassDB.class_exists(cn):
\t\tprint("GODOT_FORGE_JSON:" + JSON.stringify({"error": "Unknown class: " + cn}))
\t\tquit()
\t\treturn
\tvar props := []
\tfor p in ClassDB.class_get_property_list(cn):
\t\tprops.append({"name": p.name, "type": p.type})
\tvar sigs := []
\tfor s in ClassDB.class_get_signal_list(cn):
\t\tsigs.append({"name": s.name})
\tvar methods := []
\tfor m in ClassDB.class_get_method_list(cn):
\t\tmethods.append({"name": m.name})
\tvar parent := ClassDB.get_parent_class(cn)
\tprint("GODOT_FORGE_JSON:" + JSON.stringify({"class": cn, "parent": parent, "properties": props, "signals": sigs, "methods": methods}))
\tquit()
`;
}

const GDSCRIPT_PROJECT_SETTINGS = `extends SceneTree
func _init():
\tvar settings := []
\tfor p in ProjectSettings.get_property_list():
\t\tsettings.append({"name": p.name, "type": p.type})
\tprint("GODOT_FORGE_JSON:" + JSON.stringify({"settings": settings, "count": settings.size()}))
\tquit()
`;

/* ---------- Command ---------- */

export function createDescribeCommand(): Command {
  const cmd = new Command("describe")
    .description("Schema introspection for agent discovery")
    .argument("[command]", "Command to describe (e.g., project.init) or engine target (node-types, resource-types, class:<Name>, project-settings)")
    .option("--all", "Show all command schemas")
    .action(async (commandArg, opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;

      if (opts.all) {
        output(success("describe", { commands: Object.keys(SCHEMAS), schemas: SCHEMAS }), globals.human);
        return;
      }

      if (!commandArg) {
        output(
          success("describe", {
            available_commands: Object.keys(SCHEMAS),
            engine_targets: ENGINE_TARGETS,
            usage: "godot-forge describe <command|target>",
          }),
          globals.human,
        );
        return;
      }

      // Engine introspection targets
      if (commandArg === "node-types") {
        const data = await runEngineIntrospection(GDSCRIPT_NODE_TYPES, "node-types", globals.human);
        if (data) output(success("describe", data), globals.human);
        return;
      }

      if (commandArg === "resource-types") {
        const data = await runEngineIntrospection(GDSCRIPT_RESOURCE_TYPES, "resource-types", globals.human);
        if (data) output(success("describe", data), globals.human);
        return;
      }

      if (commandArg.startsWith("class:")) {
        const className = commandArg.slice(6);
        if (!className) {
          output(error("describe", "MISSING_CLASS", "Specify a class name: describe class:<ClassName>"), globals.human);
          process.exit(1);
          return;
        }
        const data = await runEngineIntrospection(gdscriptClassInfo(className), `class:${className}`, globals.human);
        if (data) {
          if (data.error) {
            output(error("describe", "UNKNOWN_CLASS", data.error as string), globals.human);
            process.exit(1);
          } else {
            output(success("describe", data), globals.human);
          }
        }
        return;
      }

      if (commandArg === "project-settings") {
        const data = await runEngineIntrospection(GDSCRIPT_PROJECT_SETTINGS, "project-settings", globals.human);
        if (data) output(success("describe", data), globals.human);
        return;
      }

      // Static schemas
      const schema = SCHEMAS[commandArg];
      if (!schema) {
        output(
          {
            ok: false,
            command: "describe",
            error: {
              code: "UNKNOWN_COMMAND",
              message: `Unknown: ${commandArg}`,
              suggestion: `Available: ${Object.keys(SCHEMAS).join(", ")}. Engine: node-types, resource-types, class:<Name>, project-settings`,
            },
          },
          globals.human,
        );
        process.exit(1);
        return;
      }

      output(success("describe", schema), globals.human);
    });

  return cmd;
}
