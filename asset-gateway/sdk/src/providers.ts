import type { AssetForgeTransport } from "./client.js";
import type {
  ProviderHealthListResponse,
  ProviderHealthResponse,
  ProviderListResponse,
  RequestOptions,
} from "./types.js";

export class AssetForgeProviders {
  constructor(private readonly transport: AssetForgeTransport) {}

  async list(options: RequestOptions = {}): Promise<ProviderListResponse> {
    return this.transport.request<ProviderListResponse>("GET", "/api/providers", {
      signal: options.signal,
      headers: options.headers,
    });
  }

  async health(
    idOrOptions?: string | RequestOptions,
    maybeOptions: RequestOptions = {}
  ): Promise<ProviderHealthListResponse | ProviderHealthResponse> {
    if (typeof idOrOptions === "string") {
      return this.transport.request<ProviderHealthResponse>(
        "GET",
        `/api/providers/${encodeURIComponent(idOrOptions)}/health`,
        {
          signal: maybeOptions.signal,
          headers: maybeOptions.headers,
        }
      );
    }

    const options = idOrOptions ?? {};
    return this.transport.request<ProviderHealthListResponse>("GET", "/api/providers/health", {
      signal: options.signal,
      headers: options.headers,
    });
  }
}
