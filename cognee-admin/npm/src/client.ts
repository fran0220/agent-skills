import { readFile } from "node:fs/promises";
import { basename } from "node:path";
import { apiError, configError, httpClientError, notFoundError } from "./errors.js";

export interface CognifyOptions {
  datasets?: string[];
  custom_prompt?: string;
  run_in_background?: boolean;
  chunks_per_batch?: number;
}

export interface SearchOptions {
  search_type?: string;
  top_k?: number;
  datasets?: string[];
}

export class CogneeClient {
  constructor(
    private readonly baseUrl: string,
    private readonly jwt?: string
  ) {}

  async get(path: string): Promise<unknown> {
    return this.request("GET", path);
  }

  async post(path: string, body?: unknown): Promise<unknown> {
    return this.request("POST", path, { body });
  }

  async delete(path: string): Promise<unknown> {
    return this.request("DELETE", path);
  }

  async uploadFiles(datasetName: string, files: string[]): Promise<unknown> {
    const form = new FormData();
    form.set("datasetName", datasetName);

    for (const filePath of files) {
      form.append("data", await fileBlobFromPath(filePath), basename(filePath));
    }

    return this.request("POST", "/api/v1/add", { form });
  }

  async health(): Promise<unknown> {
    return this.get("/health");
  }

  async healthDetailed(): Promise<unknown> {
    return this.get("/health/detailed");
  }

  async datasets(): Promise<unknown> {
    return this.get("/api/v1/datasets");
  }

  async createDataset(name: string): Promise<unknown> {
    return this.post("/api/v1/datasets", { name });
  }

  async deleteDataset(id: string): Promise<unknown> {
    return this.delete(`/api/v1/datasets/${encodeURIComponent(id)}`);
  }

  async deleteAllDatasets(): Promise<unknown> {
    return this.delete("/api/v1/datasets");
  }

  async datasetStatus(): Promise<unknown> {
    return this.get("/api/v1/datasets/status");
  }

  async datasetGraph(id: string): Promise<unknown> {
    return this.get(`/api/v1/datasets/${encodeURIComponent(id)}/graph`);
  }

  async datasetData(id: string): Promise<unknown> {
    return this.get(`/api/v1/datasets/${encodeURIComponent(id)}/data`);
  }

  async deleteData(datasetId: string, dataId: string): Promise<unknown> {
    return this.delete(
      `/api/v1/datasets/${encodeURIComponent(datasetId)}/data/${encodeURIComponent(dataId)}`
    );
  }

  async rawData(datasetId: string, dataId: string): Promise<unknown> {
    return this.get(
      `/api/v1/datasets/${encodeURIComponent(datasetId)}/data/${encodeURIComponent(dataId)}/raw`
    );
  }

  async addData(datasetName: string, content: string): Promise<unknown> {
    const form = new FormData();
    form.set("datasetName", datasetName);
    form.append("data", new Blob([content], { type: "text/plain" }), "inline.txt");
    return this.request("POST", "/api/v1/add", { form });
  }

  async uploadFile(datasetName: string, filePath: string): Promise<unknown> {
    return this.uploadFiles(datasetName, [filePath]);
  }

  async updateData(datasetId: string, dataId: string, filePath: string): Promise<unknown> {
    const form = new FormData();
    form.append("data", await fileBlobFromPath(filePath), basename(filePath));
    return this.request("PATCH", "/api/v1/update", {
      form,
      query: {
        dataset_id: datasetId,
        data_id: dataId,
      },
    });
  }

  async cognify(options: CognifyOptions): Promise<unknown> {
    return this.post("/api/v1/cognify", options);
  }

  async search(query: string, options: SearchOptions = {}): Promise<unknown> {
    return this.post("/api/v1/search", {
      query,
      search_type: options.search_type ?? "GRAPH_COMPLETION",
      top_k: options.top_k ?? 5,
      ...(options.datasets && options.datasets.length > 0
        ? { datasets: options.datasets }
        : {}),
    });
  }

  async searchHistory(): Promise<unknown> {
    return this.get("/api/v1/search");
  }

  async getSettings(): Promise<unknown> {
    return this.get("/api/v1/settings");
  }

  async saveSettings(settings: unknown): Promise<unknown> {
    return this.post("/api/v1/settings", settings);
  }

  async uploadOntology(key: string, filePath: string): Promise<unknown> {
    const form = new FormData();
    form.set("ontology_key", key);
    form.append("ontology_file", await fileBlobFromPath(filePath), basename(filePath));
    return this.request("POST", "/api/v1/ontologies", { form });
  }

  async listOntologies(): Promise<unknown> {
    return this.get("/api/v1/ontologies");
  }

  private async request(
    method: string,
    path: string,
    options: {
      body?: unknown;
      form?: FormData;
      headers?: Record<string, string>;
      query?: Record<string, string>;
    } = {}
  ): Promise<unknown> {
    const url = new URL(path, ensureTrailingSlash(this.baseUrl));
    if (options.query) {
      for (const [key, value] of Object.entries(options.query)) {
        url.searchParams.set(key, value);
      }
    }

    const headers = new Headers(options.headers);
    if (this.jwt) {
      headers.set("authorization", `Bearer ${this.jwt}`);
    }

    let body: BodyInit | undefined;
    if (options.form) {
      body = options.form;
    } else if (options.body instanceof URLSearchParams) {
      body = options.body;
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

    return payload;
  }
}

async function fileBlobFromPath(filePath: string): Promise<Blob> {
  let content: Buffer;
  try {
    content = await readFile(filePath);
  } catch (error) {
    throw configError(
      `Failed to read file ${filePath}: ${error instanceof Error ? error.message : String(error)}`
    );
  }

  const arrayBuffer = content.buffer.slice(
    content.byteOffset,
    content.byteOffset + content.byteLength
  ) as ArrayBuffer;
  return new Blob([arrayBuffer]);
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
