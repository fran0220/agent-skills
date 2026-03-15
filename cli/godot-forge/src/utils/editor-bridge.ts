/**
 * Bridge to Godot editor via ForgeSync TCP server.
 *
 * Protocol: CLI connects to 127.0.0.1:GODOT_FORGE_PORT (default 23685),
 * sends a JSON line, optionally reads a JSON response line.
 *
 * Fallback: If TCP connection fails (editor not open), writes to
 * .godot-forge/commands.json for legacy file-based polling.
 */

import { Socket } from "node:net";
import { writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { randomUUID } from "node:crypto";

export const DEFAULT_EDITOR_PORT = 23685;

export interface EditorCommand {
  action: "open_scene" | "reload_scene" | "select_node" | "scan" | "run_scene" | "stop";
  path?: string;
}

export interface EditorResponse {
  id: string;
  ok: boolean;
  data?: Record<string, unknown>;
  error?: string;
}

function getEditorPort(): number {
  return parseInt(process.env.GODOT_FORGE_PORT ?? String(DEFAULT_EDITOR_PORT), 10);
}

/**
 * Write command to legacy .godot-forge/commands.json (fallback).
 */
function legacySendCommand(projectDir: string, command: EditorCommand): void {
  const dir = join(projectDir, ".godot-forge");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "commands.json"), JSON.stringify(command, null, 2));
}

/**
 * Send a fire-and-forget command to the editor via TCP.
 * Falls back to legacy file-based approach if TCP fails.
 */
export function sendEditorCommand(projectDir: string, command: EditorCommand): void {
  const port = getEditorPort();
  const id = randomUUID();
  const msg = JSON.stringify({ id, action: command.action, params: { path: command.path } }) + "\n";

  const socket = new Socket();
  let connected = false;

  socket.setTimeout(2000);

  socket.on("connect", () => {
    connected = true;
    socket.write(msg, () => {
      socket.destroy();
    });
  });

  socket.on("error", () => {
    if (!connected) {
      legacySendCommand(projectDir, command);
    }
    socket.destroy();
  });

  socket.on("timeout", () => {
    if (!connected) {
      legacySendCommand(projectDir, command);
    }
    socket.destroy();
  });

  socket.connect(port, "127.0.0.1");
}

/**
 * Send multiple commands in sequence.
 */
export function sendEditorCommands(projectDir: string, commands: EditorCommand[]): void {
  for (const cmd of commands) {
    sendEditorCommand(projectDir, cmd);
  }
}

/**
 * Query the editor and wait for a response (async).
 * Returns null if the editor is not connected.
 */
export function queryEditor(
  command: EditorCommand,
  timeout = 3000,
): Promise<EditorResponse | null> {
  const port = getEditorPort();
  const id = randomUUID();
  const msg = JSON.stringify({ id, action: command.action, params: { path: command.path } }) + "\n";

  return new Promise((resolve) => {
    const socket = new Socket();
    let buffer = "";
    let resolved = false;

    const done = (value: EditorResponse | null) => {
      if (!resolved) {
        resolved = true;
        resolve(value);
      }
    };

    socket.setTimeout(timeout);

    socket.on("connect", () => {
      socket.write(msg);
    });

    socket.on("data", (chunk) => {
      buffer += chunk.toString();
      const newlineIdx = buffer.indexOf("\n");
      if (newlineIdx !== -1) {
        const line = buffer.slice(0, newlineIdx);
        try {
          const response = JSON.parse(line) as EditorResponse;
          done(response);
        } catch {
          done(null);
        }
        socket.destroy();
      }
    });

    socket.on("error", () => {
      done(null);
      socket.destroy();
    });

    socket.on("timeout", () => {
      done(null);
      socket.destroy();
    });

    socket.on("close", () => {
      done(null);
    });

    socket.connect(port, "127.0.0.1");
  });
}
