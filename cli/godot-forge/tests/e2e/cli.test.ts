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

// ─── Scene update ──────────────────────────────────────────────

describe("scene update", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "upd-test", "--project", tmpDir]);
    runJSON(["scene", "create", "--name", "UpdScene", "--root-type", "Node2D", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("scene update changes root type", () => {
    const input = JSON.stringify({ scene: "UpdScene", root_type: "Node3D" });
    const { json, exitCode } = runJSON(["scene", "update", "--project", tmpDir], input);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.changes.root_type.from).toBe("Node2D");
    expect(json.data.changes.root_type.to).toBe("Node3D");
  });

  it("scene update changes root name", () => {
    const input = JSON.stringify({ scene: "UpdScene", root_name: "NewRoot" });
    const { json, exitCode } = runJSON(["scene", "update", "--project", tmpDir], input);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.changes.root_name.from).toBe("UpdScene");
    expect(json.data.changes.root_name.to).toBe("NewRoot");
  });

  it("scene update with no changes returns NO_CHANGES error", () => {
    const input = JSON.stringify({ scene: "UpdScene" });
    const { json, exitCode } = runJSON(["scene", "update", "--project", tmpDir], input);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("NO_CHANGES");
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

  it("script validate checks a valid script", () => {
    runJSON(["script", "create", "--project", tmpDir], JSON.stringify({ name: "valid_check", extends: "Node" }));
    const { json, exitCode } = runJSON(["script", "validate", "scripts/valid_check.gd", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.valid).toBe(true);
  });

  it("script edit adds signals and methods", () => {
    runJSON(["script", "create", "--project", tmpDir], JSON.stringify({ name: "editable", extends: "Node2D" }));
    const editInput = JSON.stringify({
      path: "scripts/editable.gd",
      add_signals: ["damage_taken"],
      add_methods: ["take_damage"],
    });
    const { json, exitCode } = runJSON(["script", "edit", "--project", tmpDir], editInput);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.added.signals).toContain("damage_taken");
    expect(json.data.added.methods).toContain("take_damage");
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

  it("resource read parses .tres file as JSON", () => {
    const resInput = JSON.stringify({
      type: "Environment",
      name: "read_env",
      path: "resources/read_env.tres",
      properties: { background_mode: 1, fog_enabled: true },
    });
    runJSON(["resource", "create", "--project", tmpDir], resInput);

    const { json, exitCode } = runJSON(["resource", "read", "resources/read_env.tres", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.type).toBe("Environment");
    expect(json.data.properties.background_mode).toBe(1);
    expect(json.data.properties.fog_enabled).toBe(true);
  });

  it("resource update modifies .tres properties", () => {
    const resInput = JSON.stringify({
      type: "Environment",
      name: "upd_env",
      path: "resources/upd_env.tres",
      properties: { background_mode: 0 },
    });
    runJSON(["resource", "create", "--project", tmpDir], resInput);

    const updInput = JSON.stringify({
      path: "resources/upd_env.tres",
      properties: { background_mode: 2, fog_enabled: true },
    });
    const { json, exitCode } = runJSON(["resource", "update", "--project", tmpDir], updInput);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.updated_keys).toContain("background_mode");
    expect(json.data.updated_keys).toContain("fog_enabled");
    expect(json.data.properties.background_mode).toBe(2);
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

  it("node remove removes a node from the scene", () => {
    runJSON(["node", "add", "--project", tmpDir], JSON.stringify({ scene: "NodeScene", name: "ToRemove", type: "Sprite2D" }));
    const { json, exitCode } = runJSON(["node", "remove", "--project", tmpDir], JSON.stringify({ scene: "NodeScene", path: "ToRemove" }));
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.removed_path).toBe("ToRemove");
    expect(json.data.remaining_nodes).toBe(1); // only root
  });

  it("node remove non-existent node returns NODE_NOT_FOUND", () => {
    const { json, exitCode } = runJSON(["node", "remove", "--project", tmpDir], JSON.stringify({ scene: "NodeScene", path: "NonExistent" }));
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("NODE_NOT_FOUND");
  });

  it("node update modifies node properties", () => {
    runJSON(["node", "add", "--project", tmpDir], JSON.stringify({ scene: "NodeScene", name: "Updatable", type: "Sprite2D" }));
    const updInput = JSON.stringify({
      scene: "NodeScene",
      path: "Updatable",
      properties: { modulate: "Color(1, 0, 0, 1)" },
    });
    const { json, exitCode } = runJSON(["node", "update", "--project", tmpDir], updInput);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.node_path).toBe("Updatable");
    expect(json.data.updated_properties).toContain("modulate");
  });

  it("node move reparents a node", () => {
    runJSON(["node", "add", "--project", tmpDir], JSON.stringify({ scene: "NodeScene", name: "Parent", type: "Node2D" }));
    runJSON(["node", "add", "--project", tmpDir], JSON.stringify({ scene: "NodeScene", name: "Child", type: "Sprite2D" }));
    const moveInput = JSON.stringify({ scene: "NodeScene", path: "Child", new_parent: "Parent" });
    const { json, exitCode } = runJSON(["node", "move", "--project", tmpDir], moveInput);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.old_path).toBe("Child");
    expect(json.data.new_path).toBe("Parent/Child");
  });

  it("node connect adds a signal connection", () => {
    runJSON(["node", "add", "--project", tmpDir], JSON.stringify({ scene: "NodeScene", name: "Button", type: "Button" }));
    const connInput = JSON.stringify({
      scene: "NodeScene",
      signal: "pressed",
      from: "Button",
      to: ".",
      method: "_on_button_pressed",
    });
    const { json, exitCode } = runJSON(["node", "connect", "--project", tmpDir], connInput);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.connection.signal).toBe("pressed");
    expect(json.data.total_connections).toBe(1);
  });

  it("node disconnect removes a signal connection", () => {
    runJSON(["node", "add", "--project", tmpDir], JSON.stringify({ scene: "NodeScene", name: "Btn", type: "Button" }));
    runJSON(["node", "connect", "--project", tmpDir], JSON.stringify({
      scene: "NodeScene", signal: "pressed", from: "Btn", to: ".", method: "_on_btn_pressed",
    }));
    const discInput = JSON.stringify({ scene: "NodeScene", signal: "pressed", from: "Btn", to: "." });
    const { json, exitCode } = runJSON(["node", "disconnect", "--project", tmpDir], discInput);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.remaining_connections).toBe(0);
  });

  it("node connections lists signal connections", () => {
    runJSON(["node", "add", "--project", tmpDir], JSON.stringify({ scene: "NodeScene", name: "Sig", type: "Button" }));
    runJSON(["node", "connect", "--project", tmpDir], JSON.stringify({
      scene: "NodeScene", signal: "pressed", from: "Sig", to: ".", method: "_on_sig_pressed",
    }));
    const { json, exitCode } = runJSON(["node", "connections", "--project", tmpDir], JSON.stringify({ scene: "NodeScene" }));
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.count).toBe(1);
    expect(json.data.connections[0].signal).toBe("pressed");
  });
});

// ─── Node group flow ───────────────────────────────────────────

describe("node group flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "grp-test", "--project", tmpDir]);
    runJSON(["scene", "create", "--name", "GrpScene", "--root-type", "Node2D", "--project", tmpDir]);
    runJSON(["node", "add", "--project", tmpDir], JSON.stringify({ scene: "GrpScene", name: "Enemy", type: "CharacterBody2D" }));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("node group add assigns groups to a node", () => {
    const input = JSON.stringify({ scene: "GrpScene", path: "Enemy", groups: ["enemies", "damageable"] });
    const { json, exitCode } = runJSON(["node", "group", "add", "--project", tmpDir], input);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.groups).toContain("enemies");
    expect(json.data.groups).toContain("damageable");
  });

  it("node group list returns groups for a node", () => {
    runJSON(["node", "group", "add", "--project", tmpDir], JSON.stringify({ scene: "GrpScene", path: "Enemy", groups: ["enemies"] }));
    const { json, exitCode } = runJSON(["node", "group", "list", "--project", tmpDir], JSON.stringify({ scene: "GrpScene", path: "Enemy" }));
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.count).toBe(1);
    expect(json.data.groups).toContain("enemies");
  });

  it("node group remove removes a group from a node", () => {
    runJSON(["node", "group", "add", "--project", tmpDir], JSON.stringify({ scene: "GrpScene", path: "Enemy", groups: ["enemies", "damageable"] }));
    const { json, exitCode } = runJSON(["node", "group", "remove", "--project", tmpDir], JSON.stringify({ scene: "GrpScene", path: "Enemy", group: "enemies" }));
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.removed).toBe("enemies");
    expect(json.data.remaining_groups).toContain("damageable");
    expect(json.data.remaining_groups).not.toContain("enemies");
  });
});

// ─── Project config flow ───────────────────────────────────────

describe("project config flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "cfg-test", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("project config get retrieves a setting", () => {
    const { json, exitCode } = runJSON(["project", "config", "get", "application/config/name", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.value).toContain("cfg-test");
  });

  it("project config get missing key returns KEY_NOT_FOUND", () => {
    const { json, exitCode } = runJSON(["project", "config", "get", "application/nonexistent", "--project", tmpDir]);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("KEY_NOT_FOUND");
  });

  it("project config set writes a setting", () => {
    runJSON(["project", "config", "set", "display/window/size/viewport_width", "1920", "--project", tmpDir]);
    const { json, exitCode } = runJSON(["project", "config", "get", "display/window/size/viewport_width", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.data.value).toBe("1920");
  });

  it("project config list returns all sections", () => {
    const { json, exitCode } = runJSON(["project", "config", "list", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.sections).toBeDefined();
    expect(json.data.sections.application).toBeDefined();
  });

  it("project config list with section returns section keys", () => {
    const { json, exitCode } = runJSON(["project", "config", "list", "application", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.section).toBe("application");
    expect(json.data.settings).toBeDefined();
  });
});

// ─── Project autoload flow ─────────────────────────────────────

describe("project autoload flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "auto-test", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("project autoload add registers a singleton", () => {
    const input = JSON.stringify({ name: "GameManager", path: "res://scripts/game_manager.gd" });
    const { json, exitCode } = runJSON(["project", "autoload", "add", "--project", tmpDir], input);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.name).toBe("GameManager");
    expect(json.data.enabled).toBe(true);
  });

  it("project autoload list shows registered autoloads", () => {
    runJSON(["project", "autoload", "add", "--project", tmpDir], JSON.stringify({ name: "Global", path: "res://scripts/global.gd" }));
    const { json, exitCode } = runJSON(["project", "autoload", "list", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.count).toBeGreaterThanOrEqual(1);
    const found = json.data.autoloads.find((a: any) => a.name === "Global");
    expect(found).toBeDefined();
    expect(found.enabled).toBe(true);
  });

  it("project autoload remove unregisters a singleton", () => {
    runJSON(["project", "autoload", "add", "--project", tmpDir], JSON.stringify({ name: "ToRemove", path: "res://scripts/to_remove.gd" }));
    const { json, exitCode } = runJSON(["project", "autoload", "remove", "--name", "ToRemove", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.removed).toBe(true);
  });

  it("project autoload remove non-existent returns NOT_FOUND", () => {
    const { json, exitCode } = runJSON(["project", "autoload", "remove", "--name", "NonExistent", "--project", tmpDir]);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("NOT_FOUND");
  });
});

// ─── Project input flow ────────────────────────────────────────

describe("project input flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "input-test", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("project input add registers an input action", () => {
    const input = JSON.stringify({ name: "move_left", keys: ["A", "Left"], deadzone: 0.2 });
    const { json, exitCode } = runJSON(["project", "input", "add", "--project", tmpDir], input);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.name).toBe("move_left");
    expect(json.data.keys).toContain("A");
    expect(json.data.keys).toContain("Left");
  });

  it("project input add without keys returns MISSING_KEYS", () => {
    const input = JSON.stringify({ name: "empty_action", keys: [] });
    const { json, exitCode } = runJSON(["project", "input", "add", "--project", tmpDir], input);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("MISSING_KEYS");
  });

  it("project input list shows registered actions", () => {
    runJSON(["project", "input", "add", "--project", tmpDir], JSON.stringify({ name: "jump", keys: ["Space"] }));
    const { json, exitCode } = runJSON(["project", "input", "list", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.count).toBeGreaterThanOrEqual(1);
    const jump = json.data.actions.find((a: any) => a.name === "jump");
    expect(jump).toBeDefined();
    expect(jump.event_count).toBe(1);
  });

  it("project input remove removes an action", () => {
    runJSON(["project", "input", "add", "--project", tmpDir], JSON.stringify({ name: "to_remove", keys: ["Q"] }));
    const { json, exitCode } = runJSON(["project", "input", "remove", "--name", "to_remove", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.removed).toBe(true);
  });

  it("project input remove non-existent returns NOT_FOUND", () => {
    const { json, exitCode } = runJSON(["project", "input", "remove", "--name", "no_such_action", "--project", tmpDir]);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("NOT_FOUND");
  });
});

// ─── Project plugin flow ───────────────────────────────────────

describe("project plugin flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "plugin-test", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("project plugin list shows ForgeSync plugin enabled by default", () => {
    const { json, exitCode } = runJSON(["project", "plugin", "list", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.count).toBeGreaterThanOrEqual(1);
    expect(json.data.plugins).toContain("res://addons/forge_sync/plugin.cfg");
  });

  it("project plugin enable adds a plugin", () => {
    const { json, exitCode } = runJSON(
      ["project", "plugin", "enable", "--name", "res://addons/my_plugin/plugin.cfg", "--project", tmpDir],
    );
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.enabled).toBe(true);
  });

  it("project plugin enable already-enabled returns already_enabled", () => {
    const { json, exitCode } = runJSON(
      ["project", "plugin", "enable", "--name", "res://addons/forge_sync/plugin.cfg", "--project", tmpDir],
    );
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.already_enabled).toBe(true);
  });

  it("project plugin disable removes a plugin", () => {
    runJSON(["project", "plugin", "enable", "--name", "res://addons/temp/plugin.cfg", "--project", tmpDir]);
    const { json, exitCode } = runJSON(
      ["project", "plugin", "disable", "--name", "res://addons/temp/plugin.cfg", "--project", tmpDir],
    );
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.disabled).toBe(true);
  });

  it("project plugin disable non-enabled returns NOT_FOUND", () => {
    const { json, exitCode } = runJSON(
      ["project", "plugin", "disable", "--name", "res://addons/nope/plugin.cfg", "--project", tmpDir],
    );
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("NOT_FOUND");
  });
});

// ─── Export preset flow ────────────────────────────────────────

describe("export preset flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-test-"));
    runJSON(["project", "init", "--name", "export-test", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("export preset add creates a new preset", () => {
    const input = JSON.stringify({ name: "Linux", platform: "Linux", path: "export/game.x86_64" });
    const { json, exitCode } = runJSON(["export", "preset", "add", "--project", tmpDir], input);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.operation).toBe("added");
    expect(json.data.preset.name).toBe("Linux");
    expect(json.data.preset.platform).toBe("Linux");
  });

  it("export preset list lists created presets", () => {
    runJSON(["export", "preset", "add", "--project", tmpDir], JSON.stringify({ name: "Windows", platform: "Windows Desktop", path: "export/game.exe" }));
    const { json, exitCode } = runJSON(["export", "preset", "list", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.count).toBe(1);
    expect(json.data.presets[0].name).toBe("Windows");
  });

  it("export preset add duplicate without --force returns PRESET_EXISTS", () => {
    const input = JSON.stringify({ name: "Dup", platform: "Linux" });
    runJSON(["export", "preset", "add", "--project", tmpDir], input);
    const { json, exitCode } = runJSON(["export", "preset", "add", "--project", tmpDir], input);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("PRESET_EXISTS");
  });
});

// ─── Project validate flow ─────────────────────────────────────

describe("project validate flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-validate-"));
    runJSON(["project", "init", "--name", "validate-test", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("project validate on clean project returns valid", () => {
    const { json, exitCode } = runJSON(["project", "validate", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.valid).toBe(true);
  });
});

// ─── Project clean flow ───────────────────────────────────────

describe("project clean flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-clean-"));
    runJSON(["project", "init", "--name", "clean-test", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("project clean on clean project finds no orphans", () => {
    const { json, exitCode } = runJSON(["project", "clean", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.orphaned_count).toBe(0);
  });

  it("project clean finds orphaned script", () => {
    // Create an unreferenced script
    const { writeFileSync, mkdirSync } = require("node:fs");
    const { join } = require("node:path");
    mkdirSync(join(tmpDir, "scripts"), { recursive: true });
    writeFileSync(join(tmpDir, "scripts", "orphan.gd"), "extends Node\n");

    const { json, exitCode } = runJSON(["project", "clean", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.data.orphaned_count).toBeGreaterThan(0);
    expect(json.data.orphaned.some((f: string) => f.includes("orphan.gd"))).toBe(true);
  });
});

// ─── Project template flow ────────────────────────────────────

describe("project template flow", () => {
  it("project template list returns templates", () => {
    const { json, exitCode } = runJSON(["project", "template", "list"]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.count).toBeGreaterThan(0);
    expect(json.data.templates).toBeInstanceOf(Array);
    expect(json.data.templates[0].name).toBe("default");
  });
});

// ─── Script read flow ─────────────────────────────────────────

describe("script read flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-sread-"));
    runJSON(["project", "init", "--name", "sread-test", "--project", tmpDir]);
    // Create a script to read
    runJSON(["script", "create", "--project", tmpDir], JSON.stringify({
      name: "player",
      extends: "CharacterBody2D",
      signals: ["hit", "died"],
      methods: ["_ready", "_process"],
      exports: [{ name: "speed", type: "float", default: 200.0 }],
    }));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("script read returns parsed structure", () => {
    const { json, exitCode } = runJSON(["script", "read", "scripts/player.gd", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.extends).toBe("CharacterBody2D");
    expect(json.data.class_name).toBe("Player");
    expect(json.data.signals).toContain("hit");
    expect(json.data.signals).toContain("died");
    expect(json.data.methods).toContain("_ready");
    expect(json.data.methods).toContain("_process");
    expect(json.data.exports.length).toBeGreaterThan(0);
    expect(json.data.line_count).toBeGreaterThan(0);
  });

  it("script read non-existent file returns error", () => {
    const { json, exitCode } = runJSON(["script", "read", "scripts/nope.gd", "--project", tmpDir]);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("FILE_NOT_FOUND");
  });
});

// ─── Node duplicate flow ──────────────────────────────────────

describe("node duplicate flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-ndup-"));
    runJSON(["project", "init", "--name", "ndup-test", "--project", tmpDir]);
    runJSON(["scene", "create", "--name", "dup_test", "--root-type", "Node2D", "--project", tmpDir]);
    runJSON(["node", "add", "--project", tmpDir], JSON.stringify({ scene: "dup_test", name: "Sprite", type: "Sprite2D" }));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("node duplicate creates copy of node", () => {
    const { json, exitCode } = runJSON(["node", "duplicate", "--project", tmpDir], JSON.stringify({
      scene: "dup_test",
      path: "Sprite",
      new_name: "Sprite2",
    }));
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.duplicated_name).toBe("Sprite2");
  });

  it("node duplicate auto-names when no new_name", () => {
    const { json, exitCode } = runJSON(["node", "duplicate", "--project", tmpDir], JSON.stringify({
      scene: "dup_test",
      path: "Sprite",
    }));
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.duplicated_name).toBe("Sprite2");
  });
});

// ─── Scene rename flow ────────────────────────────────────────

describe("scene rename flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-sren-"));
    runJSON(["project", "init", "--name", "sren-test", "--project", tmpDir]);
    runJSON(["scene", "create", "--name", "old_name", "--root-type", "Node2D", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("scene rename moves file and returns paths", () => {
    const { json, exitCode } = runJSON(["scene", "rename", "--project", tmpDir], JSON.stringify({
      scene: "old_name",
      new_name: "new_name",
    }));
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.new_res_path).toContain("new_name.tscn");
    // Old file should not exist
    const { existsSync } = require("node:fs");
    const { join } = require("node:path");
    expect(existsSync(join(tmpDir, "scenes", "old_name.tscn"))).toBe(false);
    expect(existsSync(join(tmpDir, "scenes", "new_name.tscn"))).toBe(true);
  });
});

// ─── Scene merge flow ─────────────────────────────────────────

describe("scene merge flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-smerge-"));
    runJSON(["project", "init", "--name", "smerge-test", "--project", tmpDir]);
    runJSON(["scene", "create", "--name", "target", "--root-type", "Node2D", "--project", tmpDir]);
    runJSON(["scene", "create", "--name", "source", "--root-type", "Node2D", "--project", tmpDir]);
    runJSON(["node", "add", "--project", tmpDir], JSON.stringify({ scene: "source", name: "Enemy", type: "CharacterBody2D" }));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("scene merge adds source nodes to target", () => {
    const { json, exitCode } = runJSON(["scene", "merge", "--project", tmpDir], JSON.stringify({
      target: "target",
      sources: ["source"],
    }));
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.nodes_added).toBeGreaterThan(0);

    // Verify target scene now has the nodes
    const { json: readJson } = runJSON(["node", "list", "--project", tmpDir], JSON.stringify({ scene: "target" }));
    expect(readJson.data.count).toBeGreaterThan(1);
  });
});

// ─── Resource delete flow ─────────────────────────────────────

describe("resource delete flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-rdel-"));
    runJSON(["project", "init", "--name", "rdel-test", "--project", tmpDir]);
    runJSON(["resource", "create", "--project", tmpDir], JSON.stringify({
      type: "RectangleShape2D",
      name: "rect_shape",
      path: "resources/rect.tres",
    }));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("resource delete without --force fails", () => {
    const { json, exitCode } = runJSON(["resource", "delete", "resources/rect.tres", "--project", tmpDir]);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("FORCE_REQUIRED");
  });

  it("resource delete with --force deletes file", () => {
    const { json, exitCode } = runJSON(["--force", "resource", "delete", "resources/rect.tres", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.deleted).toBe(true);

    const { existsSync } = require("node:fs");
    const { join } = require("node:path");
    expect(existsSync(join(tmpDir, "resources", "rect.tres"))).toBe(false);
  });
});

// ─── Export preset remove flow ────────────────────────────────

describe("export preset remove flow", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-eprem-"));
    runJSON(["project", "init", "--name", "eprem-test", "--project", tmpDir]);
    runJSON(["export", "preset", "add", "--name", "Linux", "--platform", "Linux", "--project", tmpDir]);
    runJSON(["export", "preset", "add", "--name", "Windows", "--platform", "Windows Desktop", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("export preset remove without --force fails", () => {
    const { json, exitCode } = runJSON(["export", "preset", "remove", "--name", "Linux", "--project", tmpDir]);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("FORCE_REQUIRED");
  });

  it("export preset remove with --force removes preset", () => {
    const { json, exitCode } = runJSON(["--force", "export", "preset", "remove", "--name", "Linux", "--project", tmpDir]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.removed).toBe(true);
    expect(json.data.remaining_count).toBe(1);

    // Verify it's gone
    const { json: listJson } = runJSON(["export", "preset", "list", "--project", tmpDir]);
    expect(listJson.data.count).toBe(1);
    expect(listJson.data.presets[0].name).toBe("Windows");
  });

  it("export preset remove non-existent returns error", () => {
    const { json, exitCode } = runJSON(["--force", "export", "preset", "remove", "--name", "NonExistent", "--project", tmpDir]);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("PRESET_NOT_FOUND");
  });
});

// ─── Node add auto sub_resource ───────────────────────────────

describe("node add auto sub_resource", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "gf-autosub-"));
    runJSON(["project", "init", "--name", "autosub-test", "--project", tmpDir]);
    runJSON(["scene", "create", "--name", "physics_test", "--root-type", "Node2D", "--project", tmpDir]);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("node add CollisionShape2D auto-creates RectangleShape2D sub_resource", () => {
    const { json, exitCode } = runJSON(["node", "add", "--project", tmpDir], JSON.stringify({
      scene: "physics_test",
      name: "Collision",
      type: "CollisionShape2D",
    }));
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);

    // Read scene to verify sub_resource was created
    const { json: readJson } = runJSON(["scene", "read", "physics_test", "--project", tmpDir]);
    expect(readJson.ok).toBe(true);
    const scene = readJson.data.scene;
    expect(scene.subResources.length).toBeGreaterThan(0);
    expect(scene.subResources[0].type).toBe("RectangleShape2D");
  });
});

// ─── Describe command coverage ─────────────────────────────────

describe("describe coverage", () => {
  it("describe unknown command returns UNKNOWN_COMMAND", () => {
    const { json, exitCode } = runJSON(["describe", "nonexistent.command"]);
    expect(exitCode).toBe(1);
    expect(json.ok).toBe(false);
    expect(json.error.code).toBe("UNKNOWN_COMMAND");
  });

  it("describe with no args lists available commands", () => {
    const { json, exitCode } = runJSON(["describe"]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.available_commands).toBeInstanceOf(Array);
    expect(json.data.engine_targets).toBeInstanceOf(Array);
  });

  it("describe node.add returns schema with input/output", () => {
    const { json, exitCode } = runJSON(["describe", "node.add"]);
    expect(exitCode).toBe(0);
    expect(json.ok).toBe(true);
    expect(json.data.command).toBe("node add");
    expect(json.data.input).toBeDefined();
    expect(json.data.output).toBeDefined();
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
