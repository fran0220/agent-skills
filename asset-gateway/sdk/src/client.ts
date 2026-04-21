import {
  buildAudioRequest,
  buildImageRequest,
  buildModel3dRequest,
  buildMusicRequest,
  buildSpriteRequest,
  buildTextRequest,
  buildTtsRequest,
  buildVideoRequest,
  buildWorldRequest,
  normalizeBatchResponse,
  normalizeGenerateResponse,
} from "./generate.js";
import { AssetForgeJobs } from "./jobs.js";
import { AssetForgeProviders } from "./providers.js";
import { AssetForgeStreamNamespace } from "./stream.js";
import { AssetForgeAssets } from "./upload.js";
import { AssetForgeError } from "./types.js";
import type {
  ApiEnvelope,
  ApiErrorPayload,
  AssetForgeOptions,
  BatchGenerateRequest,
  BatchGenerateResponse,
  GenerateRequest,
  GenerateResponse,
  LoginResponse,
  Process3dRequest,
  Process3dResponse,
  ProcessRequest,
  ProcessResponse,
  RequestOptions,
  TransportRequestOptions,
  VideoRequestInput,
  ImageOptions,
  AudioOptions,
  MusicOptions,
  TtsOptions,
  Model3dOptions,
  SpriteOptions,
  WorldOptions,
  TextOptions,
  VoiceCloneRequest,
  VoiceCloneResponse,
  VoiceDeleteResponse,
  VoiceDesignRequest,
  VoiceDesignResponse,
  VoiceListQuery,
  VoiceListResponse,
  QueryValue,
} from "./types.js";

export interface AssetForgeTransport {
  readonly baseUrl: string;
  request<T>(method: string, path: string, options?: TransportRequestOptions): Promise<T>;
}

export const DEFAULT_BASE_URL = "https://asset.origingame.dev";

export class AssetForge implements AssetForgeTransport {
  readonly baseUrl: string;
  readonly job: AssetForgeJobs;
  readonly providers: AssetForgeProviders;
  readonly assets: AssetForgeAssets;
  readonly stream: AssetForgeStreamNamespace;
  readonly voice: {
    clone: (request: VoiceCloneRequest, options?: RequestOptions) => Promise<VoiceCloneResponse>;
    design: (request: VoiceDesignRequest, options?: RequestOptions) => Promise<VoiceDesignResponse>;
    list: (query?: VoiceListQuery) => Promise<VoiceListResponse>;
    delete: (voiceId: string, query?: { type?: "vc" | "vd" } & RequestOptions) => Promise<VoiceDeleteResponse>;
  };

  private readonly apiKey: string;
  private readonly fetchImpl: typeof fetch;
  private readonly defaultHeaders: HeadersInit;

  constructor(options: AssetForgeOptions) {
    if (!options.apiKey.trim()) {
      throw new AssetForgeError("AssetForge apiKey cannot be empty", {
        code: "CONFIG_ERROR",
      });
    }

    if (typeof fetch !== "function" && !options.fetch) {
      throw new AssetForgeError("Global fetch is unavailable in this runtime", {
        code: "FETCH_UNAVAILABLE",
      });
    }

    this.apiKey = options.apiKey.trim();
    this.baseUrl = options.baseUrl?.trim() || DEFAULT_BASE_URL;
    this.fetchImpl = options.fetch ?? fetch;
    this.defaultHeaders = options.headers ?? {};

    this.job = new AssetForgeJobs(this);
    this.providers = new AssetForgeProviders(this);
    this.assets = new AssetForgeAssets(this);
    this.stream = new AssetForgeStreamNamespace(this);
    this.voice = {
      clone: (request, reqOptions = {}) =>
        this.request<VoiceCloneResponse>("POST", "/api/voice/clone", {
          body: request,
          signal: reqOptions.signal,
          headers: reqOptions.headers,
        }),
      design: (request, reqOptions = {}) =>
        this.request<VoiceDesignResponse>("POST", "/api/voice/design", {
          body: request,
          signal: reqOptions.signal,
          headers: reqOptions.headers,
        }),
      list: (query = {}) =>
        this.request<VoiceListResponse>("GET", "/api/voice/list", {
          query: {
            type: query.type,
            page: query.page,
            page_size: query.page_size,
          },
          signal: query.signal,
          headers: query.headers,
        }),
      delete: (voiceId, query = {}) =>
        this.request<VoiceDeleteResponse>("DELETE", `/api/voice/${encodeURIComponent(voiceId)}`, {
          query: { type: query.type },
          signal: query.signal,
          headers: query.headers,
        }),
    };
  }

  async login(token = this.apiKey, options: RequestOptions = {}): Promise<LoginResponse> {
    return this.request<LoginResponse>("POST", "/auth/login", {
      body: { token },
      signal: options.signal,
      headers: options.headers,
    });
  }

  async generate(request: GenerateRequest, options: RequestOptions = {}): Promise<GenerateResponse> {
    const result = await this.request<GenerateResponse>("POST", "/api/generate", {
      body: request,
      signal: options.signal,
      headers: options.headers,
    });

    return normalizeGenerateResponse(this.baseUrl, result);
  }

  async batch(request: BatchGenerateRequest, options: RequestOptions = {}): Promise<BatchGenerateResponse> {
    const result = await this.request<BatchGenerateResponse>("POST", "/api/generate/batch", {
      body: request,
      signal: options.signal,
      headers: options.headers,
    });

    return normalizeBatchResponse(this.baseUrl, result);
  }

  async process(request: ProcessRequest, options: RequestOptions = {}): Promise<ProcessResponse> {
    return this.request<ProcessResponse>("POST", "/api/process", {
      body: request,
      signal: options.signal,
      headers: options.headers,
    });
  }

  async process3d(request: Process3dRequest, options: RequestOptions = {}): Promise<Process3dResponse> {
    return this.request<Process3dResponse>("POST", "/api/process3d", {
      body: request,
      signal: options.signal,
      headers: options.headers,
    });
  }

  async upload(input: Parameters<AssetForgeAssets["upload"]>[0], options: Parameters<AssetForgeAssets["upload"]>[1] = {}): Promise<ReturnType<AssetForgeAssets["upload"]> extends Promise<infer T> ? T : never> {
    return this.assets.upload(input, options);
  }

  image(prompt: string, options: ImageOptions = {}): Promise<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildImageRequest(prompt, rest), { signal });
  }

  video(
    promptOrRequest: string | VideoRequestInput,
    options: Omit<VideoRequestInput, "prompt" | "signal" | "headers"> & { signal?: AbortSignal } = {}
  ): Promise<GenerateResponse> {
    if (typeof promptOrRequest === "string") {
      return this.generate(buildVideoRequest(promptOrRequest, options), { signal: options.signal });
    }

    const { prompt, signal, headers: _headers, ...rest } = promptOrRequest;
    return this.generate(buildVideoRequest(prompt, rest), { signal });
  }

  audio(prompt: string, options: AudioOptions = {}): Promise<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildAudioRequest(prompt, rest), { signal });
  }

  music(prompt: string, options: MusicOptions = {}): Promise<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildMusicRequest(prompt, rest), { signal });
  }

  tts(prompt: string, options: TtsOptions = {}): Promise<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildTtsRequest(prompt, rest), { signal });
  }

  model3d(prompt: string, options: Model3dOptions = {}): Promise<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildModel3dRequest(prompt, rest), { signal });
  }

  sprite(prompt: string, options: SpriteOptions = {}): Promise<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildSpriteRequest(prompt, rest), { signal });
  }

  world(prompt: string, options: WorldOptions = {}): Promise<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildWorldRequest(prompt, rest), { signal });
  }

  text(prompt: string, options: TextOptions = {}): Promise<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildTextRequest(prompt, rest), { signal });
  }

  async request<T>(
    method: string,
    path: string,
    options: TransportRequestOptions = {}
  ): Promise<T> {
    const url = new URL(path, ensureTrailingSlash(this.baseUrl));
    appendQuery(url, options.query);

    const headers = new Headers(this.defaultHeaders);
    headers.set("authorization", `Bearer ${this.apiKey}`);
    applyHeaders(headers, options.headers);

    let body: BodyInit | undefined;
    if (options.form) {
      body = options.form;
    } else if (options.body !== undefined) {
      headers.set("content-type", "application/json");
      body = JSON.stringify(options.body);
    }

    let response: Response;
    try {
      response = await this.fetchImpl(url.toString(), {
        method,
        headers,
        body,
        signal: options.signal,
      });
    } catch (error) {
      throw new AssetForgeError(
        error instanceof Error ? error.message : String(error),
        {
          code: options.signal?.aborted ? "ABORTED" : "NETWORK_ERROR",
          cause: error,
        }
      );
    }

    const text = await response.text();
    const payload = parseJson(text);
    const envelope = isEnvelope(payload) ? payload : null;

    if (envelope?.ok) {
      return envelope.data as T;
    }

    if (!response.ok || envelope?.ok === false) {
      throw createApiError(response.status, envelope?.command, envelope?.ok === false ? envelope.error : payload);
    }

    return payload as T;
  }
}

function createApiError(status: number, command: string | undefined, error: unknown): AssetForgeError {
  if (typeof error === "string") {
    return new AssetForgeError(error, {
      code: defaultErrorCode(status),
      status,
      command,
      details: error,
    });
  }

  if (isApiErrorPayload(error)) {
    return new AssetForgeError(error.message ?? `AssetForge API error (${status})`, {
      code: error.code ?? defaultErrorCode(status),
      status,
      command,
      details: error,
    });
  }

  return new AssetForgeError(
    typeof error === "object" && error !== null ? JSON.stringify(error) : `AssetForge API error (${status})`,
    {
      code: defaultErrorCode(status),
      status,
      command,
      details: error,
    }
  );
}

function defaultErrorCode(status: number): string {
  if (status === 401) return "UNAUTHORIZED";
  if (status === 403) return "FORBIDDEN";
  if (status === 404) return "NOT_FOUND";
  if (status === 409) return "CONFLICT";
  return "API_ERROR";
}

function appendQuery(url: URL, query?: Record<string, QueryValue>): void {
  if (!query) {
    return;
  }

  for (const [key, value] of Object.entries(query)) {
    if (value === undefined || value === null) {
      continue;
    }
    url.searchParams.set(key, String(value));
  }
}

function applyHeaders(target: Headers, source?: HeadersInit): void {
  if (!source) {
    return;
  }

  const extra = new Headers(source);
  extra.forEach((value, key) => {
    target.set(key, value);
  });
}

function ensureTrailingSlash(value: string): string {
  return value.endsWith("/") ? value : `${value}/`;
}

function parseJson(text: string): unknown {
  if (!text) {
    return null;
  }

  try {
    return JSON.parse(text) as unknown;
  } catch {
    return text;
  }
}

function isEnvelope(value: unknown): value is ApiEnvelope<unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value) && "ok" in value;
}

function isApiErrorPayload(value: unknown): value is ApiErrorPayload {
  return typeof value === "object" && value !== null && ("message" in value || "code" in value);
}
