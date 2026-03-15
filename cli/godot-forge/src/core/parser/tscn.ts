/**
 * Parser and generator for Godot .tscn text format.
 */

import type {
  TscnDocument,
  ExtResource,
  SubResource,
  TscnNode,
  Connection,
} from "./types.js";

// ── Parsing ──────────────────────────────────────────────

interface SectionHeader {
  kind: string;
  attrs: Record<string, string>;
}

function parseSectionHeader(line: string): SectionHeader | null {
  const m = line.match(/^\[(\w+)\s*(.*)\]$/);
  if (!m) return null;
  const kind = m[1];
  const raw = m[2].trim();
  const attrs: Record<string, string> = {};

  // Match key=value or key="value" pairs
  const re = /(\w+)=(\"[^\"]*\"|[^\s]+)/g;
  let match: RegExpExecArray | null;
  while ((match = re.exec(raw)) !== null) {
    let val = match[2];
    if (val.startsWith('"') && val.endsWith('"')) {
      val = val.slice(1, -1);
    }
    attrs[match[1]] = val;
  }

  return { kind, attrs };
}

function parsePropertyValue(raw: string): unknown {
  const trimmed = raw.trim();

  // Quoted string
  if (trimmed.startsWith('"') && trimmed.endsWith('"')) {
    return trimmed.slice(1, -1);
  }

  // Boolean
  if (trimmed === "true") return true;
  if (trimmed === "false") return false;

  // null
  if (trimmed === "null") return null;

  // Number (int or float)
  if (/^-?\d+(\.\d+)?$/.test(trimmed)) {
    return trimmed.includes(".") ? parseFloat(trimmed) : parseInt(trimmed, 10);
  }

  // Return complex values (Vector2(...), ExtResource("..."), etc.) as raw strings
  return trimmed;
}

export function parseTscn(content: string): TscnDocument {
  const lines = content.split("\n");

  const doc: TscnDocument = {
    type: "gd_scene",
    loadSteps: 0,
    format: 3,
    extResources: [],
    subResources: [],
    nodes: [],
    connections: [],
  };

  let currentSection: SectionHeader | null = null;
  let currentProperties: Record<string, unknown> = {};

  function flushSection(): void {
    if (!currentSection) return;

    switch (currentSection.kind) {
      case "gd_scene":
      case "gd_resource":
        doc.type = currentSection.kind as TscnDocument["type"];
        if (currentSection.attrs.load_steps) {
          doc.loadSteps = parseInt(currentSection.attrs.load_steps, 10);
        }
        if (currentSection.attrs.format) {
          doc.format = parseInt(currentSection.attrs.format, 10);
        }
        if (currentSection.attrs.uid) {
          doc.uid = currentSection.attrs.uid;
        }
        break;

      case "ext_resource": {
        const ext: ExtResource = {
          type: currentSection.attrs.type ?? "",
          id: currentSection.attrs.id ?? "",
          path: currentSection.attrs.path ?? "",
        };
        if (currentSection.attrs.uid) {
          ext.uid = currentSection.attrs.uid;
        }
        doc.extResources.push(ext);
        break;
      }

      case "sub_resource": {
        const sub: SubResource = {
          type: currentSection.attrs.type ?? "",
          id: currentSection.attrs.id ?? "",
          properties: { ...currentProperties },
        };
        doc.subResources.push(sub);
        break;
      }

      case "node": {
        const node: TscnNode = {
          name: currentSection.attrs.name ?? "",
          properties: { ...currentProperties },
        };
        if (currentSection.attrs.type) node.type = currentSection.attrs.type;
        if (currentSection.attrs.parent) node.parent = currentSection.attrs.parent;
        if (currentSection.attrs.instance) node.instance = currentSection.attrs.instance;
        doc.nodes.push(node);
        break;
      }

      case "connection": {
        const conn: Connection = {
          signal: currentSection.attrs.signal ?? "",
          from: currentSection.attrs.from ?? "",
          to: currentSection.attrs.to ?? "",
          method: currentSection.attrs.method ?? "",
        };
        doc.connections.push(conn);
        break;
      }
    }

    currentProperties = {};
  }

  for (const line of lines) {
    const trimmed = line.trim();

    // Skip empty lines and comments
    if (trimmed === "" || trimmed.startsWith(";")) continue;

    // Section header
    if (trimmed.startsWith("[")) {
      flushSection();
      currentSection = parseSectionHeader(trimmed);
      continue;
    }

    // Property assignment: key = value
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx > 0) {
      const key = trimmed.slice(0, eqIdx).trimEnd();
      const val = trimmed.slice(eqIdx + 1).trimStart();
      // Only store if key looks like a property name (no spaces except in value)
      if (/^[\w/]+$/.test(key)) {
        currentProperties[key] = parsePropertyValue(val);
      }
    }
  }

  // Flush last section
  flushSection();

  return doc;
}

// ── Generation ───────────────────────────────────────────

function quoteAttr(value: string): string {
  // Numbers and simple identifiers don't need quotes
  if (/^-?\d+(\.\d+)?$/.test(value)) return value;
  return `"${value}"`;
}

function formatValue(value: unknown): string {
  if (typeof value === "string") {
    // Check if it's a Godot function-like value (Vector2, ExtResource, etc.)
    if (/^[A-Z]\w*\(/.test(value) || /^uid:\/\//.test(value)) {
      return value;
    }
    return `"${value}"`;
  }
  if (typeof value === "boolean") return value ? "true" : "false";
  if (value === null) return "null";
  if (typeof value === "number") {
    return Number.isInteger(value) ? String(value) : value.toFixed(value % 1 === 0 ? 1 : (String(value).split(".")[1]?.length ?? 1));
  }
  return String(value);
}

export function generateTscn(doc: TscnDocument): string {
  const parts: string[] = [];

  // Header
  const headerAttrs: string[] = [];
  if (doc.loadSteps > 0) headerAttrs.push(`load_steps=${doc.loadSteps}`);
  headerAttrs.push(`format=${doc.format}`);
  if (doc.uid) headerAttrs.push(`uid="${doc.uid}"`);
  parts.push(`[${doc.type} ${headerAttrs.join(" ")}]`);
  parts.push("");

  // External resources
  for (const ext of doc.extResources) {
    const attrs = [`type="${ext.type}"`, `path="${ext.path}"`, `id="${ext.id}"`];
    if (ext.uid) attrs.splice(2, 0, `uid="${ext.uid}"`);
    parts.push(`[ext_resource ${attrs.join(" ")}]`);
  }
  if (doc.extResources.length > 0) parts.push("");

  // Sub resources
  for (const sub of doc.subResources) {
    parts.push(`[sub_resource type="${sub.type}" id="${sub.id}"]`);
    for (const [key, val] of Object.entries(sub.properties)) {
      parts.push(`${key} = ${formatValue(val)}`);
    }
    parts.push("");
  }

  // Nodes
  for (const node of doc.nodes) {
    const attrs = [`name="${node.name}"`];
    if (node.type) attrs.push(`type="${node.type}"`);
    if (node.parent !== undefined) attrs.push(`parent="${node.parent}"`);
    if (node.instance) attrs.push(`instance=${node.instance}`);
    parts.push(`[node ${attrs.join(" ")}]`);
    for (const [key, val] of Object.entries(node.properties)) {
      parts.push(`${key} = ${formatValue(val)}`);
    }
    parts.push("");
  }

  // Connections
  for (const conn of doc.connections) {
    parts.push(
      `[connection signal="${conn.signal}" from="${conn.from}" to="${conn.to}" method="${conn.method}"]`
    );
  }
  if (doc.connections.length > 0) parts.push("");

  return parts.join("\n");
}

// ── Resource management ─────────────────────────────────

function nextId(existingIds: string[]): string {
  let max = 0;
  for (const id of existingIds) {
    const num = parseInt(id, 10);
    if (!isNaN(num) && num > max) max = num;
  }
  return String(max + 1);
}

/**
 * Add an external resource reference. Idempotent: returns existing ID if path already present.
 */
export function addExtResource(
  doc: TscnDocument,
  type: string,
  path: string,
  uid?: string
): { doc: TscnDocument; id: string } {
  const existing = doc.extResources.find((r) => r.path === path);
  if (existing) {
    return { doc, id: existing.id };
  }

  const id = nextId(doc.extResources.map((r) => r.id));
  const ext: ExtResource = { type, id, path };
  if (uid) ext.uid = uid;

  return {
    doc: { ...doc, extResources: [...doc.extResources, ext] },
    id,
  };
}

/**
 * Remove an external resource by ID.
 */
export function removeExtResource(
  doc: TscnDocument,
  id: string
): TscnDocument {
  return {
    ...doc,
    extResources: doc.extResources.filter((r) => r.id !== id),
  };
}

/**
 * Find an ext_resource by path, returns its ID or null.
 */
export function findExtResource(
  doc: TscnDocument,
  path: string
): string | null {
  const found = doc.extResources.find((r) => r.path === path);
  return found ? found.id : null;
}

/**
 * Add a sub_resource, returns the new doc and assigned ID.
 */
export function addSubResource(
  doc: TscnDocument,
  type: string,
  properties: Record<string, unknown>
): { doc: TscnDocument; id: string } {
  const id = nextId(doc.subResources.map((r) => r.id));
  const sub: SubResource = { type, id, properties };

  return {
    doc: { ...doc, subResources: [...doc.subResources, sub] },
    id,
  };
}

/**
 * Remove a sub_resource by ID.
 */
export function removeSubResource(
  doc: TscnDocument,
  id: string
): TscnDocument {
  return {
    ...doc,
    subResources: doc.subResources.filter((r) => r.id !== id),
  };
}

/**
 * Recalculate loadSteps based on ext + sub resource count.
 * loadSteps = extResources.length + subResources.length + 1 (for the root).
 * Only set when there are resources; otherwise 0.
 */
export function recalcLoadSteps(doc: TscnDocument): TscnDocument {
  const total = doc.extResources.length + doc.subResources.length;
  return {
    ...doc,
    loadSteps: total > 0 ? total + 1 : 0,
  };
}

/**
 * Add a connection to the document.
 */
export function addConnection(
  doc: TscnDocument,
  connection: Connection
): TscnDocument {
  return {
    ...doc,
    connections: [...doc.connections, connection],
  };
}

/**
 * Remove a connection matching signal, from, and to.
 */
export function removeConnection(
  doc: TscnDocument,
  signal: string,
  from: string,
  to: string
): TscnDocument {
  return {
    ...doc,
    connections: doc.connections.filter(
      (c) => !(c.signal === signal && c.from === from && c.to === to)
    ),
  };
}

// ── Scene helpers ────────────────────────────────────────

export function createEmptyScene(
  rootType: string,
  rootName: string
): TscnDocument {
  return {
    type: "gd_scene",
    loadSteps: 0,
    format: 3,
    extResources: [],
    subResources: [],
    nodes: [
      {
        name: rootName,
        type: rootType,
        properties: {},
      },
    ],
    connections: [],
  };
}

export function addNode(doc: TscnDocument, node: TscnNode): TscnDocument {
  return {
    ...doc,
    nodes: [...doc.nodes, node],
  };
}

export function removeNode(
  doc: TscnDocument,
  nodePath: string
): TscnDocument {
  // Build the full path for each node: root has no parent, children use parent + name
  function getNodePath(node: TscnNode): string {
    if (node.parent === undefined) return node.name;
    if (node.parent === ".") return node.name;
    return `${node.parent}/${node.name}`;
  }

  // Find the target path and also remove children whose parent starts with the removed path
  const targetNode = doc.nodes.find((n) => getNodePath(n) === nodePath);
  if (!targetNode) {
    return doc; // Node not found, return unchanged
  }

  const targetFullPath = getNodePath(targetNode);

  const filteredNodes = doc.nodes.filter((n) => {
    const fullPath = getNodePath(n);
    if (fullPath === targetFullPath) return false;
    // Remove children: their parent starts with the removed node path
    if (n.parent === targetFullPath) return false;
    if (n.parent?.startsWith(targetFullPath + "/")) return false;
    return true;
  });

  return {
    ...doc,
    nodes: filteredNodes,
  };
}
