export class CogneeAdminError extends Error {
  code: string;
  exitCode: number;
  suggestion?: string;

  constructor(
    message: string,
    options: { code: string; exitCode?: number; suggestion?: string }
  ) {
    super(message);
    this.name = "CogneeAdminError";
    this.code = options.code;
    this.exitCode = options.exitCode ?? 1;
    this.suggestion = options.suggestion;
  }
}

export function configError(message: string, suggestion?: string): CogneeAdminError {
  return new CogneeAdminError(message, {
    code: "CONFIG_ERROR",
    exitCode: 1,
    suggestion: suggestion ?? "Run cognee-admin health to verify configuration",
  });
}

export function notFoundError(message: string): CogneeAdminError {
  return new CogneeAdminError(message, {
    code: "NOT_FOUND",
    exitCode: 1,
  });
}

export function apiError(message: string, suggestion?: string): CogneeAdminError {
  return new CogneeAdminError(message, {
    code: "COGNEE_API_ERROR",
    exitCode: 3,
    suggestion: suggestion ?? "Check if Cognee is running: cognee-admin health",
  });
}

export function httpClientError(message: string, suggestion?: string): CogneeAdminError {
  return new CogneeAdminError(message, {
    code: "HTTP_CLIENT_ERROR",
    exitCode: 3,
    suggestion: suggestion ?? "Check network connectivity to Cognee API",
  });
}

export function internalError(message: string): CogneeAdminError {
  return new CogneeAdminError(message, {
    code: "INTERNAL_ERROR",
    exitCode: 2,
  });
}

export function normalizeError(error: unknown): CogneeAdminError {
  if (error instanceof CogneeAdminError) {
    return error;
  }

  if (error instanceof Error) {
    return internalError(error.message);
  }

  return internalError(String(error));
}
