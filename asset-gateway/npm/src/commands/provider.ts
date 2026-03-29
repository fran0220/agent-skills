import { Command } from "commander";
import { createContext, printError, printSuccess } from "./common.js";

export function createProviderCommand(): Command {
  const command = new Command("provider").description("Provider management");

  command.addCommand(
    new Command("list")
      .description("List available providers")
      .action(async function () {
        try {
          const ctx = createContext(this);
          const data = await ctx.client.get("/api/providers");
          printSuccess("provider.list", data, ctx);
        } catch (error) {
          printError("provider.list", error);
        }
      })
  );

  command.addCommand(
    new Command("health")
      .description("Check provider health")
      .argument("[name]", "Specific provider name")
      .action(async function (name?: string) {
        try {
          const ctx = createContext(this);
          const path = name
            ? `/api/providers/${encodeURIComponent(name)}/health`
            : "/api/providers/health";
          const data = await ctx.client.get(path);
          printSuccess("provider.health", data, ctx);
        } catch (error) {
          printError("provider.health", error);
        }
      })
  );

  return command;
}
