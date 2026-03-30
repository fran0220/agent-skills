import { apiError, httpClientError, notFoundError } from "./errors.js";
import type { SearchMode } from "./meta.js";

export interface SearchResultItem {
  title?: string;
  url?: string;
  snippet?: string;
  source?: string;
  score?: number;
  [key: string]: unknown;
}

export interface SearchResponse {
  query: string;
  mode: string;
  model?: string;
  content: string;
  results?: SearchResultItem[];
  tokens?: number;
  sources_used?: string[];
  [key: string]: unknown;
}

export interface SearchRequestOptions {
  mode?: SearchMode;
  model?: string;
  split?: number;
  num?: number;
}

export interface ProviderInfo {
  id?: string;
  name?: string;
  source?: string;
  [key: string]: unknown;
}

export interface ProviderHealth {
  id?: string;
  name?: string;
  healthy?: boolean;
  status?: string;
  [key: string]: unknown;
}

interface ServerEnvelope {
  ok: boolean;
  command?: string;
  data?: unknown;
  error?: unknown;
}

export class GatewayClient {
  constructor(
    private readonly baseUrl: string,
    private readonly token?: string
  ) {}

  async search(query: string, options: SearchRequestOptions = {}): Promise<SearchResponse> {
    const payload = await this.post("/api/search", {
      query,
      mode: options.mode ?? "fast",
      ...(options.model ? { model: options.model } : {}),
      ...(options.split !== undefined ? { split: options.split } : {}),
      ...(options.num !== undefined ? { num: options.num } : {}),
    });

    return payload as SearchResponse;
  }

  async models(): Promise<string[]> {
    const payload = await this.get("/api/models");
    return extractArray<string>(payload, ["models"]);
  }

  async providers(): Promise<ProviderInfo[]> {
    const payload = await this.get("/api/providers");
    return extractArray<ProviderInfo>(payload, ["providers"]);
  }

  async providersHealth(): Promise<ProviderHealth[]> {
    const payload = await this.get("/api/providers/health");
    return extractArray<ProviderHealth>(payload, ["providers", "health"]);
  }

  async health(): Promise<boolean> {
    const payload = await this.get("/health");
    return isRecord(payload) && payload.status === "ok";
  }

  async get(path: string): Promise<unknown> {
    return this.request("GET", path);
  }

  async post(path: string, body?: unknown): Promise<unknown> {
    return this.request("POST", path, { body });
  }

  private async request(
    method: string,
    path: string,
    options: { body?: unknown; headers?: Record<string, string> } = {}
  ): Promise<unknown> {
    const url = new URL(path, ensureTrailingSlash(this.baseUrl));
    const headers = new Headers(options.headers);

    if (this.token) {
      headers.set("authorization", `Bearer ${this.token}`);
    }

    let body: BodyInit | undefined;
    if (options.body !== undefined) {
      headers.set("content-type", "application/json");
      body = JSON.stringify(options.body);
    }

    let response: Response;
    try {
      response = await fetch(url, { method, headers, body });
    } catch (error) {
      throw httpClientError(error instanceof Error ? error.message : String(error));
    }

    const text = await response.text();
    const payload = parseResponse(text);

    if (!response.ok) {
      const preview = typeof payload === "string" ? payload : JSON.stringify(payload);
      if (response.status === 404) {
        throw notFoundError(`HTTP 404 - ${truncate(preview, 512)}`);
      }
      throw apiError(`HTTP ${response.status} - ${truncate(preview, 512)}`);
    }

    if (isEnvelope(payload)) {
      if (!payload.ok) {
        const message = typeof payload.error === "object" && payload.error !== null
          ? JSON.stringify(payload.error)
          : String(payload.error);
        throw apiError(message);
      }
      return payload.data;
    }

    return payload;
  }
}

function extractArray<T>(payload: unknown, keys: string[]): T[] {
  if (Array.isArray(payload)) {
    return payload as T[];
  }

  if (isRecord(payload)) {
    for (const key of keys) {
      const value = payload[key];
      if (Array.isArray(value)) {
        return value as T[];
      }
    }

    const arrayValue = Object.values(payload).find(Array.isArray);
    if (Array.isArray(arrayValue)) {
      return arrayValue as T[];
    }
  }

  throw apiError(`Unexpected response shape for ${keys.join("/")}`);
}

function isEnvelope(value: unknown): value is ServerEnvelope {
  return isRecord(value) && "ok" in value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function ensureTrailingSlash(url: string): string {
  return url.endsWith("/") ? url : `${url}/`;
}

function parseResponse(text: string): unknown {
  if (!text) {
    return null;
  }

  try {
    return JSON.parse(text) as unknown;
  } catch {
    return text;
  }
}

function truncate(value: string, maxLength: number): string {
  if (value.length <= maxLength) {
    return value;
  }
  return `${value.slice(0, maxLength)}...[truncated]`;
}
