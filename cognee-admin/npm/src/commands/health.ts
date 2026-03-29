import { Command } from "commander";
import { configError } from "../errors.js";
import { createContext, printError, printSuccess } from "./common.js";

export function createHealthCommand(): Command {
  return new Command("health")
    .description("Check Cognee API health")
    .option("--detailed", "Include detailed system info")
    .option("--watch", "Continuously poll until Ctrl+C")
    .option("--interval <seconds>", "Polling interval in seconds for watch mode", "30")
    .action(async function (options: { detailed?: boolean; watch?: boolean; interval?: string }) {
      const interval = Number.parseInt(options.interval ?? "30", 10);

      if (!Number.isFinite(interval) || interval < 1) {
        printError("health", configError("--interval must be at least 1 second"));
        return;
      }

      try {
        const ctx = createContext(this, false);
        if (!options.watch) {
          const data = options.detailed
            ? await ctx.client.healthDetailed()
            : await ctx.client.health();
          printSuccess("health", data, ctx);
          return;
        }

        let stopped = false;
        const onSigint = () => {
          stopped = true;
        };
        process.once("SIGINT", onSigint);

        while (!stopped) {
          const data = options.detailed
            ? await ctx.client.healthDetailed()
            : await ctx.client.health();
          printSuccess("health", data, ctx);
          await sleep(interval * 1000);
        }

        process.removeListener("SIGINT", onSigint);
        printSuccess(
          "health",
          { watch_stopped: true, interval_secs: interval },
          ctx
        );
      } catch (error) {
        printError("health", error);
      }
    });
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
