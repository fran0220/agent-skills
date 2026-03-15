/**
 * node command group — Node tree manipulation within scenes (L2).
 */

import { Command } from "commander";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { resolve, relative } from "node:path";
import { success, error, output, filterFields } from "../utils/output.js";
import { readInput } from "../utils/input.js";
import { sendEditorCommand } from "../utils/editor-bridge.js";
import type { GlobalOptions } from "../types/index.js";
import {
  parseTscn,
  generateTscn,
  addNode as addNodeToDoc,
  removeNode as removeNodeFromDoc,
  addExtResource,
  findExtResource,
  addConnection,
  removeConnection,
  recalcLoadSteps,
} from "../core/parser/tscn.js";
import type { TscnNode } from "../core/parser/types.js";

interface NodeAddInput {
  scene: string;
  name: string;
  type: string;
  parent?: string;
  properties?: Record<string, unknown>;
  script?: string;
  instance?: string;
}

interface NodeRemoveInput {
  scene: string;
  path: string;
}

interface NodeListInput {
  scene: string;
}

interface NodeUpdateInput {
  scene: string;
  path: string;
  properties: Record<string, unknown>;
  script?: string;
}

function resolveScenePath(projectDir: string, scenePath: string): string {
  if (scenePath.startsWith("res://")) {
    return resolve(projectDir, scenePath.slice(6));
  }
  if (scenePath.startsWith("/")) {
    return scenePath;
  }
  if (scenePath.endsWith(".tscn")) {
    return resolve(projectDir, scenePath);
  }
  return resolve(projectDir, "scenes", `${scenePath}.tscn`);
}

function getNodePath(node: TscnNode): string {
  if (node.parent === undefined) return node.name;
  if (node.parent === ".") return node.name;
  return `${node.parent}/${node.name}`;
}

function readScene(scenePath: string, commandName: string, human?: boolean) {
  if (!existsSync(scenePath)) {
    output(
      error(commandName, "SCENE_NOT_FOUND", `Scene not found: ${scenePath}`, "Check the scene name or path"),
      human
    );
    process.exit(1);
  }
  return parseTscn(readFileSync(scenePath, "utf-8"));
}

export function createNodeCommand(): Command {
  const cmd = new Command("node").description("Node tree manipulation within scenes");

  // ── node add ──────────────────────────────────────────

  cmd
    .command("add")
    .description("Add a node to an existing scene")
    .option("--scene <scene>", "Scene name or path")
    .option("--name <name>", "Node name")
    .option("--type <type>", "Node type")
    .option("--parent <parent>", "Parent node path", ".")
    .option("--script <path>", "Attach a GDScript file")
    .option("--instance <path>", "Instance a PackedScene")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "node.add";

      try {
        const input = await readInput<Partial<NodeAddInput>>(globals.input);
        const sceneName = opts.scene ?? input?.scene;
        const nodeName = opts.name ?? input?.name;
        const nodeType = opts.type ?? input?.type;
        const parent = opts.parent ?? input?.parent ?? ".";
        const properties = input?.properties ?? {};
        const scriptPath = opts.script ?? input?.script;
        const instance = opts.instance ?? input?.instance;

        if (!sceneName) {
          output(
            error(commandName, "MISSING_SCENE", "Scene name is required", 'Pass --scene <scene> or provide {"scene": "..."} via stdin'),
            globals.human
          );
          process.exit(1);
        }
        if (!nodeName) {
          output(
            error(commandName, "MISSING_NAME", "Node name is required", 'Pass --name <name> or provide {"name": "..."} via stdin'),
            globals.human
          );
          process.exit(1);
        }
        if (!nodeType && !instance) {
          output(
            error(commandName, "MISSING_TYPE_OR_INSTANCE", "Either --type or --instance is required"),
            globals.human
          );
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);
        let doc = readScene(scenePath, commandName, globals.human);

        const newNode: TscnNode = {
          name: nodeName,
          type: nodeType,
          parent,
          properties,
        };

        if (instance) {
          const extRes = addExtResource(doc, "PackedScene", instance);
          doc = extRes.doc;
          newNode.instance = `ExtResource("${extRes.id}")`;
          delete newNode.type;
        }

        if (scriptPath) {
          const extRes = addExtResource(doc, "Script", scriptPath);
          doc = extRes.doc;
          newNode.properties.script = `ExtResource("${extRes.id}")`;
        }

        let updated = addNodeToDoc(doc, newNode);
        updated = recalcLoadSteps(updated);

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              scene: scenePath,
              node: { name: nodeName, type: nodeType, parent },
              node_count: updated.nodes.length,
            }),
            globals.human
          );
          return;
        }

        writeFileSync(scenePath, generateTscn(updated));

        const resPath = `res://${relative(projectDir, scenePath)}`;

        // Notify editor to reload the scene
        sendEditorCommand(projectDir, { action: "reload_scene" });

        const result = {
          scene: scenePath,
          res_path: resPath,
          node: { name: nodeName, type: nodeType, parent },
          node_count: updated.nodes.length,
        };

        output(
          success(commandName, globals.fields ? filterFields(result, globals.fields) : result),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "ADD_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── node remove ───────────────────────────────────────

  cmd
    .command("remove")
    .description("Remove a node from a scene")
    .option("--scene <scene>", "Scene name or path")
    .option("--path <path>", "Node path to remove")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "node.remove";

      try {
        const input = await readInput<Partial<NodeRemoveInput>>(globals.input);
        const sceneName = opts.scene ?? input?.scene;
        const nodePath = opts.path ?? input?.path;

        if (!sceneName) {
          output(
            error(commandName, "MISSING_SCENE", "Scene name is required", 'Pass --scene <scene> or provide {"scene": "..."} via stdin'),
            globals.human
          );
          process.exit(1);
        }
        if (!nodePath) {
          output(
            error(commandName, "MISSING_PATH", "Node path is required", 'Pass --path <path> or provide {"path": "..."} via stdin'),
            globals.human
          );
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);
        const doc = readScene(scenePath, commandName, globals.human);

        const updated = removeNodeFromDoc(doc, nodePath);

        if (updated.nodes.length === doc.nodes.length) {
          output(
            error(commandName, "NODE_NOT_FOUND", `Node not found at path: ${nodePath}`, "Use 'node list' to see available nodes"),
            globals.human
          );
          process.exit(1);
        }

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              scene: scenePath,
              removed_path: nodePath,
              remaining_nodes: updated.nodes.length,
            }),
            globals.human
          );
          return;
        }

        writeFileSync(scenePath, generateTscn(updated));

        const result = {
          scene: scenePath,
          res_path: `res://${relative(projectDir, scenePath)}`,
          removed_path: nodePath,
          remaining_nodes: updated.nodes.length,
        };

        output(
          success(commandName, globals.fields ? filterFields(result, globals.fields) : result),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "REMOVE_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── node list ─────────────────────────────────────────

  cmd
    .command("list")
    .description("List all nodes in a scene")
    .option("--scene <scene>", "Scene name or path")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "node.list";

      try {
        const input = await readInput<Partial<NodeListInput>>(globals.input);
        const sceneName = opts.scene ?? input?.scene;

        if (!sceneName) {
          output(
            error(commandName, "MISSING_SCENE", "Scene name is required", 'Pass --scene <scene> or provide {"scene": "..."} via stdin'),
            globals.human
          );
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);
        const doc = readScene(scenePath, commandName, globals.human);

        const nodes = doc.nodes.map((n) => ({
          name: n.name,
          type: n.type ?? null,
          parent: n.parent ?? null,
          path: getNodePath(n),
          property_count: Object.keys(n.properties).length,
        }));

        output(
          success(commandName, {
            scene: scenePath,
            res_path: `res://${relative(projectDir, scenePath)}`,
            count: nodes.length,
            nodes,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "LIST_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── node update ───────────────────────────────────────

  cmd
    .command("update")
    .description("Update node properties")
    .option("--scene <scene>", "Scene name or path")
    .option("--path <path>", "Node path to update")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "node.update";

      try {
        const input = await readInput<Partial<NodeUpdateInput>>(globals.input);
        const sceneName = opts.scene ?? input?.scene;
        const nodePath = opts.path ?? input?.path;
        const properties = input?.properties;

        if (!sceneName) {
          output(
            error(commandName, "MISSING_SCENE", "Scene name is required", 'Pass --scene <scene> or provide {"scene": "..."} via stdin'),
            globals.human
          );
          process.exit(1);
        }
        if (!nodePath) {
          output(
            error(commandName, "MISSING_PATH", "Node path is required", 'Pass --path <path> or provide {"path": "..."} via stdin'),
            globals.human
          );
          process.exit(1);
        }
        const scriptPath = input?.script;

        if ((!properties || Object.keys(properties).length === 0) && !scriptPath) {
          output(
            error(commandName, "MISSING_PROPERTIES", "Properties or script are required", 'Provide {"properties": {...}} or {"script": "..."} via stdin or --input'),
            globals.human
          );
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);
        let doc = readScene(scenePath, commandName, globals.human);

        // Find the target node
        const targetIdx = doc.nodes.findIndex((n) => getNodePath(n) === nodePath);
        if (targetIdx === -1) {
          output(
            error(commandName, "NODE_NOT_FOUND", `Node not found at path: ${nodePath}`, "Use 'node list' to see available nodes"),
            globals.human
          );
          process.exit(1);
        }

        const updatedNodes = [...doc.nodes];
        updatedNodes[targetIdx] = {
          ...updatedNodes[targetIdx],
          properties: {
            ...updatedNodes[targetIdx].properties,
            ...(properties ?? {}),
          },
        };

        if (scriptPath) {
          const extRes = addExtResource(doc, "Script", scriptPath);
          doc = extRes.doc;
          updatedNodes[targetIdx] = {
            ...updatedNodes[targetIdx],
            properties: {
              ...updatedNodes[targetIdx].properties,
              script: `ExtResource("${extRes.id}")`,
            },
          };
        }

        let updated = { ...doc, nodes: updatedNodes };
        updated = recalcLoadSteps(updated);

        if (globals.dryRun) {
          output(
            success(commandName, {
              action: "dry-run",
              scene: scenePath,
              node_path: nodePath,
              updated_properties: Object.keys(properties),
            }),
            globals.human
          );
          return;
        }

        writeFileSync(scenePath, generateTscn(updated));

        const result = {
          scene: scenePath,
          res_path: `res://${relative(projectDir, scenePath)}`,
          node_path: nodePath,
          updated_properties: Object.keys(properties),
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

  // ── node connect ──────────────────────────────────────

  cmd
    .command("connect")
    .description("Add a signal connection in a scene")
    .option("--scene <scene>", "Scene name or path")
    .option("--signal <signal>", "Signal name")
    .option("--from <from>", "Source node path")
    .option("--to <to>", "Target node path")
    .option("--method <method>", "Handler method name")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "node.connect";

      try {
        const input = await readInput<{ scene?: string; signal?: string; from?: string; to?: string; method?: string }>(globals.input);
        const sceneName = opts.scene ?? input?.scene;
        const sig = opts.signal ?? input?.signal;
        const from = opts.from ?? input?.from;
        const to = opts.to ?? input?.to;
        const method = opts.method ?? input?.method;

        if (!sceneName || !sig || !from || !to || !method) {
          output(error(commandName, "MISSING_FIELDS", "scene, signal, from, to, and method are all required"), globals.human);
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);
        const doc = readScene(scenePath, commandName, globals.human);

        const connection = { signal: sig, from, to, method };
        const updated = addConnection(doc, connection);

        if (globals.dryRun) {
          output(success(commandName, { action: "dry-run", scene: scenePath, connection }), globals.human);
          return;
        }

        writeFileSync(scenePath, generateTscn(updated));
        sendEditorCommand(projectDir, { action: "reload_scene" });

        output(
          success(commandName, {
            scene: scenePath,
            res_path: `res://${relative(projectDir, scenePath)}`,
            connection,
            total_connections: updated.connections.length,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "CONNECT_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── node disconnect ─────────────────────────────────────

  cmd
    .command("disconnect")
    .description("Remove a signal connection from a scene")
    .option("--scene <scene>", "Scene name or path")
    .option("--signal <signal>", "Signal name")
    .option("--from <from>", "Source node path")
    .option("--to <to>", "Target node path")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "node.disconnect";

      try {
        const input = await readInput<{ scene?: string; signal?: string; from?: string; to?: string }>(globals.input);
        const sceneName = opts.scene ?? input?.scene;
        const sig = opts.signal ?? input?.signal;
        const from = opts.from ?? input?.from;
        const to = opts.to ?? input?.to;

        if (!sceneName || !sig || !from || !to) {
          output(error(commandName, "MISSING_FIELDS", "scene, signal, from, and to are all required"), globals.human);
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);
        const doc = readScene(scenePath, commandName, globals.human);

        const updated = removeConnection(doc, sig, from, to);

        if (updated.connections.length === doc.connections.length) {
          output(
            error(commandName, "CONNECTION_NOT_FOUND", `No connection found for signal="${sig}" from="${from}" to="${to}"`),
            globals.human
          );
          process.exit(1);
        }

        if (globals.dryRun) {
          output(success(commandName, { action: "dry-run", scene: scenePath, signal: sig, from, to }), globals.human);
          return;
        }

        writeFileSync(scenePath, generateTscn(updated));
        sendEditorCommand(projectDir, { action: "reload_scene" });

        output(
          success(commandName, {
            scene: scenePath,
            res_path: `res://${relative(projectDir, scenePath)}`,
            removed: { signal: sig, from, to },
            remaining_connections: updated.connections.length,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "DISCONNECT_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── node connections ────────────────────────────────────

  cmd
    .command("connections")
    .description("List all signal connections in a scene")
    .option("--scene <scene>", "Scene name or path")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "node.connections";

      try {
        const input = await readInput<{ scene?: string }>(globals.input);
        const sceneName = opts.scene ?? input?.scene;

        if (!sceneName) {
          output(
            error(commandName, "MISSING_SCENE", "Scene name is required", 'Pass --scene <scene> or provide {"scene": "..."} via stdin'),
            globals.human
          );
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);
        const doc = readScene(scenePath, commandName, globals.human);

        output(
          success(commandName, {
            scene: scenePath,
            res_path: `res://${relative(projectDir, scenePath)}`,
            count: doc.connections.length,
            connections: doc.connections,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "CONNECTIONS_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── node move ───────────────────────────────────────────

  cmd
    .command("move")
    .description("Move a node to a new parent")
    .option("--scene <scene>", "Scene name or path")
    .option("--path <path>", "Node path to move")
    .option("--new-parent <parent>", "New parent path")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "node.move";

      try {
        const input = await readInput<{ scene?: string; path?: string; new_parent?: string }>(globals.input);
        const sceneName = opts.scene ?? input?.scene;
        const nodePath = opts.path ?? input?.path;
        const newParent = opts.newParent ?? input?.new_parent;

        if (!sceneName || !nodePath || !newParent) {
          output(error(commandName, "MISSING_FIELDS", "scene, path, and new-parent are all required"), globals.human);
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);
        const doc = readScene(scenePath, commandName, globals.human);

        const targetIdx = doc.nodes.findIndex((n) => getNodePath(n) === nodePath);
        if (targetIdx === -1) {
          output(
            error(commandName, "NODE_NOT_FOUND", `Node not found at path: ${nodePath}`, "Use 'node list' to see available nodes"),
            globals.human
          );
          process.exit(1);
        }

        const targetNode = doc.nodes[targetIdx];
        const oldPath = getNodePath(targetNode);
        const updatedNodes = [...doc.nodes];

        // Update the target node's parent
        updatedNodes[targetIdx] = { ...targetNode, parent: newParent };

        // Update children whose parent starts with the old path
        for (let i = 0; i < updatedNodes.length; i++) {
          if (i === targetIdx) continue;
          const n = updatedNodes[i];
          if (n.parent === oldPath) {
            const newChildParent = newParent === "." ? targetNode.name : `${newParent}/${targetNode.name}`;
            updatedNodes[i] = { ...n, parent: newChildParent };
          } else if (n.parent?.startsWith(oldPath + "/")) {
            const suffix = n.parent.slice(oldPath.length);
            const newChildParent = newParent === "." ? targetNode.name + suffix : `${newParent}/${targetNode.name}${suffix}`;
            updatedNodes[i] = { ...n, parent: newChildParent };
          }
        }

        const updated = { ...doc, nodes: updatedNodes };
        const newPath = newParent === "." ? targetNode.name : `${newParent}/${targetNode.name}`;

        if (globals.dryRun) {
          output(success(commandName, { action: "dry-run", scene: scenePath, old_path: oldPath, new_path: newPath }), globals.human);
          return;
        }

        writeFileSync(scenePath, generateTscn(updated));
        sendEditorCommand(projectDir, { action: "reload_scene" });

        output(
          success(commandName, {
            scene: scenePath,
            res_path: `res://${relative(projectDir, scenePath)}`,
            old_path: oldPath,
            new_path: newPath,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "MOVE_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  // ── node group ──────────────────────────────────────────

  const groupCmd = new Command("group").description("Node group management");

  groupCmd
    .command("add")
    .description("Add node to groups")
    .option("--scene <scene>", "Scene name or path")
    .option("--path <path>", "Node path")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "node.group.add";

      try {
        const input = await readInput<{ scene?: string; path?: string; groups?: string[] }>(globals.input);
        const sceneName = opts.scene ?? input?.scene;
        const nodePath = opts.path ?? input?.path;
        const groups = input?.groups;

        if (!sceneName || !nodePath) {
          output(error(commandName, "MISSING_FIELDS", "scene and path are required"), globals.human);
          process.exit(1);
        }
        if (!groups || groups.length === 0) {
          output(error(commandName, "MISSING_GROUPS", "groups array is required via stdin/--input"), globals.human);
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);
        const doc = readScene(scenePath, commandName, globals.human);

        const targetIdx = doc.nodes.findIndex((n) => getNodePath(n) === nodePath);
        if (targetIdx === -1) {
          output(
            error(commandName, "NODE_NOT_FOUND", `Node not found at path: ${nodePath}`, "Use 'node list' to see available nodes"),
            globals.human
          );
          process.exit(1);
        }

        const updatedNodes = [...doc.nodes];
        const existing = updatedNodes[targetIdx].groups ?? [];
        const merged = [...new Set([...existing, ...groups])];
        updatedNodes[targetIdx] = { ...updatedNodes[targetIdx], groups: merged };

        const updated = { ...doc, nodes: updatedNodes };

        if (globals.dryRun) {
          output(success(commandName, { action: "dry-run", scene: scenePath, node_path: nodePath, groups: merged }), globals.human);
          return;
        }

        writeFileSync(scenePath, generateTscn(updated));

        output(
          success(commandName, {
            scene: scenePath,
            res_path: `res://${relative(projectDir, scenePath)}`,
            node_path: nodePath,
            groups: merged,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "GROUP_ADD_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  groupCmd
    .command("remove")
    .description("Remove node from a group")
    .option("--scene <scene>", "Scene name or path")
    .option("--path <path>", "Node path")
    .option("--group <group>", "Group name to remove")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "node.group.remove";

      try {
        const input = await readInput<{ scene?: string; path?: string; group?: string }>(globals.input);
        const sceneName = opts.scene ?? input?.scene;
        const nodePath = opts.path ?? input?.path;
        const group = opts.group ?? input?.group;

        if (!sceneName || !nodePath || !group) {
          output(error(commandName, "MISSING_FIELDS", "scene, path, and group are all required"), globals.human);
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);
        const doc = readScene(scenePath, commandName, globals.human);

        const targetIdx = doc.nodes.findIndex((n) => getNodePath(n) === nodePath);
        if (targetIdx === -1) {
          output(
            error(commandName, "NODE_NOT_FOUND", `Node not found at path: ${nodePath}`, "Use 'node list' to see available nodes"),
            globals.human
          );
          process.exit(1);
        }

        const updatedNodes = [...doc.nodes];
        const existing = updatedNodes[targetIdx].groups ?? [];
        const filtered = existing.filter((g) => g !== group);
        updatedNodes[targetIdx] = { ...updatedNodes[targetIdx], groups: filtered.length > 0 ? filtered : undefined };

        const updated = { ...doc, nodes: updatedNodes };

        if (globals.dryRun) {
          output(success(commandName, { action: "dry-run", scene: scenePath, node_path: nodePath, removed: group }), globals.human);
          return;
        }

        writeFileSync(scenePath, generateTscn(updated));

        output(
          success(commandName, {
            scene: scenePath,
            res_path: `res://${relative(projectDir, scenePath)}`,
            node_path: nodePath,
            removed: group,
            remaining_groups: filtered,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "GROUP_REMOVE_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  groupCmd
    .command("list")
    .description("List groups for a node")
    .option("--scene <scene>", "Scene name or path")
    .option("--path <path>", "Node path")
    .action(async (opts, command) => {
      const globals = command.optsWithGlobals() as GlobalOptions;
      const commandName = "node.group.list";

      try {
        const input = await readInput<{ scene?: string; path?: string }>(globals.input);
        const sceneName = opts.scene ?? input?.scene;
        const nodePath = opts.path ?? input?.path;

        if (!sceneName || !nodePath) {
          output(error(commandName, "MISSING_FIELDS", "scene and path are required"), globals.human);
          process.exit(1);
        }

        const projectDir = resolve(globals.project ?? ".");
        const scenePath = resolveScenePath(projectDir, sceneName);
        const doc = readScene(scenePath, commandName, globals.human);

        const target = doc.nodes.find((n) => getNodePath(n) === nodePath);
        if (!target) {
          output(
            error(commandName, "NODE_NOT_FOUND", `Node not found at path: ${nodePath}`, "Use 'node list' to see available nodes"),
            globals.human
          );
          process.exit(1);
        }

        const groups = target.groups ?? [];

        output(
          success(commandName, {
            scene: scenePath,
            res_path: `res://${relative(projectDir, scenePath)}`,
            node_path: nodePath,
            count: groups.length,
            groups,
          }),
          globals.human
        );
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        output(error(commandName, "GROUP_LIST_FAILED", msg), globals.human);
        process.exit(1);
      }
    });

  cmd.addCommand(groupCmd);

  return cmd;
}
