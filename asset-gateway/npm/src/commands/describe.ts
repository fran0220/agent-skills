import { Command } from "commander";
import { SCHEMAS } from "../describe-schemas.js";
import { CLI_DESCRIPTION, CLI_NAME, CLI_VERSION } from "../meta.js";
import { output, success } from "../output.js";
import type { GlobalOptions } from "./common.js";

export { SCHEMAS };

export function createDescribeCommand(): Command {
  return new Command("describe")
    .description("Self-describe available commands (JSON Schema)")
    .argument("[command]", "Specific command group to describe (e.g. generate, process)")
    .action(function (commandArg?: string) {
      const globals = this.optsWithGlobals<GlobalOptions>();
      if (!commandArg) {
        output(
          success("describe", {
            name: CLI_NAME,
            version: CLI_VERSION,
            description: CLI_DESCRIPTION,
            commands: SCHEMAS,
          }),
          globals.human
        );
        return;
      }

      const schema = SCHEMAS[commandArg];
      if (!schema) {
        output(
          success("describe", {
            error: `Unknown command: ${commandArg}`,
            available: Object.keys(SCHEMAS),
          }),
          globals.human
        );
        return;
      }

      output(success("describe", { command: commandArg, schema }), globals.human);
    });
}
