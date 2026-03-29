import type { Command } from "commander";
import { CogneeClient } from "../client.js";
import { resolveAdminToken, resolveCogneeUrl } from "../config.js";
import { configError, normalizeError } from "../errors.js";
import { error, filterFields, output, success } from "../output.js";

export interface GlobalOptions {
  cogneeUrl?: string;
  adminToken?: string;
  human?: boolean;
  fields?: string;
}

export interface CommandContext {
  client: CogneeClient;
  human: boolean;
  fields?: string;
}

export function getGlobalOptions(command: Command): GlobalOptions {
  return command.optsWithGlobals<GlobalOptions>();
}

export function createContext(command: Command, requireAuth = true): CommandContext {
  const globals = getGlobalOptions(command);
  const url = resolveCogneeUrl({ cogneeUrl: globals.cogneeUrl });
  const token = resolveAdminToken({ adminToken: globals.adminToken });

  if (requireAuth && !token) {
    throw configError(
      "Authentication required. Use --admin-token, set COGNEE_ADMIN_TOKEN, or run cognee-admin auth set <token>.",
      "cognee-admin auth set <ca_xxx>"
    );
  }

  return {
    client: new CogneeClient(url, token),
    human: Boolean(globals.human),
    fields: globals.fields,
  };
}

export function printSuccess(commandName: string, data: unknown, ctx: Pick<CommandContext, "human" | "fields">): void {
  const filtered = isRecord(data) ? filterFields(data, ctx.fields) : data;
  output(success(commandName, filtered), ctx.human);
}

export function printError(commandName: string, err: unknown, human = false): never {
  const normalized = normalizeError(err);
  output(
    error(commandName, normalized.code, normalized.message, normalized.suggestion),
    human
  );
  process.exit(normalized.exitCode);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
