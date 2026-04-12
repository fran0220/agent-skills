import type { AssetForgeTransport } from "./client.js";
import { normalizeMaybeGenerateResponse } from "./generate.js";
import { AssetForgeError } from "./types.js";
import type {
  JobCancelResponse,
  JobListQuery,
  JobListResponse,
  JobStatusResponse,
  RequestOptions,
  WaitForJobOptions,
} from "./types.js";

export class AssetForgeJobs {
  constructor(private readonly transport: AssetForgeTransport) {}

  async list(query: JobListQuery = {}, options: RequestOptions = {}): Promise<JobListResponse> {
    return this.transport.request<JobListResponse>("GET", "/api/jobs", {
      query: {
        status: query.status,
        limit: query.limit,
      },
      signal: options.signal,
      headers: options.headers,
    });
  }

  async status(id: string, options: RequestOptions = {}): Promise<JobStatusResponse> {
    const job = await this.transport.request<JobStatusResponse>(
      "GET",
      `/api/jobs/${encodeURIComponent(id)}`,
      {
        signal: options.signal,
        headers: options.headers,
      }
    );

    return {
      ...job,
      response:
        job.response !== undefined && job.response !== null
          ? normalizeMaybeGenerateResponse(this.transport.baseUrl, job.response)
          : job.response,
    };
  }

  async cancel(id: string, options: RequestOptions = {}): Promise<JobCancelResponse> {
    return this.transport.request<JobCancelResponse>(
      "POST",
      `/api/jobs/${encodeURIComponent(id)}/cancel`,
      {
        signal: options.signal,
        headers: options.headers,
      }
    );
  }

  async wait(id: string, options: WaitForJobOptions = {}): Promise<JobStatusResponse> {
    const intervalMs = Math.max(250, options.intervalMs ?? 1000);

    while (true) {
      if (options.signal?.aborted) {
        throw new AssetForgeError("Job wait aborted", { code: "ABORTED" });
      }

      const job = await this.status(id, options);
      if (isTerminalStatus(job.status)) {
        return job;
      }

      await sleep(intervalMs, options.signal);
    }
  }
}

function isTerminalStatus(status: string): boolean {
  return status === "completed" || status === "failed" || status === "cancelled";
}

function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(new AssetForgeError("Operation aborted", { code: "ABORTED" }));
      return;
    }

    const timeout = setTimeout(() => {
      cleanup();
      resolve();
    }, ms);

    const onAbort = () => {
      clearTimeout(timeout);
      cleanup();
      reject(new AssetForgeError("Operation aborted", { code: "ABORTED" }));
    };

    const cleanup = () => signal?.removeEventListener("abort", onAbort);
    signal?.addEventListener("abort", onAbort, { once: true });
  });
}
