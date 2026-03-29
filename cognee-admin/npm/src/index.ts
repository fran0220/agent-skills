#!/usr/bin/env node

/**
 * cognee-admin — lightweight Cognee API CLI client.
 *
 * JSON output is the DEFAULT. Use --human for human-readable format.
 */

import { Command } from "commander";
import { createAuthCommand } from "./commands/auth.js";
import { createCognifyCommand } from "./commands/cognify.js";
import { createConfigCommand } from "./commands/config.js";
import { createDataCommand } from "./commands/data.js";
import { createDatasetCommand } from "./commands/dataset.js";
import { createDescribeCommand } from "./commands/describe.js";
import { createHealthCommand } from "./commands/health.js";
import { createOntologyCommand } from "./commands/ontology.js";
import { createSearchCommand } from "./commands/search.js";
import { CLI_VERSION, DEFAULT_COGNEE_URL } from "./meta.js";

const program = new Command()
  .name("cognee-admin")
  .description("Cognee knowledge engine management CLI")
  .version(CLI_VERSION)
  .option(
    "--cognee-url <url>",
    `Cognee API URL (default: $COGNEE_URL, auth config, or ${DEFAULT_COGNEE_URL})`
  )
  .option("--admin-token <token>", "Admin token (ca_xxx) for API authentication")
  .option("--human", "Human-readable output instead of JSON")
  .option("--fields <fields>", "Comma-separated list of output fields");

program.addCommand(createHealthCommand());
program.addCommand(createAuthCommand());
program.addCommand(createDatasetCommand());
program.addCommand(createDataCommand());
program.addCommand(createCognifyCommand());
program.addCommand(createSearchCommand());
program.addCommand(createConfigCommand());
program.addCommand(createOntologyCommand());
program.addCommand(createDescribeCommand());

await program.parseAsync(process.argv);
