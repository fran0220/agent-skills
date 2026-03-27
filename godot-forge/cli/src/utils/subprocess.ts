/**
 * Subprocess wrapper for Godot engine calls (L3).
 */

import { execFileSync, type ExecFileSyncOptions } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { readdirSync } from "node:fs";

export interface GodotExecResult {
  stdout: string;
  exitCode: number;
}

function tryExec(command: string, args: string[]): boolean {
  try {
    execFileSync(command, args, { stdio: "pipe", timeout: 5000 });
    return true;
  } catch {
    return false;
  }
}

function tryPath(path: string): boolean {
  return existsSync(path) && tryExec(path, ["--version"]);
}

/**
 * Find the Godot binary in PATH or known locations.
 * Supports macOS, Linux, and Windows.
 */
export function findGodot(): string {
  const platform = process.platform;

  // 1. Try common binary names directly (cross-platform, no which/where needed)
  const names = ["godot", "godot4", "Godot_v4"];
  for (const name of names) {
    if (tryExec(name, ["--version"])) return name;
  }

  // 2. Platform-specific paths
  if (platform === "darwin") {
    const macPaths = [
      "/Applications/Godot.app/Contents/MacOS/Godot",
      "/Applications/Godot_mono.app/Contents/MacOS/Godot",
    ];
    for (const p of macPaths) {
      if (tryPath(p)) return p;
    }
  }

  if (platform === "linux") {
    // Standard paths
    const linuxPaths = [
      "/usr/bin/godot",
      "/usr/local/bin/godot",
      "/usr/bin/godot4",
      "/snap/bin/godot",
    ];
    for (const p of linuxPaths) {
      if (tryPath(p)) return p;
    }

    // Flatpak
    if (tryExec("flatpak", ["run", "org.godotengine.Godot", "--version"])) {
      return "flatpak";
    }

    // Snap via snap run
    if (tryExec("snap", ["run", "godot", "--version"])) {
      return "snap";
    }

    // AppImage in ~/Applications/
    const appImageDir = join(homedir(), "Applications");
    if (existsSync(appImageDir)) {
      try {
        const entries = readdirSync(appImageDir);
        const appImage = entries.find(
          (e) => /^Godot.*\.AppImage$/i.test(e)
        );
        if (appImage) {
          const fullPath = join(appImageDir, appImage);
          if (tryPath(fullPath)) return fullPath;
        }
      } catch {
        // ignore read errors
      }
    }
  }

  if (platform === "win32") {
    const localAppData = process.env.LOCALAPPDATA ?? "";
    const winPaths = [
      "C:\\Program Files\\Godot\\godot.exe",
      join(localAppData, "Godot", "godot.exe"),
    ];
    for (const p of winPaths) {
      if (tryPath(p)) return p;
    }
  }

  throw new Error(
    "Godot not found in PATH. Install Godot 4.4+ and ensure 'godot' is in PATH."
  );
}

/**
 * Build the command and args for running Godot, handling Flatpak/Snap wrappers.
 */
function resolveGodotCommand(
  godot: string,
  extraArgs: string[]
): { command: string; args: string[] } {
  if (godot === "flatpak") {
    return {
      command: "flatpak",
      args: ["run", "org.godotengine.Godot", ...extraArgs],
    };
  }
  if (godot === "snap") {
    return {
      command: "snap",
      args: ["run", "godot", ...extraArgs],
    };
  }
  return { command: godot, args: extraArgs };
}

/**
 * Run a GDScript file via godot --headless.
 */
export function runGodotScript(
  scriptPath: string,
  projectPath?: string
): GodotExecResult {
  const godot = findGodot();
  const baseArgs = ["--headless", "-s", scriptPath];
  if (projectPath) {
    baseArgs.unshift("--path", projectPath);
  }
  const { command, args } = resolveGodotCommand(godot, baseArgs);

  const opts: ExecFileSyncOptions = {
    cwd: projectPath,
    stdio: "pipe",
    timeout: 60_000,
  };

  try {
    const stdout = execFileSync(command, args, opts);
    return { stdout: stdout.toString(), exitCode: 0 };
  } catch (e: unknown) {
    const err = e as { stdout?: Buffer; stderr?: Buffer; status?: number };
    return {
      stdout: (err.stdout?.toString() ?? "") + (err.stderr?.toString() ?? ""),
      exitCode: err.status ?? 1,
    };
  }
}
