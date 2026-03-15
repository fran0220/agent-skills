/**
 * AST types for Godot .tscn / .tres file format.
 */

export interface TscnDocument {
  type: "gd_scene" | "gd_resource";
  loadSteps: number;
  format: number;
  uid?: string;
  extResources: ExtResource[];
  subResources: SubResource[];
  nodes: TscnNode[];
  connections: Connection[];
}

export interface ExtResource {
  type: string;
  id: string;
  uid?: string;
  path: string;
}

export interface SubResource {
  type: string;
  id: string;
  properties: Record<string, unknown>;
}

export interface TscnNode {
  name: string;
  type?: string;
  parent?: string;
  instance?: string;
  groups?: string[];
  unique_name_in_owner?: boolean;
  index?: number;
  properties: Record<string, unknown>;
}

export interface Connection {
  signal: string;
  from: string;
  to: string;
  method: string;
  flags?: number;
  binds?: unknown[];
}
