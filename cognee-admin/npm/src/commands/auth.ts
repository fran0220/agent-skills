import { existsSync, unlinkSync } from "node:fs";
import { Command } from "commander";
import { configPath, loadAuthConfig, resolveCogneeUrl, saveAuthConfig } from "../config.js";
import { configError } from "../errors.js";
import { output, success } from "../output.js";
import type { GlobalOptions } from "./common.js";
import { printError } from "./common.js";

export function createAuthCommand(): Command {
  const command = new Command("auth").description("Credential management");

  command.addCommand(
    new Command("set")
      .description("Save a ca_xxx token locally")
      .argument("<token>", "Admin token (ca_xxx)")
      .action(function (token: string) {
        const globals = this.optsWithGlobals<GlobalOptions>();
        try {
          if (!token.startsWith("ca_")) {
            throw configError("Token must start with 'ca_'");
          }

          const current = loadAuthConfig();
          const cogneeUrl = resolveCogneeUrl({ cogneeUrl: globals.cogneeUrl });

          saveAuthConfig({ ...current, admin_token: token, cognee_url: cogneeUrl });

          output(
            success("auth.set", { saved: true, cognee_url: cogneeUrl }),
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
            has_token: Boolean(config.admin_token),
            token_preview: config.admin_token ? mask(config.admin_token) : null,
            cognee_url: config.cognee_url ?? null,
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
