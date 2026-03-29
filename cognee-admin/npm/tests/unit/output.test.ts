import { describe, it, expect } from "vitest";
import { success, error, filterFields } from "../../src/output.js";

describe("success()", () => {
  it("creates correct envelope with ok: true, command, and data", () => {
    const result = success("dataset.list", { count: 2 });
    expect(result).toEqual({
      ok: true,
      command: "dataset.list",
      data: { count: 2 },
    });
  });
});

describe("error()", () => {
  it("creates correct envelope with ok: false, command, and error details", () => {
    const result = error("dataset.list", "NOT_FOUND", "Dataset not found");
    expect(result).toEqual({
      ok: false,
      command: "dataset.list",
      error: { code: "NOT_FOUND", message: "Dataset not found" },
    });
  });

  it("includes suggestion field when provided", () => {
    const result = error(
      "login",
      "CONFIG_ERROR",
      "Missing credentials",
      "Run cognee-admin login --username <user> --password <pass>"
    );
    expect(result).toEqual({
      ok: false,
      command: "login",
      error: {
        code: "CONFIG_ERROR",
        message: "Missing credentials",
        suggestion: "Run cognee-admin login --username <user> --password <pass>",
      },
    });
  });
});

describe("filterFields()", () => {
  const data = { total: 3, uploaded: 2, failed: 1 };

  it("filters to specified fields", () => {
    const result = filterFields(data, "uploaded,failed");
    expect(result).toEqual({ uploaded: 2, failed: 1 });
  });

  it("returns full data when no fields specified", () => {
    const result = filterFields(data);
    expect(result).toEqual(data);
  });

  it("handles non-existent field names gracefully", () => {
    const result = filterFields(data, "uploaded,missing");
    expect(result).toEqual({ uploaded: 2 });
  });
});
