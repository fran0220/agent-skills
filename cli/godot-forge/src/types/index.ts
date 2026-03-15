/**
 * Common types for godot-forge CLI.
 */

export interface ProjectConfig {
  name: string;
  path: string;
  godotVersion?: string;
  template?: string;
}

export interface SceneSpec {
  name: string;
  root_type: string;
  path?: string;
  children?: NodeSpec[];
}

export interface NodeSpec {
  name: string;
  type: string;
  parent?: string;
  properties?: Record<string, unknown>;
  children?: NodeSpec[];
}

export interface ExportVar {
  name: string;
  type: string;
  default?: unknown;
}

export interface ScriptSpec {
  name: string;
  extends: string;
  path?: string;
  signals?: string[];
  methods?: string[];
  exports?: ExportVar[];
}

export interface ScriptEditSpec {
  path: string;
  add_signals?: string[];
  add_methods?: string[];
  add_exports?: ExportVar[];
}

export interface ScriptInfo {
  path: string;
  extends: string | null;
  class_name: string | null;
}

export interface ProjectConfigInput {
  section: string;
  key: string;
  value?: string;
}

export interface AutoloadEntry {
  name: string;
  path: string;
  enabled?: boolean; // "*" prefix means enabled
}

export interface InputAction {
  name: string;
  deadzone?: number;
  events?: InputEvent[];
}

export interface InputEvent {
  type: "key" | "mouse_button" | "joypad_button" | "joypad_motion";
  keycode?: string; // for key events (physical_keycode)
  button_index?: number; // for mouse/joypad button
  axis?: number; // for joypad motion
  axis_value?: number;
}

export interface ConnectionSpec {
  scene: string;
  signal: string;
  from: string;
  to: string;
  method: string;
}

export interface NodeMoveInput {
  scene: string;
  path: string;
  new_parent: string;
}

/** Global CLI options inherited by all commands */
export interface GlobalOptions {
  project?: string;
  human?: boolean;
  fields?: string;
  force?: boolean;
  dryRun?: boolean;
  input?: string;
}
