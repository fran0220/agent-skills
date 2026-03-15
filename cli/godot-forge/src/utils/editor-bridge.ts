/**
 * Bridge to Godot editor via ForgeSync plugin.
 * Writes command JSON to .godot-forge/commands.json, which the plugin polls.
 */

import { writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";

export interface EditorCommand {
  action: "open_scene" | "reload_scene" | "select_node" | "scan" | "run_scene" | "stop";
  path?: string;
}

/**
 * Send a command to the Godot editor via the ForgeSync plugin.
 */
export function sendEditorCommand(projectDir: string, command: EditorCommand): void {
  const dir = join(projectDir, ".godot-forge");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "commands.json"), JSON.stringify(command, null, 2));
}

/**
 * Send multiple commands in sequence.
 */
export function sendEditorCommands(projectDir: string, commands: EditorCommand[]): void {
  const dir = join(projectDir, ".godot-forge");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "commands.json"), JSON.stringify(commands, null, 2));
}
