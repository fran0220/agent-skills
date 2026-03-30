import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { configError } from "./errors.js";
import { DEFAULT_GATEWAY_URL } from "./meta.js";

export interface AuthConfig {
  token?: string;
  gateway_url?: string;
}

export function configDir(): string {
  const home = process.env.HOME;
  if (!home) {
    throw configError("HOME environment variable is not set");
  }

  return join(home, ".config", "ai-search");
}

export function configPath(): string {
  return join(configDir(), "auth.json");
}

export function loadAuthConfig(): AuthConfig {
  const path = configPath();
  if (!existsSync(path)) {
    return {};
  }

  const content = readFileSync(path, "utf8");
  if (!content.trim()) {
    return {};
  }

  try {
    const parsed = JSON.parse(content) as AuthConfig;
    return { token: parsed.token, gateway_url: parsed.gateway_url };
  } catch (error) {
    throw configError(
      `Failed to parse auth config at ${path}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

export function saveAuthConfig(config: AuthConfig): void {
  const path = configPath();
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, JSON.stringify(config, null, 2) + "\n", { mode: 0o600 });
  chmodSync(path, 0o600);
}

export function resolveGatewayUrl(overrides?: { gatewayUrl?: string }): string {
  return overrides?.gatewayUrl ?? process.env.AI_SEARCH_GATEWAY_URL ?? loadAuthConfig().gateway_url ?? DEFAULT_GATEWAY_URL;
}

export function resolveToken(overrides?: { token?: string }): string | undefined {
  return overrides?.token ?? process.env.AI_SEARCH_TOKEN ?? loadAuthConfig().token;
}
