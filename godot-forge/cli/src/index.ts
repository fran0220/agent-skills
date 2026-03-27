#!/usr/bin/env node

/**
 * godot-forge — Agent-Primary CLI for Godot 4.x game development.
 *
 * JSON output is the DEFAULT. Use --human for human-readable format.
 */

import { Command } from "commander";
import { createProjectCommand } from "./commands/project.js";
import { createSceneCommand } from "./commands/scene.js";
import { createNodeCommand } from "./commands/node.js";
import { createScriptCommand } from "./commands/script.js";
import { createEngineCommand } from "./commands/engine.js";
import { createExportCommand } from "./commands/export.js";
import { createResourceCommand } from "./commands/resource.js";
import { createDescribeCommand } from "./commands/describe.js";

const program = new Command()
  .name("godot-forge")
  .description("Agent-Primary CLI for AI-driven Godot 4.x game development")
  .version("0.1.0")
  // Global options
  .option("--project <path>", "Project directory (default: current directory)")
  .option("--human", "Human-readable output instead of JSON")
  .option("--fields <fields>", "Comma-separated list of output fields")
  .option("--force", "Force overwrite existing files")
  .option("--dry-run", "Show planned changes without executing")
  .option("--input <file>", "Read JSON input from file");

// Register command groups
program.addCommand(createProjectCommand());
program.addCommand(createSceneCommand());
program.addCommand(createNodeCommand());
program.addCommand(createScriptCommand());
program.addCommand(createEngineCommand());
program.addCommand(createExportCommand());
program.addCommand(createResourceCommand());
program.addCommand(createDescribeCommand());

program.parse();
