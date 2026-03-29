import { readFile } from "node:fs/promises";
import { basename } from "node:path";
import { apiError, configError, httpClientError, notFoundError } from "./errors.js";

export class GatewayClient {
  constructor(
    private readonly baseUrl: string,
    private readonly token?: string
  ) {}

  async get(path: string): Promise<unknown> {
    return this.request("GET", path);
  }

  async post(path: string, body?: unknown): Promise<unknown> {
    return this.request("POST", path, { body });
  }

  async put(path: string, body?: unknown): Promise<unknown> {
    return this.request("PUT", path, { body });
  }

  async delete(path: string): Promise<unknown> {
    return this.request("DELETE", path);
  }

  async uploadFile(filePath: string): Promise<unknown> {
    let content: Buffer;
    try {
      content = await readFile(filePath);
    } catch (error) {
      throw configError(
        `Failed to read file ${filePath}: ${error instanceof Error ? error.message : String(error)}`
      );
    }

    const form = new FormData();
    const arrayBuffer = content.buffer.slice(
      content.byteOffset,
      content.byteOffset + content.byteLength
    ) as ArrayBuffer;
    form.append("file", new Blob([arrayBuffer]), basename(filePath));

    return this.request("POST", "/api/assets/upload", { form });
  }

  private async request(
    method: string,
    path: string,
    options: {
      body?: unknown;
      form?: FormData;
      headers?: Record<string, string>;
    } = {}
  ): Promise<unknown> {
    const url = new URL(path, ensureTrailingSlash(this.baseUrl));

    const headers = new Headers(options.headers);
    if (this.token) {
      headers.set("authorization", `Bearer ${this.token}`);
    }

    let body: BodyInit | undefined;
    if (options.form) {
      body = options.form;
    } else if (options.body !== undefined) {
      headers.set("content-type", "application/json");
      body = JSON.stringify(options.body);
    }

    let response: Response;
    try {
      response = await fetch(url, { method, headers, body });
    } catch (error) {
      throw httpClientError(
        error instanceof Error ? error.message : String(error)
      );
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

    // Server returns { ok, command, data } or { ok, error }
    if (isEnvelope(payload)) {
      if (!payload.ok) {
        const errMsg = typeof payload.error === "object" && payload.error !== null
          ? JSON.stringify(payload.error)
          : String(payload.error);
        throw apiError(errMsg);
      }
      return payload.data;
    }

    return payload;
  }
}

interface ServerEnvelope {
  ok: boolean;
  command?: string;
  data?: unknown;
  error?: unknown;
}

function isEnvelope(value: unknown): value is ServerEnvelope {
  return typeof value === "object" && value !== null && "ok" in value;
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
