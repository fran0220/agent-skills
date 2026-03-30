export interface SuccessResult<T = unknown> {
  ok: true;
  command: string;
  data: T;
}

export interface ErrorResult {
  ok: false;
  command: string;
  error: {
    code: string;
    message: string;
    suggestion?: string;
  };
}

export type Result<T = unknown> = SuccessResult<T> | ErrorResult;

export function success<T>(command: string, data: T): SuccessResult<T> {
  return { ok: true, command, data };
}

export function error(command: string, code: string, message: string, suggestion?: string): ErrorResult {
  return { ok: false, command, error: { code, message, ...(suggestion ? { suggestion } : {}) } };
}

export function output(result: Result, human = false): void {
  if (human) {
    if (result.ok) {
      const data = result.data as Record<string, unknown>;
      if (data.content) {
        process.stdout.write(String(data.content) + "\n");
      } else {
        process.stdout.write(JSON.stringify(data, null, 2) + "\n");
      }
    } else {
      process.stderr.write(`✗ ${result.error.code}: ${result.error.message}\n`);
      if (result.error.suggestion) {
        process.stderr.write(`  → ${result.error.suggestion}\n`);
      }
    }
  } else {
    process.stdout.write(JSON.stringify(result, null, 2) + "\n");
  }
}
