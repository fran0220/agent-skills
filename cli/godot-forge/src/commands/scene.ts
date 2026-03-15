/**
 * scene command group — Scene CRUD operations (L2).
 */

import { Command } from "commander";
import {
  readFileSync,
  writeFileSync,
  existsSync,
  unlinkSync,
  readdirSync,
  statSync,
} from "node:fs";
import { join, resolve, relative } from "node:path";
import { success, error, output, filterFields } from "../utils/output.js";
import { readInput } from "../utils/input.js";
import { sendEditorCommand } from "../utils/editor-bridge.js";
import type { GlobalOptions, SceneSpec, NodeSpec } from "../types/index.js";
import {
  parseTscn,
  generateTscn,
  createEmptyScene,
  addNode,
} from "../core/parser/tscn.js";
import type { TscnNode } from "../core/parser/types.js";

function resolveScenePath(projectDir: string, scenePath: string): string {
  // "res://..." paths — strip prefix and resolve from project root
  if (scenePath.startsWith("res://")) {
    return join(projectDir, scenePath.slice(6));
  }
  // Absolute path — use as-is
  if (scenePath.startsWith("/")) {
    return scenePath;
  }
  // Already has .tscn extension
  if (scenePath.endsWith(".tscn")) {
    return join(projectDir, scenePath);
  }
  // Short name — resolve to scenes/<name>.tscn
  return join(projectDir, "scenes", `${scenePath}.tscn`);
}

function flattenChildren(
  children: NodeSpec[],
  parentPath: string
): TscnNode[] {
  const nodes: TscnNode[] = [];
  for (const child of children) {
    const node: TscnNode = {
      name: child.name,
      type: child.type,
      parent: parentPath,
      properties: child.properties ?? {},
    };
    nodes.push(node);
    if (child.children?.length) {
      const childPath = parentPath === "." ? child.name : `${parentPath}/${child.name}`;
      nodes.push(...flattenChildren(child.children, childPath));
    }
  }
  return nodes;
}

function findTscnFiles(dir: string): string[] {
  const results: string[] = [];
  try {
    const entries = readdirSync(dir);
    for (const entry of entries) {
      const fullPath = join(dir, entry);
      // Skip hidden directories
      if (entry.startsWith(".")) continue;
      try {
        const stat = statSync(fullPath);
        if (stat.isDirectory()) {
          results.push(...findTscnFiles(fullPath));
        } else if (entry.endsWith(".tscn")) {
          results.push(fullPath);
        }
      } catch {
        // Skip inaccessible entries
      }
    }
  } catch {
    // Skip inaccessible directories
  }
  return results;
}

export function createSceneCommand(): Command {
  const cmd = new Command("scene").description("Scene CRUD operations");

  // ── scene create ──────────────────────────────────────

  cmd
    .command("create")
    .description("Create a new .tscn scene file")
    .option("--name <name>", "Scene name")
    .option("--root-type <type>", "Root node type", "Node2D")
    .option("--path <path>", "Output path (relative to project)")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "scene.create";

      try {
        const input = await readInput<Partial<SceneSpec>>(globals.input);
        const name = opts.name ?? input?.name;
        const rootType = opts.rootType ?? input?.root_type ?? "Node2D";
        const children = input?.children ?? [];

        if (!name) {
          output(
            error(commandName, "MISSING_NAME", "Scene name is required", 'Pass --name <name> or provide {"name": "..."} via stdin'),
            globals.human
          );
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(
          projectDir,
          opts.path ?? input?.path ?? name
        );

        if (existsSync(scenePath) && !globals.force) {
          output(
            error(commandName, "SCENE_EXISTS", `Scene already exists at ${scenePath}`, "Use --force to overwrite"),
            globals.human
          );
          process.exit(1);
        }

        let doc = createEmptyScene(rootType, name);

        // Add children from spec
        if (children.length) {
          const childNodes = flattenChildren(children, ".");
          for (const node of childNodes) {
            doc = addNode(doc, node);
          }
        }

        // Update load_steps if needed
        doc.loadSteps = doc.extResources.length + doc.subResources.length;

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              path: scenePath,
              root_type: rootType,
              node_count: doc.nodes.length,
            }),
            globals.human
          );
          return;
        }

        // Ensure parent directory exists
        const { mkdirSync } = await import("node:fs");
        mkdirSync(join(scenePath, ".."), { recursive: true });

        writeFileSync(scenePath, generateTscn(doc));

        const resPath = `res://${relative(projectDir, scenePath)}`;

        // Notify editor: scan + open the new scene
        sendEditorCommand(projectDir, { action: "open_scene", path: resPath });

        const result = {
          path: scenePath,
          res_path: resPath,
          root_type: rootType,
          root_name: name,
          node_count: doc.nodes.length,
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

  // ── scene read ────────────────────────────────────────

  cmd
    .command("read")
    .description("Read and return scene structure as JSON")
    .argument("<scene>", "Scene name or path")
    .action(async (scene: string, _opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "scene.read";

      try {
        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, scene);

        if (!existsSync(scenePath)) {
          output(
            error(commandName, "SCENE_NOT_FOUND", `Scene not found: ${scenePath}`, "Check the scene name or path"),
            globals.human
          );
          process.exit(1);
        }

        const content = readFileSync(scenePath, "utf-8");
        const doc = parseTscn(content);

        const resPath = `res://${relative(projectDir, scenePath)}`;
        const result = {
          path: scenePath,
          res_path: resPath,
          scene: doc,
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

  // ── scene list ────────────────────────────────────────

  cmd
    .command("list")
    .description("List all .tscn files in the project")
    .action(async (_opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "scene.list";

      try {
        const projectDir = resolve(globals.project ?? ".");

        if (!existsSync(join(projectDir, "project.godot"))) {
          output(
            error(commandName, "NOT_A_PROJECT", `No project.godot found at ${projectDir}`, "Run 'godot-forge project init' first or pass --project <path>"),
            globals.human
          );
          process.exit(1);
        }

        const files = findTscnFiles(projectDir);
        const scenes = files.map((f) => ({
          path: f,
          res_path: `res://${relative(projectDir, f)}`,
          name: f.split("/").pop()!.replace(".tscn", ""),
        }));

        output(
          success(commandName, { count: scenes.length, scenes }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "LIST_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── scene update ──────────────────────────────────────

  cmd
    .command("update")
    .description("Update scene metadata (root type, root name, UID)")
    .option("--scene <scene>", "Scene name or path")
    .option("--root-type <type>", "New root node type")
    .option("--root-name <name>", "New root node name")
    .option("--uid <uid>", "Scene UID")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "scene.update";

      try {
        interface SceneUpdateInput {
          scene: string;
          root_type?: string;
          root_name?: string;
          uid?: string;
        }

        const input = await readInput<SceneUpdateInput>(globals.input);
        const sceneName = opts.scene ?? input?.scene;
        const rootType = opts.rootType ?? input?.root_type;
        const rootName = opts.rootName ?? input?.root_name;
        const uid = opts.uid ?? input?.uid;

        if (!sceneName) {
          output(
            error(commandName, "MISSING_SCENE", "Scene name or path is required", 'Pass --scene <s> or provide {"scene": "..."} via stdin'),
            globals.human
          );
          process.exit(1);
        }

        if (!rootType && !rootName && !uid) {
          output(
            error(commandName, "NO_CHANGES", "At least one update field is required", "Provide --root-type, --root-name, or --uid"),
            globals.human
          );
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);

        if (!existsSync(scenePath)) {
          output(
            error(commandName, "SCENE_NOT_FOUND", `Scene not found: ${scenePath}`, "Check the scene name or path"),
            globals.human
          );
          process.exit(1);
        }

        const content = readFileSync(scenePath, "utf-8");
        const doc = parseTscn(content);

        const changes: Record<string, { from: unknown; to: unknown }> = {};

        if (rootType && doc.nodes.length > 0) {
          changes.root_type = { from: doc.nodes[0].type, to: rootType };
          doc.nodes[0].type = rootType;
        }

        if (rootName && doc.nodes.length > 0) {
          const oldName = doc.nodes[0].name;
          changes.root_name = { from: oldName, to: rootName };
          doc.nodes[0].name = rootName;

          // Update children whose parent references the old root name
          for (let i = 1; i < doc.nodes.length; i++) {
            const node = doc.nodes[i];
            if (node.parent === oldName) {
              node.parent = rootName;
            } else if (node.parent?.startsWith(oldName + "/")) {
              node.parent = rootName + node.parent.slice(oldName.length);
            }
          }
        }

        if (uid) {
          changes.uid = { from: doc.uid, to: uid };
          doc.uid = uid;
        }

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              path: scenePath,
              changes,
            }),
            globals.human
          );
          return;
        }

        writeFileSync(scenePath, generateTscn(doc));

        const resPath = `res://${relative(projectDir, scenePath)}`;
        sendEditorCommand(projectDir, { action: "reload_scene", path: resPath });

        const result = {
          path: scenePath,
          res_path: resPath,
          changes,
        };

        output(
          success(commandName, globals.fields ? filterFields(result, globals.fields) : result),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "UPDATE_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── scene delete ──────────────────────────────────────

  cmd
    .command("delete")
    .description("Delete a scene file (requires --force)")
    .argument("<scene>", "Scene name or path")
    .action(async (scene: string, _opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "scene.delete";

      try {
        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, scene);

        if (!existsSync(scenePath)) {
          output(
            error(commandName, "SCENE_NOT_FOUND", `Scene not found: ${scenePath}`, "Check the scene name or path"),
            globals.human
          );
          process.exit(1);
        }

        if (!globals.force) {
          output(
            error(commandName, "FORCE_REQUIRED", "Deleting a scene requires --force", "Pass --force to confirm deletion"),
            globals.human
          );
          process.exit(1);
        }

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              path: scenePath,
              would_delete: true,
            }),
            globals.human
          );
          return;
        }

        unlinkSync(scenePath);

        const resPath = `res://${relative(projectDir, scenePath)}`;
        output(
          success(commandName, { path: scenePath, res_path: resPath, deleted: true }),
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
