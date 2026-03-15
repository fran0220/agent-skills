import { describe, it, expect } from "vitest";
import {
  parseTscn,
  generateTscn,
  createEmptyScene,
  addNode,
  removeNode,
} from "../../src/core/parser/tscn.js";
import type { TscnNode } from "../../src/core/parser/types.js";

// ── Fixtures ─────────────────────────────────────────────

const MINIMAL_SCENE = `[gd_scene format=3]

[node name="Root" type="Node2D"]
`;

const COMPLEX_SCENE = `[gd_scene load_steps=3 format=3 uid="uid://abc123"]

[ext_resource type="Script" path="res://scripts/player.gd" id="1_abc"]

[sub_resource type="RectangleShape2D" id="RectangleShape2D_xyz"]
size = Vector2(16, 24)

[node name="Player" type="CharacterBody2D"]
script = ExtResource("1_abc")

[node name="Sprite" type="Sprite2D" parent="."]

[node name="Collision" type="CollisionShape2D" parent="."]
shape = SubResource("RectangleShape2D_xyz")

[connection signal="ready" from="." to="." method="_on_ready"]
`;

// ── parseTscn ────────────────────────────────────────────

describe("parseTscn", () => {
  it("parses a minimal scene", () => {
    const doc = parseTscn(MINIMAL_SCENE);

    expect(doc.type).toBe("gd_scene");
    expect(doc.format).toBe(3);
    expect(doc.loadSteps).toBe(0);
    expect(doc.uid).toBeUndefined();
    expect(doc.extResources).toHaveLength(0);
    expect(doc.subResources).toHaveLength(0);
    expect(doc.connections).toHaveLength(0);
    expect(doc.nodes).toHaveLength(1);
    expect(doc.nodes[0]).toEqual({
      name: "Root",
      type: "Node2D",
      properties: {},
    });
  });

  it("parses ext_resource, sub_resource, nodes, and connections", () => {
    const doc = parseTscn(COMPLEX_SCENE);

    expect(doc.type).toBe("gd_scene");
    expect(doc.format).toBe(3);
    expect(doc.loadSteps).toBe(3);
    expect(doc.uid).toBe("uid://abc123");

    // ext_resource
    expect(doc.extResources).toHaveLength(1);
    expect(doc.extResources[0]).toEqual({
      type: "Script",
      id: "1_abc",
      path: "res://scripts/player.gd",
    });

    // sub_resource
    expect(doc.subResources).toHaveLength(1);
    expect(doc.subResources[0]).toEqual({
      type: "RectangleShape2D",
      id: "RectangleShape2D_xyz",
      properties: { size: "Vector2(16, 24)" },
    });

    // nodes
    expect(doc.nodes).toHaveLength(3);
    expect(doc.nodes[0]).toMatchObject({
      name: "Player",
      type: "CharacterBody2D",
      properties: { script: 'ExtResource("1_abc")' },
    });
    expect(doc.nodes[1]).toMatchObject({
      name: "Sprite",
      type: "Sprite2D",
      parent: ".",
    });
    expect(doc.nodes[2]).toMatchObject({
      name: "Collision",
      type: "CollisionShape2D",
      parent: ".",
      properties: { shape: 'SubResource("RectangleShape2D_xyz")' },
    });

    // connections
    expect(doc.connections).toHaveLength(1);
    expect(doc.connections[0]).toEqual({
      signal: "ready",
      from: ".",
      to: ".",
      method: "_on_ready",
    });
  });
});

// ── Property values ──────────────────────────────────────

describe("property value parsing", () => {
  function parseProperty(value: string): unknown {
    const doc = parseTscn(`[gd_scene format=3]\n\n[node name="N" type="Node"]\nprop = ${value}\n`);
    return doc.nodes[0].properties.prop;
  }

  it("parses quoted strings", () => {
    expect(parseProperty('"hello world"')).toBe("hello world");
  });

  it("parses integers", () => {
    expect(parseProperty("42")).toBe(42);
    expect(parseProperty("-7")).toBe(-7);
  });

  it("parses floats", () => {
    expect(parseProperty("3.14")).toBe(3.14);
    expect(parseProperty("-0.5")).toBe(-0.5);
  });

  it("parses booleans", () => {
    expect(parseProperty("true")).toBe(true);
    expect(parseProperty("false")).toBe(false);
  });

  it("parses null", () => {
    expect(parseProperty("null")).toBeNull();
  });

  it("preserves Vector2 as raw string", () => {
    expect(parseProperty("Vector2(10, 20)")).toBe("Vector2(10, 20)");
  });

  it("preserves ExtResource as raw string", () => {
    expect(parseProperty('ExtResource("1_abc")')).toBe('ExtResource("1_abc")');
  });

  it("preserves SubResource as raw string", () => {
    expect(parseProperty('SubResource("shape_id")')).toBe('SubResource("shape_id")');
  });

  it("preserves Color as raw string", () => {
    expect(parseProperty("Color(1, 0, 0, 1)")).toBe("Color(1, 0, 0, 1)");
  });
});

// ── generateTscn ─────────────────────────────────────────

describe("generateTscn", () => {
  it("generates valid .tscn text from AST", () => {
    const doc = parseTscn(COMPLEX_SCENE);
    const output = generateTscn(doc);

    expect(output).toContain("[gd_scene");
    expect(output).toContain('format=3');
    expect(output).toContain('uid="uid://abc123"');
    expect(output).toContain('[ext_resource type="Script"');
    expect(output).toContain('path="res://scripts/player.gd"');
    expect(output).toContain('[sub_resource type="RectangleShape2D"');
    expect(output).toContain("size = Vector2(16, 24)");
    expect(output).toContain('[node name="Player" type="CharacterBody2D"]');
    expect(output).toContain('[node name="Sprite" type="Sprite2D" parent="."]');
    expect(output).toContain('script = ExtResource("1_abc")');
    expect(output).toContain('[connection signal="ready"');
  });

  it("generates a minimal scene", () => {
    const doc = createEmptyScene("Node2D", "Root");
    const output = generateTscn(doc);

    expect(output).toContain("[gd_scene format=3]");
    expect(output).toContain('[node name="Root" type="Node2D"]');
    expect(output).not.toContain("[ext_resource");
    expect(output).not.toContain("[sub_resource");
    expect(output).not.toContain("[connection");
  });
});

// ── Round-trip ───────────────────────────────────────────

describe("round-trip", () => {
  it("parseTscn → generateTscn → parseTscn produces the same AST", () => {
    const ast1 = parseTscn(COMPLEX_SCENE);
    const generated = generateTscn(ast1);
    const ast2 = parseTscn(generated);

    expect(ast2.type).toBe(ast1.type);
    expect(ast2.format).toBe(ast1.format);
    expect(ast2.loadSteps).toBe(ast1.loadSteps);
    expect(ast2.uid).toBe(ast1.uid);
    expect(ast2.extResources).toEqual(ast1.extResources);
    expect(ast2.subResources).toEqual(ast1.subResources);
    expect(ast2.nodes).toEqual(ast1.nodes);
    expect(ast2.connections).toEqual(ast1.connections);
  });

  it("round-trips a minimal scene", () => {
    const ast1 = parseTscn(MINIMAL_SCENE);
    const generated = generateTscn(ast1);
    const ast2 = parseTscn(generated);

    expect(ast2).toEqual(ast1);
  });
});

// ── createEmptyScene ─────────────────────────────────────

describe("createEmptyScene", () => {
  it("creates correct minimal AST", () => {
    const doc = createEmptyScene("Control", "UI");

    expect(doc.type).toBe("gd_scene");
    expect(doc.format).toBe(3);
    expect(doc.loadSteps).toBe(0);
    expect(doc.uid).toBeUndefined();
    expect(doc.extResources).toEqual([]);
    expect(doc.subResources).toEqual([]);
    expect(doc.connections).toEqual([]);
    expect(doc.nodes).toHaveLength(1);
    expect(doc.nodes[0]).toEqual({
      name: "UI",
      type: "Control",
      properties: {},
    });
  });
});

// ── addNode ──────────────────────────────────────────────

describe("addNode", () => {
  it("adds a node to an existing document", () => {
    const doc = createEmptyScene("Node2D", "Root");
    const child: TscnNode = {
      name: "Sprite",
      type: "Sprite2D",
      parent: ".",
      properties: {},
    };
    const updated = addNode(doc, child);

    expect(updated.nodes).toHaveLength(2);
    expect(updated.nodes[1]).toEqual(child);
    // Original is not mutated
    expect(doc.nodes).toHaveLength(1);
  });

  it("adds a node with properties", () => {
    const doc = createEmptyScene("Node2D", "Root");
    const child: TscnNode = {
      name: "Label",
      type: "Label",
      parent: ".",
      properties: { text: "Hello", visible: true },
    };
    const updated = addNode(doc, child);

    expect(updated.nodes[1].properties).toEqual({
      text: "Hello",
      visible: true,
    });
  });
});

// ── removeNode ───────────────────────────────────────────

describe("removeNode", () => {
  it("removes a node by path", () => {
    let doc = createEmptyScene("Node2D", "Root");
    doc = addNode(doc, { name: "Sprite", type: "Sprite2D", parent: ".", properties: {} });
    doc = addNode(doc, { name: "Label", type: "Label", parent: ".", properties: {} });

    const updated = removeNode(doc, "Sprite");

    expect(updated.nodes).toHaveLength(2);
    expect(updated.nodes.map((n) => n.name)).toEqual(["Root", "Label"]);
    // Original is not mutated
    expect(doc.nodes).toHaveLength(3);
  });

  it("removes a node and its children", () => {
    let doc = createEmptyScene("Node2D", "Root");
    doc = addNode(doc, { name: "Container", type: "VBoxContainer", parent: ".", properties: {} });
    doc = addNode(doc, { name: "Child1", type: "Label", parent: "Container", properties: {} });
    doc = addNode(doc, { name: "Child2", type: "Button", parent: "Container", properties: {} });
    doc = addNode(doc, { name: "Other", type: "Sprite2D", parent: ".", properties: {} });

    const updated = removeNode(doc, "Container");

    expect(updated.nodes).toHaveLength(2);
    expect(updated.nodes.map((n) => n.name)).toEqual(["Root", "Other"]);
  });

  it("returns unchanged document when node not found", () => {
    const doc = createEmptyScene("Node2D", "Root");
    const updated = removeNode(doc, "NonExistent");

    expect(updated.nodes).toEqual(doc.nodes);
  });
});
