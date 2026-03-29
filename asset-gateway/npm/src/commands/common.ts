import type { Command } from "commander";
import { GatewayClient } from "../client.js";
import { resolveGatewayUrl, resolveToken } from "../config.js";
import { configError, normalizeError } from "../errors.js";
import { error, filterFields, output, success } from "../output.js";

export interface GlobalOptions {
  gatewayUrl?: string;
  token?: string;
  human?: boolean;
  fields?: string;
}

export interface CommandContext {
  client: GatewayClient;
  human: boolean;
  fields?: string;
}

export function getGlobalOptions(command: Command): GlobalOptions {
  return command.optsWithGlobals<GlobalOptions>();
}

export function createContext(command: Command, requireAuth = true): CommandContext {
  const globals = getGlobalOptions(command);
  const url = resolveGatewayUrl({ gatewayUrl: globals.gatewayUrl });
  const token = resolveToken({ token: globals.token });

  if (requireAuth && !token) {
    throw configError(
      "Authentication required. Use --token, set ASSET_GATEWAY_TOKEN, or run asset-gateway auth set <token>.",
      "asset-gateway auth set <token>"
    );
  }

  return {
    client: new GatewayClient(url, token),
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
