export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonObject | JsonValue[];
export type JsonObject = { [key: string]: JsonValue };

export type AssetType =
  | "image"
  | "video"
  | "audio"
  | "music"
  | "tts"
  | "model3d"
  | "sprite"
  | "world"
  | "text";

export type ImageEditMode = "edit" | "inpaint" | "restyle" | "expand";
export type JobStatus = "pending" | "running" | "completed" | "failed" | "cancelled" | string;
export type VoiceType = "vc" | "vd";
export type CropMode = "tightest" | "power_of2";
export type ComposeDirection = "horizontal" | "vertical" | "grid";

export interface ApiErrorPayload {
  code?: string;
  message?: string;
  suggestion?: string;
  [key: string]: unknown;
}

export class AssetForgeError extends Error {
  readonly code: string;
  readonly status?: number;
  readonly command?: string;
  readonly details?: unknown;

  constructor(
    message: string,
    options: {
      code?: string;
      status?: number;
      command?: string;
      details?: unknown;
      cause?: unknown;
    } = {}
  ) {
    super(message, options.cause !== undefined ? { cause: options.cause } : undefined);
    this.name = "AssetForgeError";
    this.code = options.code ?? "ASSETFORGE_ERROR";
    this.status = options.status;
    this.command = options.command;
    this.details = options.details;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

export function toAssetForgeError(error: unknown): AssetForgeError {
  if (error instanceof AssetForgeError) {
    return error;
  }

  if (error instanceof Error) {
    return new AssetForgeError(error.message, {
      code: "INTERNAL_ERROR",
      cause: error,
    });
  }

  return new AssetForgeError(String(error), { code: "INTERNAL_ERROR" });
}

export interface AssetForgeOptions {
  apiKey: string;
  baseUrl?: string;
  fetch?: typeof fetch;
  headers?: HeadersInit;
}

export interface RequestOptions {
  signal?: AbortSignal;
  headers?: HeadersInit;
}

export interface TransportRequestOptions extends RequestOptions {
  body?: unknown;
  form?: FormData;
  query?: Record<string, QueryValue>;
}

export type QueryValue = string | number | boolean | null | undefined;

export interface ApiSuccessEnvelope<T> {
  ok: true;
  command?: string;
  data: T;
}

export interface ApiErrorEnvelope {
  ok: false;
  command?: string;
  error: ApiErrorPayload | string;
}

export type ApiEnvelope<T> = ApiSuccessEnvelope<T> | ApiErrorEnvelope;

export interface GenerateRequest {
  asset_type: AssetType;
  prompt?: string;
  model?: string;
  input_file?: string;
  provider?: string;
  size?: string;
  transparent?: boolean;
  reference_images?: string[];
  edit_mode?: ImageEditMode;
  session_id?: string;
  params?: JsonObject;
}

export interface GenerateResponse {
  provider_id: string;
  output_path?: string | null;
  output_url?: string | null;
  output_data?: string | null;
  metadata: JsonValue;
  cost_usd?: number | null;
  elapsed_ms?: number;
  job_id?: string;
  user_id?: string;
  session_id?: string;
  url?: string | null;
}

export interface BatchSharedRequest {
  transparent?: boolean;
  size?: string;
  model?: string;
  provider?: string;
  reference_images?: string[];
  input_file?: string;
}

export interface BatchComposeRequest {
  direction?: ComposeDirection;
  columns?: number;
  padding?: number;
  frame_width?: number;
  frame_height?: number;
}

export interface BatchGenerateRequest {
  asset_type: AssetType;
  prompts: string[];
  shared?: BatchSharedRequest;
  compose?: BatchComposeRequest;
}

export interface BatchFrameResult {
  index: number;
  provider_id?: string | null;
  output_data?: string | null;
  output_url?: string | null;
  cost_usd?: number | null;
  elapsed_ms?: number | null;
  error?: string | null;
  url?: string | null;
}

export interface BatchSpritesheetResult {
  output_data: string;
  width: number;
  height: number;
}

export interface BatchGenerateResponse {
  job_id: string;
  user_id: string;
  frames: BatchFrameResult[];
  total_cost_usd?: number;
  elapsed_ms?: number;
  spritesheet?: BatchSpritesheetResult;
}

export interface ProviderCapabilities {
  supports_transparency: boolean;
  supports_streaming: boolean;
  max_concurrent: number;
  rate_limit_rpm?: number | null;
  priority: number;
}

export interface ProviderInfo {
  id: string;
  display_name: string;
  asset_types: AssetType[];
  capabilities: ProviderCapabilities;
}

export interface ProviderHealth {
  healthy: boolean;
  latency_ms?: number | null;
  message?: string | null;
}

export interface ProviderListResponse {
  providers: ProviderInfo[];
}

export interface ProviderHealthEntry {
  id: string;
  health: ProviderHealth;
}

export interface ProviderHealthListResponse {
  providers: ProviderHealthEntry[];
}

export interface ProviderHealthResponse {
  id: string;
  health: ProviderHealth;
}

export interface UploadResult {
  filename: string;
  url: string;
  size: number;
  content_type: string;
}

export interface AssetFile {
  filename: string;
  url: string;
  size: number;
}

export interface AssetListResponse {
  files: AssetFile[];
}

export interface AssetDeleteResponse {
  filename: string;
}

export type UploadInput =
  | File
  | Blob
  | ArrayBuffer
  | Uint8Array
  | {
      data: BlobPart | BlobPart[];
      filename: string;
      type?: string;
    };

export interface UploadOptions extends RequestOptions {
  filename?: string;
  type?: string;
}

export interface JobListQuery {
  status?: string;
  limit?: number;
}

export interface JobSummary {
  id: string;
  user_id?: string | null;
  asset_type: AssetType | string;
  provider_id: string;
  status: JobStatus;
  error_message?: string | null;
  output_path?: string | null;
  cost_usd?: number | null;
  created_at: string;
  started_at?: string | null;
  completed_at?: string | null;
  duration_ms?: number | null;
}

export interface JobListResponse {
  jobs: JobSummary[];
}

export interface JobStatusResponse extends JobSummary {
  request: JsonValue;
  response?: JsonValue | null;
}

export interface JobCancelResponse {
  id: string;
  previous_status: JobStatus;
  status: JobStatus;
}

export interface WaitForJobOptions extends RequestOptions {
  intervalMs?: number;
}

export type ProcessOperation =
  | {
      op: "smart_crop";
      mode?: CropMode;
    }
  | {
      op: "resize";
      width: number;
      height: number;
    }
  | {
      op: "compose";
      direction: ComposeDirection;
      columns?: number;
      padding?: number;
      frame_width?: number;
      frame_height?: number;
    }
  | {
      op: "extract_frames";
      count?: number;
    }
  | {
      op: "remove_bg";
      bg_color?: string;
    };

export interface ProcessRequest {
  input?: string;
  inputs?: string[];
  operations: ProcessOperation[];
}

export interface ProcessOutputItem {
  output_data: string;
  width: number;
  height: number;
}

export interface ProcessResponse {
  output_data: string;
  width: number;
  height: number;
  outputs: ProcessOutputItem[];
  operations_applied: string[];
  elapsed_ms: number;
}

export interface Process3dRequest {
  task_id: string;
  operation: string;
  params?: JsonObject;
  frame_count?: number;
  resolution?: number;
  camera_angle?: string;
  directions?: number;
}

export type Process3dResponse = JsonValue;

export interface VoiceCloneRequest {
  audio_base64: string;
  audio_mime?: string;
  name: string;
  target_model?: string;
}

export interface VoiceDesignRequest {
  voice_prompt: string;
  preview_text: string;
  name: string;
  target_model?: string;
  language?: string;
}

export interface VoiceListQuery extends RequestOptions {
  type?: VoiceType;
  page?: number;
  page_size?: number;
}

export type VoiceListResponse = JsonValue;
export type VoiceCloneResponse = JsonValue;
export type VoiceDesignResponse = JsonValue;
export type VoiceDeleteResponse = JsonValue;
export type LoginResponse = JsonValue;

export interface ShortcutOptions extends RequestOptions {
  provider?: string;
  model?: string;
  params?: JsonObject;
}

export interface ImageOptions extends ShortcutOptions {
  size?: string;
  transparent?: boolean;
  input?: string;
  referenceImages?: string[];
  editMode?: ImageEditMode;
  sessionId?: string;
}

export interface VideoRequestInput extends ShortcutOptions {
  prompt: string;
  input?: string;
  size?: string;
}

export interface AudioOptions extends ShortcutOptions {
  type?: string;
  duration?: number;
}

export interface MusicOptions extends ShortcutOptions {
  duration?: number;
  forceInstrumental?: boolean;
  outputFormat?: string;
}

export interface TtsOptions extends ShortcutOptions {
  voice?: string;
  voiceId?: string;
  language?: string;
  instructions?: string;
}

export interface Model3dOptions extends ShortcutOptions {
  input?: string;
  modelVersion?: string;
  faceLimit?: number;
  pbr?: boolean;
  textureQuality?: "standard" | "detailed" | string;
  autoSize?: boolean;
  negativePrompt?: string;
  multiview?: string[];
  style?: string;
}

export interface SpriteOptions extends ShortcutOptions {
  input?: string;
  animationType?: string;
  direction?: string;
  duration?: number;
  style?: string;
  outputFormat?: string;
  fps?: number;
}

export interface WorldOptions extends ShortcutOptions {
  input?: string;
  displayName?: string;
}

export interface TextOptions extends ShortcutOptions {
  maxTokens?: number;
}

export interface StreamProgressEvent {
  jobId?: string;
  status: JobStatus;
  percent: number | null;
  elapsedMs: number;
  estimate: boolean;
  job?: JobStatusResponse;
}

export interface StreamCancelledEvent {
  reason?: string;
}

export interface StreamEventMap<TDone> {
  progress: StreamProgressEvent;
  done: TDone;
  error: AssetForgeError;
  cancelled: StreamCancelledEvent;
}
