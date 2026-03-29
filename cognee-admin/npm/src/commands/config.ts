import { Command } from "commander";
import { configError } from "../errors.js";
import { createContext, printError, printSuccess } from "./common.js";

export function createConfigCommand(): Command {
  const command = new Command("config").description("Cognee settings management");

  command.addCommand(
    new Command("get")
      .description("Get current settings")
      .action(async function () {
        try {
          const ctx = createContext(this);
          printSuccess("config.get", await ctx.client.getSettings(), ctx);
        } catch (error) {
          printError("config.get", error);
        }
      })
  );

  command.addCommand(
    new Command("set")
      .description("Update settings")
      .argument("<settings>", "Settings as JSON")
      .action(async function (settings: string) {
        try {
          const ctx = createContext(this);
          const parsed = JSON.parse(settings) as unknown;
          printSuccess("config.set", await ctx.client.saveSettings(parsed), ctx);
        } catch (error) {
          if (error instanceof SyntaxError) {
            printError("config.set", configError(`Invalid JSON: ${error.message}`));
          }
          printError("config.set", error);
        }
      })
  );

  return command;
}
