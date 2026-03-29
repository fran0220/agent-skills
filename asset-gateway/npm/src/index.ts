#!/usr/bin/env node

/**
 * asset-gateway — universal asset generation gateway CLI.
 *
 * JSON output is the DEFAULT. Use --human for human-readable format.
 */

import { Command } from "commander";
import { createAuthCommand } from "./commands/auth.js";
import { createDescribeCommand } from "./commands/describe.js";
import { createGenerateCommand } from "./commands/generate.js";
import { createJobCommand } from "./commands/job.js";
import { createProviderCommand } from "./commands/provider.js";
import { createUploadCommand } from "./commands/upload.js";
import { CLI_VERSION, DEFAULT_GATEWAY_URL } from "./meta.js";

const program = new Command()
  .name("asset-gateway")
  .description("Universal asset generation gateway CLI")
  .version(CLI_VERSION)
  .option(
    "--gateway-url <url>",
    `Gateway URL (default: $ASSET_GATEWAY_URL, auth config, or ${DEFAULT_GATEWAY_URL})`
  )
  .option("--token <token>", "API token for authentication")
  .option("--human", "Human-readable output instead of JSON")
  .option("--fields <fields>", "Comma-separated list of output fields");

program.addCommand(createAuthCommand());
program.addCommand(createGenerateCommand());
program.addCommand(createProviderCommand());
program.addCommand(createUploadCommand());
program.addCommand(createJobCommand());
program.addCommand(createDescribeCommand());

await program.parseAsync(process.argv);
