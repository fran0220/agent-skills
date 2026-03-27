import { describe, it, expect } from "vitest";
import { success, error, filterFields } from "../../src/utils/output.js";

describe("success()", () => {
  it("creates correct envelope with ok: true, command, and data", () => {
    const result = success("project.init", { name: "my-game" });
    expect(result).toEqual({
      ok: true,
      command: "project.init",
      data: { name: "my-game" },
    });
  });
});

describe("error()", () => {
  it("creates correct envelope with ok: false, command, and error details", () => {
    const result = error("project.init", "NOT_FOUND", "Project not found");
    expect(result).toEqual({
      ok: false,
      command: "project.init",
      error: { code: "NOT_FOUND", message: "Project not found" },
    });
  });

  it("includes suggestion field when provided", () => {
    const result = error(
      "project.init",
      "NOT_FOUND",
      "Project not found",
      "Run godot-forge project init first"
    );
    expect(result).toEqual({
      ok: false,
      command: "project.init",
      error: {
        code: "NOT_FOUND",
        message: "Project not found",
        suggestion: "Run godot-forge project init first",
      },
    });
  });
});

describe("filterFields()", () => {
  const data = { name: "my-game", version: "1.0", engine: "godot" };

  it("filters to specified fields", () => {
    const result = filterFields(data, "name,version");
    expect(result).toEqual({ name: "my-game", version: "1.0" });
  });

  it("returns full data when no fields specified", () => {
    const result = filterFields(data);
    expect(result).toEqual(data);
  });

  it("handles non-existent field names gracefully", () => {
    const result = filterFields(data, "name,nonexistent");
    expect(result).toEqual({ name: "my-game" });
  });
});
