/**
 * Parser and generator for Godot project.godot INI-like format.
 */

export interface GodotConfig {
  sections: Map<string, Map<string, string>>; // section → key → raw value string
  comments: string[]; // preserved header comments
}

/**
 * Parse project.godot content into a GodotConfig.
 * Handles multi-line values (e.g., input actions with `{...}` blocks).
 */
export function parseGodotConfig(content: string): GodotConfig {
  const lines = content.split("\n");
  const config: GodotConfig = {
    sections: new Map(),
    comments: [],
  };

  let currentSection = ""; // "" = root (before any section header)
  let inHeaderComments = true;
  let multiLineKey: string | null = null;
  let multiLineValue = "";
  let braceDepth = 0;

  for (const line of lines) {
    // Collect header comments (before first section or key)
    if (inHeaderComments) {
      if (line.startsWith(";") || line.trim() === "") {
        config.comments.push(line);
        continue;
      }
      inHeaderComments = false;
    }

    // Multi-line value continuation
    if (multiLineKey !== null) {
      multiLineValue += "\n" + line;
      for (const ch of line) {
        if (ch === "{") braceDepth++;
        else if (ch === "}") braceDepth--;
      }
      if (braceDepth <= 0) {
        // Finished multi-line value
        const sectionMap = getOrCreateSection(config, currentSection);
        sectionMap.set(multiLineKey, multiLineValue);
        multiLineKey = null;
        multiLineValue = "";
        braceDepth = 0;
      }
      continue;
    }

    const trimmed = line.trim();

    // Skip empty lines and comments within sections
    if (trimmed === "" || trimmed.startsWith(";")) continue;

    // Section header
    const sectionMatch = trimmed.match(/^\[(.+)\]$/);
    if (sectionMatch) {
      currentSection = sectionMatch[1];
      if (!config.sections.has(currentSection)) {
        config.sections.set(currentSection, new Map());
      }
      continue;
    }

    // Key=value assignment
    const eqIdx = trimmed.indexOf("=");
    if (eqIdx > 0) {
      const key = trimmed.slice(0, eqIdx).trimEnd();
      const value = trimmed.slice(eqIdx + 1).trimStart();

      // Check if value starts a multi-line block
      let openBraces = 0;
      let closeBraces = 0;
      for (const ch of value) {
        if (ch === "{") openBraces++;
        else if (ch === "}") closeBraces++;
      }

      if (openBraces > closeBraces) {
        // Multi-line value
        multiLineKey = key;
        multiLineValue = value;
        braceDepth = openBraces - closeBraces;
        continue;
      }

      const sectionMap = getOrCreateSection(config, currentSection);
      sectionMap.set(key, value);
    }
  }

  return config;
}

function getOrCreateSection(
  config: GodotConfig,
  section: string
): Map<string, string> {
  let sectionMap = config.sections.get(section);
  if (!sectionMap) {
    sectionMap = new Map();
    config.sections.set(section, sectionMap);
  }
  return sectionMap;
}

/**
 * Generate project.godot content from a GodotConfig.
 */
export function generateGodotConfig(config: GodotConfig): string {
  const parts: string[] = [];

  // Header comments
  for (const comment of config.comments) {
    parts.push(comment);
  }

  // Root section (empty key "")
  const rootSection = config.sections.get("");
  if (rootSection) {
    for (const [key, value] of rootSection) {
      parts.push(`${key}=${value}`);
    }
  }

  // Named sections
  for (const [sectionName, entries] of config.sections) {
    if (sectionName === "") continue;

    parts.push("");
    parts.push(`[${sectionName}]`);
    if (entries.size > 0) {
      for (const [key, value] of entries) {
        parts.push(`${key}=${value}`);
      }
    }
  }

  parts.push("");
  return parts.join("\n");
}

/**
 * Get a value from a section (returns raw string or undefined).
 */
export function configGet(
  config: GodotConfig,
  section: string,
  key: string
): string | undefined {
  return config.sections.get(section)?.get(key);
}

/**
 * Set a value in a section (creates section if needed).
 * Returns the same config instance (mutated).
 */
export function configSet(
  config: GodotConfig,
  section: string,
  key: string,
  value: string
): GodotConfig {
  const sectionMap = getOrCreateSection(config, section);
  sectionMap.set(key, value);
  return config;
}

/**
 * Remove a key from a section.
 */
export function configUnset(
  config: GodotConfig,
  section: string,
  key: string
): GodotConfig {
  config.sections.get(section)?.delete(key);
  return config;
}

/**
 * List all keys in a section.
 */
export function configListSection(
  config: GodotConfig,
  section: string
): Map<string, string> | undefined {
  return config.sections.get(section);
}

/**
 * List all section names.
 */
export function configListSections(config: GodotConfig): string[] {
  return Array.from(config.sections.keys()).filter((s) => s !== "");
}
