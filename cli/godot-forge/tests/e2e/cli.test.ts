/**
 * End-to-end tests for godot-forge CLI.
 *
 * Runs the CLI as a subprocess and verifies JSON output.
 */

import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, existsSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { describe, it, expect, beforeAll, beforeEach, afterEach } from "vitest";

const CLI_DIR = join(import.meta.dirname, "../..");
const CLI_ENTRY = join(CLI_DIR, "dist/index.js");

/** Run CLI and return parsed result */
function run(
  args: string[],
  stdin?: string
): { stdout: string; exitCode: number } {
  try {
    const result = execFileSync("node", [CLI_ENTRY, ...args], {
      cwd: CLI_DIR,
      input: stdin,
      stdio: ["pipe", "pipe", "pipe"],
      env: { ...process.env, NODE_OPTIONS: "" },
      timeout: 15_000,
    });
    return { stdout: result.toString(), exitCode: 0 };
  } catch (e: any) {
    return {
      stdout: (e.stdout?.toString() ?? "") + (e.stderr?.toString() ?? ""),
      exitCode: e.status ?? 1,
    };
  }
}

/** Run CLI and parse JSON output */
function runJSON(args: string[], stdin?: string) {
  const { stdout, exitCode } = run(args, stdin);
  try {
    const json = JSON.parse(stdout);
    return { json, exitCode };
  } catch {
    throw new Error(
      `Failed to parse JSON output (exit ${exitCode}):\n${stdout}`
    );
  }
}

beforeAll(() => {
  execFileSync("npm", ["run", "build"], { cwd: CLI_DIR, stdio: "pipe" });
});

// ─── Smoke tests ───────────────────────────────────────────────

describe("smoke tests", () => {
  it("--version outputs version string", () => {
    const { stdout, exitCode } = run(["--version"]);
    expect(exitCode).toBe(0);
    expect(stdout.trim()).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it("describe --all lists all commands", () => {
    const { json, exitCode } = runJSON(["describe", "--all"]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.command).toBe("describe");
    expect(json.data.commands).toBeInstanceOf(Array);
    expect(json.data.commands.length).toBeGreaterThan(0);
    expect(json.data.schemas).toBeDefined();
  });

  it("describe project.init returns schema", () => {
    const { json, exitCode } = runJSON(["describe", "project.init"]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.command).toBe("project init");
    expect(json.data.input).toBeDefined();
    expect(json.data.output).toBeDefined();
  });
});

// ─── Project flow ──────────────────────────────────────────────

describe("project flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("project init creates project.godot", () => {
    const { json, exitCode } = runJSON([
      "project",
      "init",
      "--name",
      "test-game",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.command).toBe("project.init");
    expect(json.data.name).toBe("test-game");
    expect(json.data.path).toBe(tmpDir);
    expect(json.data.files_created).toContain("project.godot");
    expect(existsSync(join(tmpDir, "project.godot"))).toBe(true);
    expect(existsSync(join(tmpDir, "scenes"))).toBe(true);
    expect(existsSync(join(tmpDir, "scripts"))).toBe(true);
  });

  it("project info returns name and version", () => {
    // Init first
    runJSON(["project", "init", "--name", "info-test", "--project", tmpDir]);

    const { json, exitCode } = runJSON([
      "project",
      "info",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.command).toBe("project.info");
    expect(json.data.name).toBe("info-test");
    expect(json.data.godot_version).toBe("4.4");
    expect(json.data.path).toBe(tmpDir);
  });

  it("project init without name returns MISSING_NAME error", () => {
    const { json, exitCode } = runJSON([
      "project",
      "init",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("MISSING_NAME");
  });
});

// ─── Scene flow ────────────────────────────────────────────────

describe("scene flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "scene-test", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("scene create with stdin JSON creates .tscn file", () => {
    const input = JSON.stringify({
      name: "Main",
      root_type: "Node2D",
    });
    const { json, exitCode } = runJSON(
      ["scene", "create", "--project", tmpDir],
      input
    );
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.command).toBe("scene.create");
    expect(json.data.root_type).toBe("Node2D");
    expect(json.data.root_name).toBe("Main");
    expect(json.data.node_count).toBeGreaterThanOrEqual(1);
    expect(existsSync(join(tmpDir, "scenes", "Main.tscn"))).toBe(true);
  });

  it("scene list lists created scene", () => {
    runJSON(
      ["scene", "create", "--name", "Level1", "--project", tmpDir],
    );

    const { json, exitCode } = runJSON([
      "scene",
      "list",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.count).toBe(1);
    expect(json.data.scenes[0].name).toBe("Level1");
  });

  it("scene read returns scene structure", () => {
    runJSON(
      ["scene", "create", "--name", "ReadTest", "--root-type", "Node3D", "--project", tmpDir],
    );

    const { json, exitCode } = runJSON([
      "scene",
      "read",
      "ReadTest",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.command).toBe("scene.read");
    expect(json.data.scene).toBeDefined();
    expect(json.data.scene.nodes).toBeInstanceOf(Array);
    expect(json.data.scene.nodes[0].name).toBe("ReadTest");
    expect(json.data.scene.nodes[0].type).toBe("Node3D");
  });

  it("scene delete --force removes scene file", () => {
    runJSON(
      ["scene", "create", "--name", "ToDelete", "--project", tmpDir],
    );
    expect(existsSync(join(tmpDir, "scenes", "ToDelete.tscn"))).toBe(true);

    const { json, exitCode } = runJSON([
      "--force",
      "scene",
      "delete",
      "ToDelete",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.deleted).toBe(true);
    expect(existsSync(join(tmpDir, "scenes", "ToDelete.tscn"))).toBe(false);
  });

  it("scene delete without --force returns FORCE_REQUIRED", () => {
    runJSON(
      ["scene", "create", "--name", "Protected", "--project", tmpDir],
    );

    const { json, exitCode } = runJSON([
      "scene",
      "delete",
      "Protected",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("FORCE_REQUIRED");
  });
});

// ─── Script flow ───────────────────────────────────────────────

describe("script flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "script-test", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("script create with stdin JSON creates .gd file", () => {
    const input = JSON.stringify({
      name: "player",
      extends: "CharacterBody2D",
      methods: ["_ready", "_process"],
      signals: ["health_changed"],
      exports: [{ name: "speed", type: "float", default: 200.0 }],
    });
    const { json, exitCode } = runJSON(
      ["script", "create", "--project", tmpDir],
      input
    );
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.command).toBe("script.create");
    expect(json.data.class_name).toBe("Player");
    expect(json.data.extends).toBe("CharacterBody2D");
    expect(json.data.methods).toContain("_ready");
    expect(json.data.signals).toContain("health_changed");
    expect(existsSync(join(tmpDir, "scripts", "player.gd"))).toBe(true);
  });

  it("script list lists created scripts", () => {
    const input = JSON.stringify({ name: "enemy", extends: "Node2D" });
    runJSON(["script", "create", "--project", tmpDir], input);

    const { json, exitCode } = runJSON([
      "script",
      "list",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.count).toBeGreaterThanOrEqual(1);
    const enemyScript = json.data.scripts.find((s: any) => s.path.includes("enemy.gd"));
    expect(enemyScript).toBeDefined();
    expect(enemyScript.class_name).toBe("Enemy");
  });
});

// ─── Resource flow ─────────────────────────────────────────────

describe("resource flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "res-test", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("resource create with stdin JSON creates .tres file", () => {
    const input = JSON.stringify({
      type: "Environment",
      name: "default_env",
      path: "resources/default_env.tres",
      properties: { background_mode: 1 },
    });
    const { json, exitCode } = runJSON(
      ["resource", "create", "--project", tmpDir],
      input
    );
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.command).toBe("resource.create");
    expect(json.data.type).toBe("Environment");
    expect(json.data.name).toBe("default_env");
    expect(
      existsSync(join(tmpDir, "resources", "default_env.tres"))
    ).toBe(true);
  });

  it("resource list returns categorized listing", () => {
    // Create a resource and a script so we have something to list
    const resInput = JSON.stringify({
      type: "Environment",
      name: "env",
      path: "resources/env.tres",
    });
    runJSON(["resource", "create", "--project", tmpDir], resInput);

    const scriptInput = JSON.stringify({ name: "test_script", extends: "Node" });
    runJSON(["script", "create", "--project", tmpDir], scriptInput);

    const { json, exitCode } = runJSON([
      "resource",
      "list",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.resources).toBeInstanceOf(Array);
    expect(json.data.scripts).toBeInstanceOf(Array);
    expect(json.data.resources.length).toBe(1);
    expect(json.data.scripts.length).toBeGreaterThanOrEqual(1);
  });

  it("resource check reports no missing refs", () => {
    const { json, exitCode } = runJSON([
      "resource",
      "check",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.missing_count).toBe(0);
    expect(json.data.missing).toEqual([]);
  });
});

// ─── Node flow ─────────────────────────────────────────────────

describe("node flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "node-test", "--project", tmpDir]);
    runJSON([
      "scene",
      "create",
      "--name",
      "NodeScene",
      "--root-type",
      "Node2D",
      "--project",
      tmpDir,
    ]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("node add adds a node to the scene", () => {
    const input = JSON.stringify({
      scene: "NodeScene",
      name: "Player",
      type: "CharacterBody2D",
      parent: ".",
    });
    const { json, exitCode } = runJSON(
      ["node", "add", "--project", tmpDir],
      input
    );
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.command).toBe("node.add");
    expect(json.data.node.name).toBe("Player");
    expect(json.data.node.type).toBe("CharacterBody2D");
    expect(json.data.node_count).toBe(2); // root + Player
  });

  it("node list lists all nodes including added one", () => {
    const addInput = JSON.stringify({
      scene: "NodeScene",
      name: "Sprite",
      type: "Sprite2D",
    });
    runJSON(["node", "add", "--project", tmpDir], addInput);

    const listInput = JSON.stringify({ scene: "NodeScene" });
    const { json, exitCode } = runJSON(
      ["node", "list", "--project", tmpDir],
      listInput
    );
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.count).toBe(2);
    const names = json.data.nodes.map((n: any) => n.name);
    expect(names).toContain("NodeScene");
    expect(names).toContain("Sprite");
  });
});

// ─── Global options ────────────────────────────────────────────

describe("global options", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "opts-test", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("--human flag produces non-JSON output", () => {
    const { stdout, exitCode } = run([
      "--human",
      "project",
      "info",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(0);
    expect(stdout).toContain("✓");
    expect(stdout).toContain("opts-test");
    // Should NOT be valid JSON
    expect(() => JSON.parse(stdout)).toThrow();
  });

  it("--fields filters output fields", () => {
    const { json, exitCode } = runJSON([
      "--fields",
      "name",
      "project",
      "info",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.name).toBe("opts-test");
    expect(json.data.path).toBeUndefined();
    expect(json.data.godot_version).toBeUndefined();
  });

  it("--dry-run shows plan without executing", () => {
    const newDir = join(tmpDir, "dry-run-proj");
    const { json, exitCode } = runJSON([
      "--dry-run",
      "project",
      "init",
      "--name",
      "dry-test",
      "--project",
      newDir,
    ]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.action).toBe("dry-run");
    expect(json.data.files).toBeInstanceOf(Array);
    // Should NOT have created the project
    expect(existsSync(join(newDir, "project.godot"))).toBe(false);
  });

  it("--force allows overwrite of existing project", () => {
    // First init
    runJSON(["project", "init", "--name", "first", "--project", tmpDir]);

    // Second init with --force
    const { json, exitCode } = runJSON([
      "--force",
      "project",
      "init",
      "--name",
      "second",
      "--project",
      tmpDir,
    ]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.name).toBe("second");

    // Verify project was overwritten
    const { json: info } = runJSON(["project", "info", "--project", tmpDir]);
    expect(info.data.name).toBe("second");
  });
});
