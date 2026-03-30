export const CLI_NAME = "ai-search";
export const CLI_VERSION = "0.2.0";
export const DEFAULT_GATEWAY_URL = "https://search.xiaomao.chat";
export const SEARCH_MODES = ["fast", "deep", "answer"] as const;
export type SearchMode = (typeof SEARCH_MODES)[number];
export const SEARCH_MODELS = [
  "grok-4.1-fast",
  "grok-4.1-expert",
  "grok-4.20-beta",
  "grok-4",
  "grok-4-thinking",
] as const;
