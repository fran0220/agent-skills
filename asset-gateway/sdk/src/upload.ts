import type { AssetForgeTransport } from "./client.js";
import { normalizeAssetFile, normalizeUploadResult } from "./generate.js";
import { AssetForgeError } from "./types.js";
import type {
  AssetDeleteResponse,
  AssetListResponse,
  RequestOptions,
  UploadInput,
  UploadOptions,
  UploadResult,
} from "./types.js";

export class AssetForgeAssets {
  constructor(private readonly transport: AssetForgeTransport) {}

  async upload(input: UploadInput, options: UploadOptions = {}): Promise<UploadResult> {
    const form = new FormData();
    const { blob, filename } = normalizeUploadInput(input, options);
    form.append("file", blob, filename);

    const result = await this.transport.request<UploadResult>("POST", "/api/assets/upload", {
      form,
      signal: options.signal,
      headers: options.headers,
    });

    return normalizeUploadResult(this.transport.baseUrl, result);
  }

  async list(options: RequestOptions = {}): Promise<AssetListResponse> {
    const result = await this.transport.request<AssetListResponse>("GET", "/api/assets", {
      signal: options.signal,
      headers: options.headers,
    });

    return {
      ...result,
      files: result.files.map((file) => normalizeAssetFile(this.transport.baseUrl, file)),
    };
  }

  async delete(filename: string, options: RequestOptions = {}): Promise<AssetDeleteResponse> {
    return this.transport.request<AssetDeleteResponse>(
      "DELETE",
      `/api/assets/${encodeURIComponent(filename)}`,
      {
        signal: options.signal,
        headers: options.headers,
      }
    );
  }
}

function normalizeUploadInput(
  input: UploadInput,
  options: UploadOptions
): { blob: Blob; filename: string } {
  if (typeof File !== "undefined" && input instanceof File) {
    return {
      blob: input,
      filename: options.filename ?? input.name,
    };
  }

  if (input instanceof Blob) {
    return {
      blob: input,
      filename: options.filename ?? inferFilename(input.type, "upload"),
    };
  }

  if (input instanceof ArrayBuffer) {
    return {
      blob: new Blob([input], { type: options.type ?? "application/octet-stream" }),
      filename: options.filename ?? inferFilename(options.type, "upload"),
    };
  }

  if (input instanceof Uint8Array) {
    const arrayBuffer = new Uint8Array(input).buffer;
    return {
      blob: new Blob([arrayBuffer], { type: options.type ?? "application/octet-stream" }),
      filename: options.filename ?? inferFilename(options.type, "upload"),
    };
  }

  if (typeof input === "object" && input !== null && "data" in input) {
    if (!input.filename.trim()) {
      throw new AssetForgeError("Upload filename cannot be empty", {
        code: "INVALID_UPLOAD_INPUT",
      });
    }

    return {
      blob: new Blob(Array.isArray(input.data) ? input.data : [input.data], {
        type: input.type ?? options.type ?? "application/octet-stream",
      }),
      filename: options.filename ?? input.filename,
    };
  }

  throw new AssetForgeError("Unsupported upload input", {
    code: "INVALID_UPLOAD_INPUT",
  });
}

function inferFilename(contentType: string | undefined, basename: string): string {
  const extension = mimeToExtension(contentType);
  return extension ? `${basename}.${extension}` : `${basename}.bin`;
}

function mimeToExtension(contentType: string | undefined): string | null {
  switch (contentType) {
    case "image/png":
      return "png";
    case "image/jpeg":
      return "jpg";
    case "image/webp":
      return "webp";
    case "video/mp4":
      return "mp4";
    case "audio/mpeg":
      return "mp3";
    case "audio/wav":
      return "wav";
    case "model/gltf-binary":
      return "glb";
    default:
      return null;
  }
}
