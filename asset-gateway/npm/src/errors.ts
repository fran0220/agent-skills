export class GatewayError extends Error {
  code: string;
  exitCode: number;
  suggestion?: string;

  constructor(
    message: string,
    options: { code: string; exitCode?: number; suggestion?: string }
  ) {
    super(message);
    this.name = "GatewayError";
    this.code = options.code;
    this.exitCode = options.exitCode ?? 1;
    this.suggestion = options.suggestion;
  }
}

export function configError(message: string, suggestion?: string): GatewayError {
  return new GatewayError(message, {
    code: "CONFIG_ERROR",
    exitCode: 1,
    suggestion: suggestion ?? "Run asset-gateway auth set <token> to configure credentials",
  });
}

export function notFoundError(message: string): GatewayError {
  return new GatewayError(message, {
    code: "NOT_FOUND",
    exitCode: 1,
  });
}

export function apiError(message: string, suggestion?: string): GatewayError {
  return new GatewayError(message, {
    code: "GATEWAY_API_ERROR",
    exitCode: 3,
    suggestion: suggestion ?? "Check if the gateway is running: asset-gateway provider health",
  });
}

export function httpClientError(message: string, suggestion?: string): GatewayError {
  return new GatewayError(message, {
    code: "HTTP_CLIENT_ERROR",
    exitCode: 3,
    suggestion: suggestion ?? "Check network connectivity to the gateway",
  });
}

export function internalError(message: string): GatewayError {
  return new GatewayError(message, {
    code: "INTERNAL_ERROR",
    exitCode: 2,
  });
}

export function normalizeError(error: unknown): GatewayError {
  if (error instanceof GatewayError) {
    return error;
  }

  if (error instanceof Error) {
    return internalError(error.message);
  }

  return internalError(String(error));
}
