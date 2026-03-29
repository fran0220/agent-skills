import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { configError } from "./errors.js";
import { DEFAULT_COGNEE_URL } from "./meta.js";

export interface AuthConfig {
  admin_token?: string;
  cognee_url?: string;
}

export function configDir(): string {
  return join(homedir(), ".config", "cognee-admin");
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
    const parsed = JSON.parse(content) as AuthConfig & { token?: string };

    // Migrate legacy single-token format
    if (!parsed.admin_token && parsed.token && parsed.token.startsWith("ca_")) {
      return { admin_token: parsed.token, cognee_url: parsed.cognee_url };
    }

    return { admin_token: parsed.admin_token, cognee_url: parsed.cognee_url };
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

export function resolveCogneeUrl(overrides?: { cogneeUrl?: string }): string {
  return overrides?.cogneeUrl ?? process.env.COGNEE_URL ?? loadAuthConfig().cognee_url ?? DEFAULT_COGNEE_URL;
}

export function resolveAdminToken(overrides?: { adminToken?: string }): string | undefined {
  return overrides?.adminToken ?? process.env.COGNEE_ADMIN_TOKEN ?? loadAuthConfig().admin_token;
}
