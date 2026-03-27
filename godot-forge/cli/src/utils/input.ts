/**
 * Read JSON input from stdin or --input file.
 */

import { readFileSync } from "node:fs";

export async function readInput<T = unknown>(
  inputFile?: string
): Promise<T | undefined> {
  // --input flag: read from file
  if (inputFile) {
    try {
      const content = readFileSync(inputFile, "utf-8");
      return JSON.parse(content) as T;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      throw new Error(`Failed to read input file "${inputFile}": ${msg}`);
    }
  }

  // Check if stdin has data (non-TTY)
  if (!process.stdin.isTTY) {
    return readStdin<T>();
  }

  return undefined;
}

function readStdin<T>(): Promise<T | undefined> {
  return new Promise((resolve, reject) => {
    let data = "";
    process.stdin.setEncoding("utf-8");
    process.stdin.on("data", (chunk) => {
      data += chunk;
    });
    process.stdin.on("end", () => {
      if (!data.trim()) {
        resolve(undefined);
        return;
      }
      try {
        resolve(JSON.parse(data) as T);
      } catch {
        reject(new Error("Invalid JSON on stdin"));
      }
    });
    process.stdin.on("error", reject);
  });
}
