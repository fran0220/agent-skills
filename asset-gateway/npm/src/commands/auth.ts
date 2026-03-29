import { existsSync, unlinkSync } from "node:fs";
import { Command } from "commander";
import { configPath, loadAuthConfig, resolveGatewayUrl, saveAuthConfig } from "../config.js";
import { output, success } from "../output.js";
import type { GlobalOptions } from "./common.js";
import { printError } from "./common.js";

export function createAuthCommand(): Command {
  const command = new Command("auth").description("Credential management");

  command.addCommand(
    new Command("set")
      .description("Save token and gateway URL locally")
      .argument("<token>", "API token (agk_ prefix for admin, or any API key)")
      .action(function (token: string) {
        const globals = this.optsWithGlobals<GlobalOptions>();
        try {
          const current = loadAuthConfig();
          const gatewayUrl = resolveGatewayUrl({ gatewayUrl: globals.gatewayUrl });

          saveAuthConfig({ ...current, token, gateway_url: gatewayUrl });

          output(
            success("auth.set", { saved: true, gateway_url: gatewayUrl }),
            Boolean(globals.human)
          );
        } catch (error) {
          printError("auth.set", error);
        }
      })
  );

  command.addCommand(
    new Command("status")
      .description("Show current authentication status")
      .action(function () {
        const globals = this.optsWithGlobals<GlobalOptions>();
        const config = loadAuthConfig();
        const path = configPath();

        output(
          success("auth.status", {
            config_path: path,
            has_token: Boolean(config.token),
            token_preview: config.token ? mask(config.token) : null,
            gateway_url: config.gateway_url ?? null,
          }),
          Boolean(globals.human)
        );
      })
  );

  command.addCommand(
    new Command("clear")
      .description("Remove saved credentials")
      .action(function () {
        const globals = this.optsWithGlobals<GlobalOptions>();
        const path = configPath();
        if (existsSync(path)) {
          unlinkSync(path);
        }
        output(success("auth.clear", { cleared: true }), Boolean(globals.human));
      })
  );

  return command;
}

function mask(token: string): string {
  if (token.length <= 12) return "***";
  return `${token.slice(0, 8)}...${token.slice(-4)}`;
}
