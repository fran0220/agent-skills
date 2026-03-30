#!/usr/bin/env node

import { existsSync, unlinkSync } from "node:fs";
import { Command } from "commander";
import { GatewayClient } from "./client.js";
import { configPath, loadAuthConfig, resolveGatewayUrl, resolveToken, saveAuthConfig } from "./config.js";
import { configError, inputError, normalizeError } from "./errors.js";
import { CLI_NAME, CLI_VERSION, DEFAULT_GATEWAY_URL, SEARCH_MODES, SEARCH_MODELS, type SearchMode } from "./meta.js";
import { error as errorResult, output, success } from "./output.js";

interface GlobalOptions {
  gatewayUrl?: string;
  token?: string;
  human?: boolean;
}

interface SearchCommandOptions extends GlobalOptions {
  stdin?: boolean;
  mode?: string;
  model?: string;
  split?: string;
  num?: string;
}

interface StdinSearchInput {
  query?: unknown;
  mode?: unknown;
  model?: unknown;
  split?: unknown;
  num?: unknown;
}

const program = new Command()
  .name(CLI_NAME)
  .description("AI Search Gateway CLI client")
  .version(CLI_VERSION)
  .option(
    "--gateway-url <url>",
    `Gateway URL (default: $AI_SEARCH_GATEWAY_URL, auth config, or ${DEFAULT_GATEWAY_URL})`
  )
  .option("--token <token>", "Gateway token for authentication")
  .option("--human", "Human-readable output instead of JSON");

program
  .argument("[query...]", "Search query")
  .option("--mode <mode>", `Search mode (${SEARCH_MODES.join(", ")}; default: fast)`)
  .option("-m, --model <model>", "Model to use")
  .option("--split <n>", "Max sub-queries for query splitting")
  .option("--num <n>", "Max search results to request")
  .action(async function (this: Command, queryParts: string[]) {
    if (queryParts.length === 0) {
      program.help();
      return;
    }

    await runSearchCommand(queryParts, this, "search");
  });

program
  .command("search [query...]")
  .description("Search the web through the gateway")
  .option("--mode <mode>", `Search mode (${SEARCH_MODES.join(", ")}; default: fast)`)
  .option("-m, --model <model>", "Model to use")
  .option("--split <n>", "Max sub-queries for query splitting")
  .option("--num <n>", "Max search results to request")
  .option("--stdin", "Read query from stdin JSON")
  .action(async function (this: Command, queryParts: string[]) {
    await runSearchCommand(queryParts, this, "search");
  });

program
  .command("models")
  .description("List available gateway models")
  .action(async function (this: Command) {
    const globals = this.optsWithGlobals<GlobalOptions>();
    try {
      const client = createClient(globals);
      const models = await client.models();
      output(success("models", { models }), Boolean(globals.human));
    } catch (err) {
      printError("models", err, Boolean(globals.human));
    }
  });

program
  .command("providers")
  .description("List gateway search providers")
  .action(async function (this: Command) {
    const globals = this.optsWithGlobals<GlobalOptions>();
    try {
      const client = createClient(globals);
      const providers = await client.providers();
      output(success("providers", { providers }), Boolean(globals.human));
    } catch (err) {
      printError("providers", err, Boolean(globals.human));
    }
  })
  .addCommand(
    new Command("health")
      .description("Check provider health")
      .action(async function (this: Command) {
        const globals = this.optsWithGlobals<GlobalOptions>();
        try {
          const client = createClient(globals);
          const providers = await client.providersHealth();
          output(success("providers.health", { providers }), Boolean(globals.human));
        } catch (err) {
          printError("providers.health", err, Boolean(globals.human));
        }
      })
  );

program
  .command("health")
  .description("Check gateway health")
  .action(async function (this: Command) {
    const globals = this.optsWithGlobals<GlobalOptions>();
    try {
      const client = createClient(globals, false);
      const healthy = await client.health();
      output(success("health", { status: healthy ? "ok" : "unexpected" }), Boolean(globals.human));
    } catch (err) {
      printError("health", err, Boolean(globals.human));
    }
  });

program
  .command("auth")
  .description("Credential management")
  .addCommand(
    new Command("set")
      .description("Save token and gateway URL locally")
      .argument("<token>", "Gateway token")
      .action(function (this: Command, token: string) {
        const globals = this.optsWithGlobals<GlobalOptions>();
        try {
          const current = loadAuthConfig();
          const gatewayUrl = resolveGatewayUrl({ gatewayUrl: globals.gatewayUrl });
          saveAuthConfig({ ...current, token, gateway_url: gatewayUrl });
          output(success("auth.set", { saved: true, gateway_url: gatewayUrl }), Boolean(globals.human));
        } catch (err) {
          printError("auth.set", err, Boolean(globals.human));
        }
      })
  )
  .addCommand(
    new Command("status")
      .description("Show saved authentication status")
      .action(function (this: Command) {
        const globals = this.optsWithGlobals<GlobalOptions>();
        try {
          const config = loadAuthConfig();
          output(
            success("auth.status", {
              config_path: configPath(),
              has_token: Boolean(config.token),
              token_preview: config.token ? mask(config.token) : null,
              gateway_url: config.gateway_url ?? null,
            }),
            Boolean(globals.human)
          );
        } catch (err) {
          printError("auth.status", err, Boolean(globals.human));
        }
      })
  )
  .addCommand(
    new Command("clear")
      .description("Remove saved credentials")
      .action(function (this: Command) {
        const globals = this.optsWithGlobals<GlobalOptions>();
        try {
          const path = configPath();
          if (existsSync(path)) {
            unlinkSync(path);
          }
          output(success("auth.clear", { cleared: true }), Boolean(globals.human));
        } catch (err) {
          printError("auth.clear", err, Boolean(globals.human));
        }
      })
  );

program
  .command("config")
  .description("Manage configuration")
  .addCommand(
    new Command("set")
      .description("Set a config value")
      .argument("<key>", "Config key (gateway_url)")
      .argument("<value>", "Config value")
      .action(function (this: Command, key: string, value: string) {
        const globals = this.optsWithGlobals<GlobalOptions>();
        try {
          if (key !== "gateway_url") {
            throw inputError(`Unsupported config key: ${key}`);
          }

          const current = loadAuthConfig();
          saveAuthConfig({ ...current, gateway_url: value });
          output(success("config.set", { key, value }), Boolean(globals.human));
        } catch (err) {
          printError("config.set", err, Boolean(globals.human));
        }
      })
  )
  .addCommand(
    new Command("show")
      .description("Show current configuration")
      .action(function (this: Command) {
        const globals = this.optsWithGlobals<GlobalOptions>();
        try {
          const token = resolveToken({ token: globals.token });
          output(
            success("config.show", {
              config_path: configPath(),
              gateway_url: resolveGatewayUrl({ gatewayUrl: globals.gatewayUrl }),
              has_token: Boolean(token),
              token_preview: token ? mask(token) : null,
              search_modes: SEARCH_MODES,
              search_models: SEARCH_MODELS,
            }),
            Boolean(globals.human)
          );
        } catch (err) {
          printError("config.show", err, Boolean(globals.human));
        }
      })
  );

await program.parseAsync(process.argv);

async function runSearchCommand(queryParts: string[], command: Command, commandName: string): Promise<void> {
  const options = command.optsWithGlobals<SearchCommandOptions>();

  try {
    const request = await resolveSearchRequest(queryParts, options);
    const client = createClient(options);
    const result = await client.search(request.query, {
      mode: request.mode,
      model: request.model,
      split: request.split,
      num: request.num,
    });

    output(success(commandName, result), Boolean(options.human));
  } catch (err) {
    printError(commandName, err, Boolean(options.human));
  }
}

function createClient(globals: GlobalOptions, requireAuth = true): GatewayClient {
  const gatewayUrl = resolveGatewayUrl({ gatewayUrl: globals.gatewayUrl });
  const token = resolveToken({ token: globals.token });

  if (requireAuth && !token) {
    throw configError(
      "Authentication required. Use --token, set AI_SEARCH_TOKEN, or run ai-search auth set <token>.",
      "ai-search auth set <token>"
    );
  }

  return new GatewayClient(gatewayUrl, token);
}

async function resolveSearchRequest(
  queryParts: string[],
  options: SearchCommandOptions
): Promise<{ query: string; mode: SearchMode; model?: string; split?: number; num?: number }> {
  const input = options.stdin ? await readStdinInput() : {};
  const query = queryParts.join(" ") || stringValue(input.query) || "";

  if (!query) {
    throw inputError("Search query is required");
  }

  const modeValue = options.mode ?? stringValue(input.mode) ?? "fast";
  const split = parsePositiveInteger(options.split ?? input.split, "split");
  const num = parsePositiveInteger(options.num ?? input.num, "num");

  return {
    query,
    mode: parseMode(modeValue),
    model: options.model ?? stringValue(input.model) ?? undefined,
    ...(split !== undefined ? { split } : {}),
    ...(num !== undefined ? { num } : {}),
  };
}

async function readStdinInput(): Promise<StdinSearchInput> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }

  const text = Buffer.concat(chunks).toString("utf8").trim();
  if (!text) {
    throw inputError("Expected JSON on stdin because --stdin was provided");
  }

  try {
    const parsed = JSON.parse(text) as unknown;
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
      throw new Error("stdin JSON must be an object");
    }
    return parsed as StdinSearchInput;
  } catch (err) {
    throw inputError(
      `Failed to parse stdin JSON: ${err instanceof Error ? err.message : String(err)}`
    );
  }
}

function parseMode(value: string): SearchMode {
  if ((SEARCH_MODES as readonly string[]).includes(value)) {
    return value as SearchMode;
  }

  throw inputError(`Unsupported search mode: ${value}`);
}

function parsePositiveInteger(value: unknown, name: string): number | undefined {
  if (value === undefined || value === null || value === "") {
    return undefined;
  }

  const normalized = typeof value === "number" ? String(value) : String(value).trim();
  if (!/^[1-9]\d*$/.test(normalized)) {
    throw inputError(`${name} must be a positive integer`);
  }

  return Number.parseInt(normalized, 10);
}

function stringValue(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

function mask(token: string): string {
  if (token.length <= 12) {
    return "***";
  }
  return `${token.slice(0, 8)}...${token.slice(-4)}`;
}

function printError(commandName: string, err: unknown, human = false): never {
  const normalized = normalizeError(err);
  output(
    errorResult(commandName, normalized.code, normalized.message, normalized.suggestion),
    human
  );
  process.exit(normalized.exitCode);
}
