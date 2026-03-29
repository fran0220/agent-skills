/**
 * Unified output envelope for Agent-Primary CLI.
 *
 * JSON is the DEFAULT output. --human is opt-in.
 */

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

export function error(
  command: string,
  code: string,
  message: string,
  suggestion?: string
): ErrorResult {
  return {
    ok: false,
    command,
    error: { code, message, ...(suggestion ? { suggestion } : {}) },
  };
}

/**
 * Write result to stdout as JSON or human-readable format.
 */
export function output(result: Result, human = false): void {
  if (human) {
    if (result.ok) {
      console.log(`✓ ${result.command}`);
      console.log(formatHuman(result.data));
    } else {
      console.error(`✗ ${result.command}: ${result.error.message}`);
      if (result.error.suggestion) {
        console.error(`  → ${result.error.suggestion}`);
      }
    }
  } else {
    process.stdout.write(JSON.stringify(result, null, 2) + "\n");
  }
}

/**
 * Filter output fields if --fields is specified.
 */
export function filterFields<T extends Record<string, unknown>>(
  data: T,
  fields?: string
): Partial<T> {
  if (!fields) return data;
  const allowed = new Set(fields.split(",").map((f) => f.trim()));
  const filtered: Record<string, unknown> = {};
  for (const key of allowed) {
    if (key in data) {
      filtered[key] = data[key];
    }
  }
  return filtered as Partial<T>;
}

function formatHuman(data: unknown, indent = 0): string {
  if (data === null || data === undefined) return "";
  if (typeof data === "string") return " ".repeat(indent) + data;
  if (typeof data !== "object") return " ".repeat(indent) + String(data);

  const pad = " ".repeat(indent);
  if (Array.isArray(data)) {
    return data.map((item) => formatHuman(item, indent + 2)).join("\n");
  }

  return Object.entries(data as Record<string, unknown>)
    .map(([key, val]) => {
      if (typeof val === "object" && val !== null) {
        return `${pad}${key}:\n${formatHuman(val, indent + 2)}`;
      }
      return `${pad}${key}: ${val}`;
    })
    .join("\n");
}
