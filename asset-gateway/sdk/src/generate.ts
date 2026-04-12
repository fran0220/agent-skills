import type {
  AudioOptions,
  GenerateRequest,
  GenerateResponse,
  ImageOptions,
  JsonObject,
  JsonValue,
  Model3dOptions,
  MusicOptions,
  SpriteOptions,
  TextOptions,
  TtsOptions,
  UploadResult,
  VideoRequestInput,
  WorldOptions,
  BatchGenerateResponse,
  BatchFrameResult,
  AssetFile,
} from "./types.js";

function copyParams(params?: JsonObject): JsonObject | undefined {
  if (!params) {
    return undefined;
  }
  return { ...params };
}

function mergeParams(base?: JsonObject, extra?: JsonObject): JsonObject | undefined {
  const merged = {
    ...(base ?? {}),
    ...(extra ?? {}),
  };
  return Object.keys(merged).length > 0 ? merged : undefined;
}

export function resolveAssetUrl(baseUrl: string, value?: string | null): string | null {
  if (!value) {
    return null;
  }

  try {
    return new URL(value, ensureTrailingSlash(baseUrl)).toString();
  } catch {
    return value;
  }
}

export function normalizeGenerateResponse(
  baseUrl: string,
  response: GenerateResponse
): GenerateResponse {
  const outputUrl = resolveAssetUrl(baseUrl, response.output_url);
  return {
    ...response,
    output_url: outputUrl,
    url: outputUrl,
  };
}

export function normalizeBatchResponse(
  baseUrl: string,
  response: BatchGenerateResponse
): BatchGenerateResponse {
  return {
    ...response,
    frames: response.frames.map((frame) => normalizeBatchFrame(baseUrl, frame)),
  };
}

export function normalizeBatchFrame(
  baseUrl: string,
  frame: BatchFrameResult
): BatchFrameResult {
  const outputUrl = resolveAssetUrl(baseUrl, frame.output_url);
  return {
    ...frame,
    output_url: outputUrl,
    url: outputUrl,
  };
}

export function normalizeUploadResult(baseUrl: string, response: UploadResult): UploadResult {
  return {
    ...response,
    url: resolveAssetUrl(baseUrl, response.url) ?? response.url,
  };
}

export function normalizeAssetFile(baseUrl: string, file: AssetFile): AssetFile {
  return {
    ...file,
    url: resolveAssetUrl(baseUrl, file.url) ?? file.url,
  };
}

export function normalizeMaybeGenerateResponse(baseUrl: string, value: JsonValue): JsonValue {
  if (!isRecord(value)) {
    return value;
  }

  const outputUrl = typeof value.output_url === "string" ? resolveAssetUrl(baseUrl, value.output_url) : null;
  if (!outputUrl) {
    return value;
  }

  return {
    ...value,
    output_url: outputUrl,
    url: outputUrl,
  };
}

export function buildImageRequest(prompt: string, options: Omit<ImageOptions, "signal" | "headers"> = {}): GenerateRequest {
  const request: GenerateRequest = {
    asset_type: "image",
    prompt,
  };

  if (options.provider) request.provider = options.provider;
  if (options.model) request.model = options.model;
  if (options.size) request.size = options.size;
  if (options.transparent !== undefined) request.transparent = options.transparent;
  if (options.input) request.input_file = options.input;
  if (options.referenceImages?.length) request.reference_images = options.referenceImages;
  if (options.editMode) request.edit_mode = options.editMode;
  if (options.sessionId) request.session_id = options.sessionId;
  const params = copyParams(options.params);
  if (params) request.params = params;

  return request;
}

export function buildVideoRequest(prompt: string, options: Omit<VideoRequestInput, "prompt" | "signal" | "headers"> = {}): GenerateRequest {
  const request: GenerateRequest = {
    asset_type: "video",
    prompt,
  };

  if (options.provider) request.provider = options.provider;
  if (options.model) request.model = options.model;
  if (options.input) request.input_file = options.input;
  if (options.size) request.size = options.size;
  const params = copyParams(options.params);
  if (params) request.params = params;

  return request;
}

export function buildAudioRequest(prompt: string, options: Omit<AudioOptions, "signal" | "headers"> = {}): GenerateRequest {
  return {
    asset_type: "audio",
    prompt,
    provider: options.provider,
    model: options.model,
    params: mergeParams(options.params, {
      ...(options.type ? { audio_type: options.type } : {}),
      ...(options.duration !== undefined ? { duration_seconds: options.duration } : {}),
    }),
  };
}

export function buildMusicRequest(prompt: string, options: Omit<MusicOptions, "signal" | "headers"> = {}): GenerateRequest {
  return {
    asset_type: "music",
    prompt,
    provider: options.provider,
    model: options.model,
    params: mergeParams(options.params, {
      ...(options.duration !== undefined ? { duration_seconds: options.duration } : {}),
      ...(options.forceInstrumental ? { force_instrumental: true } : {}),
      ...(options.outputFormat ? { output_format: options.outputFormat } : {}),
    }),
  };
}

export function buildTtsRequest(prompt: string, options: Omit<TtsOptions, "signal" | "headers"> = {}): GenerateRequest {
  return {
    asset_type: "tts",
    prompt,
    provider: options.provider,
    model: options.model,
    params: mergeParams(options.params, {
      ...(options.voice ? { voice: options.voice } : {}),
      ...(options.voiceId ? { voice_id: options.voiceId } : {}),
      ...(options.language ? { language_type: options.language } : {}),
      ...(options.instructions ? { instructions: options.instructions } : {}),
    }),
  };
}

export function buildModel3dRequest(prompt: string, options: Omit<Model3dOptions, "signal" | "headers"> = {}): GenerateRequest {
  return {
    asset_type: "model3d",
    prompt,
    provider: options.provider,
    model: options.model,
    input_file: options.input,
    params: mergeParams(options.params, {
      ...(options.modelVersion ? { model_version: options.modelVersion } : {}),
      ...(options.faceLimit !== undefined ? { face_limit: options.faceLimit } : {}),
      ...(options.pbr ? { pbr: true } : {}),
      ...(options.textureQuality ? { texture_quality: options.textureQuality } : {}),
      ...(options.autoSize ? { auto_size: true } : {}),
      ...(options.negativePrompt ? { negative_prompt: options.negativePrompt } : {}),
      ...(options.multiview?.length ? { multiview: options.multiview } : {}),
      ...(options.style ? { style: options.style } : {}),
    }),
  };
}

export function buildSpriteRequest(prompt: string, options: Omit<SpriteOptions, "signal" | "headers"> = {}): GenerateRequest {
  return {
    asset_type: "sprite",
    prompt,
    provider: options.provider,
    model: options.model,
    input_file: options.input,
    params: mergeParams(options.params, {
      animation_type: options.animationType ?? "walk",
      direction: options.direction ?? "right",
      duration: options.duration ?? 2,
      output_format: options.outputFormat ?? "spritesheet",
      ...(options.fps !== undefined ? { fps: options.fps } : {}),
      ...(options.style ? { style: options.style } : {}),
    }),
  };
}

export function buildWorldRequest(prompt: string, options: Omit<WorldOptions, "signal" | "headers"> = {}): GenerateRequest {
  return {
    asset_type: "world",
    prompt,
    provider: options.provider,
    model: options.model,
    input_file: options.input,
    params: mergeParams(options.params, {
      ...(options.displayName ? { display_name: options.displayName } : {}),
    }),
  };
}

export function buildTextRequest(prompt: string, options: Omit<TextOptions, "signal" | "headers"> = {}): GenerateRequest {
  return {
    asset_type: "text",
    prompt,
    provider: options.provider,
    model: options.model,
    params: mergeParams(options.params, {
      ...(options.maxTokens !== undefined ? { max_tokens: options.maxTokens } : {}),
    }),
  };
}

function ensureTrailingSlash(value: string): string {
  return value.endsWith("/") ? value : `${value}/`;
}

function isRecord(value: JsonValue): value is Record<string, JsonValue> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
