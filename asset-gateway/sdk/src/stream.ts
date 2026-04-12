import type { AssetForge } from "./client.js";
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
} from "./generate.js";
import { toAssetForgeError } from "./types.js";
import type {
  AudioOptions,
  GenerateRequest,
  GenerateResponse,
  ImageOptions,
  JobStatusResponse,
  Model3dOptions,
  MusicOptions,
  RequestOptions,
  SpriteOptions,
  StreamCancelledEvent,
  StreamEventMap,
  StreamProgressEvent,
  TextOptions,
  TtsOptions,
  VideoRequestInput,
  WorldOptions,
} from "./types.js";

type StreamHandler<T> = (payload: T) => void;

type ListenerRegistry<TDone> = {
  [K in keyof StreamEventMap<TDone>]: Set<StreamHandler<StreamEventMap<TDone>[K]>>;
};

export class AssetForgeStream<TDone> {
  private readonly listeners: ListenerRegistry<TDone> = {
    progress: new Set(),
    done: new Set(),
    error: new Set(),
    cancelled: new Set(),
  };

  private settled = false;
  private cancelHook?: (reason?: string) => Promise<void> | void;
  readonly signal: AbortSignal;
  readonly result: Promise<TDone>;
  private readonly abortController = new AbortController();
  private resolveResult!: (value: TDone) => void;
  private rejectResult!: (reason: unknown) => void;

  constructor(externalSignal?: AbortSignal) {
    this.signal = this.abortController.signal;
    this.result = new Promise<TDone>((resolve, reject) => {
      this.resolveResult = resolve;
      this.rejectResult = reject;
    });

    if (externalSignal) {
      linkAbortSignals(externalSignal, this.abortController);
    }
  }

  on<K extends keyof StreamEventMap<TDone>>(
    event: K,
    handler: StreamHandler<StreamEventMap<TDone>[K]>
  ): this {
    this.listeners[event].add(handler);
    return this;
  }

  off<K extends keyof StreamEventMap<TDone>>(
    event: K,
    handler: StreamHandler<StreamEventMap<TDone>[K]>
  ): this {
    this.listeners[event].delete(handler);
    return this;
  }

  async cancel(reason = "cancelled"): Promise<void> {
    if (this.settled) {
      return;
    }

    this.abortController.abort(reason);
    try {
      await this.cancelHook?.(reason);
    } finally {
      this.emit("cancelled", { reason });
      this.reject(toAssetForgeError(new Error(reason)));
    }
  }

  setCancelHook(cancelHook: (reason?: string) => Promise<void> | void): void {
    this.cancelHook = cancelHook;
  }

  emitProgress(payload: StreamProgressEvent): void {
    if (!this.settled) {
      this.emit("progress", payload);
    }
  }

  resolve(value: TDone): void {
    if (this.settled) {
      return;
    }
    this.settled = true;
    this.resolveResult(value);
    this.emit("done", value);
  }

  reject(error: unknown): void {
    if (this.settled) {
      return;
    }
    this.settled = true;
    const normalized = toAssetForgeError(error);
    this.rejectResult(normalized);
    this.emit("error", normalized);
  }

  private emit<K extends keyof StreamEventMap<TDone>>(
    event: K,
    payload: StreamEventMap<TDone>[K]
  ): void {
    for (const handler of this.listeners[event]) {
      handler(payload);
    }
  }
}

export class AssetForgeStreamNamespace {
  constructor(private readonly forge: AssetForge) {}

  job(jobId: string, options: RequestOptions & { intervalMs?: number } = {}): AssetForgeStream<JobStatusResponse> {
    const stream = new AssetForgeStream<JobStatusResponse>(options.signal);
    stream.setCancelHook(async () => {
      try {
        await this.forge.job.cancel(jobId);
      } catch {
        // Ignore cancel races; caller can inspect job state afterward.
      }
    });

    void this.pollJob(jobId, stream, options.intervalMs ?? 1000);
    return stream;
  }

  generate(request: GenerateRequest, options: RequestOptions = {}): AssetForgeStream<GenerateResponse> {
    const stream = new AssetForgeStream<GenerateResponse>(options.signal);
    void this.runLifecycleRequest(() => this.forge.generate(request, { signal: stream.signal }), stream);
    return stream;
  }

  image(prompt: string, options: ImageOptions = {}): AssetForgeStream<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildImageRequest(prompt, rest), { signal });
  }

  video(
    promptOrRequest: string | VideoRequestInput,
    options: Omit<VideoRequestInput, "prompt" | "signal" | "headers"> & { signal?: AbortSignal } = {}
  ): AssetForgeStream<GenerateResponse> {
    if (typeof promptOrRequest === "string") {
      return this.generate(buildVideoRequest(promptOrRequest, options), { signal: options.signal });
    }

    const { prompt, signal, headers: _headers, ...rest } = promptOrRequest;
    return this.generate(buildVideoRequest(prompt, rest), { signal });
  }

  audio(prompt: string, options: AudioOptions = {}): AssetForgeStream<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildAudioRequest(prompt, rest), { signal });
  }

  music(prompt: string, options: MusicOptions = {}): AssetForgeStream<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildMusicRequest(prompt, rest), { signal });
  }

  tts(prompt: string, options: TtsOptions = {}): AssetForgeStream<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildTtsRequest(prompt, rest), { signal });
  }

  model3d(prompt: string, options: Model3dOptions = {}): AssetForgeStream<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildModel3dRequest(prompt, rest), { signal });
  }

  sprite(prompt: string, options: SpriteOptions = {}): AssetForgeStream<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildSpriteRequest(prompt, rest), { signal });
  }

  world(prompt: string, options: WorldOptions = {}): AssetForgeStream<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildWorldRequest(prompt, rest), { signal });
  }

  text(prompt: string, options: TextOptions = {}): AssetForgeStream<GenerateResponse> {
    const { signal, headers: _headers, ...rest } = options;
    return this.generate(buildTextRequest(prompt, rest), { signal });
  }

  private async pollJob(
    jobId: string,
    stream: AssetForgeStream<JobStatusResponse>,
    intervalMs: number
  ): Promise<void> {
    const startedAt = Date.now();

    while (!stream.signal.aborted) {
      try {
        const job = await this.forge.job.status(jobId, { signal: stream.signal });
        stream.emitProgress({
          jobId,
          status: job.status,
          percent: percentFromStatus(job.status),
          elapsedMs: Date.now() - startedAt,
          estimate: false,
          job,
        });

        if (job.status === "completed") {
          stream.resolve(job);
          return;
        }

        if (job.status === "failed" || job.status === "cancelled") {
          stream.reject(new Error(job.error_message ?? `Job ${job.status}`));
          return;
        }

        await sleep(intervalMs, stream.signal);
      } catch (error) {
        if (stream.signal.aborted) {
          return;
        }
        stream.reject(error);
        return;
      }
    }
  }

  private async runLifecycleRequest(
    task: () => Promise<GenerateResponse>,
    stream: AssetForgeStream<GenerateResponse>
  ): Promise<void> {
    const startedAt = Date.now();
    let percent = 0;

    stream.emitProgress({
      status: "pending",
      percent: 0,
      elapsedMs: 0,
      estimate: true,
    });

    const timer = setInterval(() => {
      percent = Math.min(percent + 12, 96);
      stream.emitProgress({
        status: "running",
        percent,
        elapsedMs: Date.now() - startedAt,
        estimate: true,
      });
    }, 900);

    try {
      const result = await task();
      clearInterval(timer);
      stream.emitProgress({
        jobId: result.job_id,
        status: "completed",
        percent: 100,
        elapsedMs: Date.now() - startedAt,
        estimate: true,
      });
      stream.resolve(result);
    } catch (error) {
      clearInterval(timer);
      if (!stream.signal.aborted) {
        stream.reject(error);
      }
    }
  }
}

function percentFromStatus(status: string): number | null {
  switch (status) {
    case "pending":
      return 20;
    case "running":
      return 70;
    case "completed":
    case "failed":
    case "cancelled":
      return 100;
    default:
      return null;
  }
}

function linkAbortSignals(source: AbortSignal, controller: AbortController): void {
  if (source.aborted) {
    controller.abort(source.reason);
    return;
  }

  const onAbort = () => {
    controller.abort(source.reason);
    source.removeEventListener("abort", onAbort);
  };

  source.addEventListener("abort", onAbort, { once: true });
}

function sleep(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal.aborted) {
      reject(new Error("Operation aborted"));
      return;
    }

    const timeout = setTimeout(() => {
      cleanup();
      resolve();
    }, ms);

    const onAbort = () => {
      clearTimeout(timeout);
      cleanup();
      reject(new Error("Operation aborted"));
    };

    const cleanup = () => signal.removeEventListener("abort", onAbort);
    signal.addEventListener("abort", onAbort, { once: true });
  });
}
