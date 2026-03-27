# Extension Patterns (0.56.2 Contracts)

This is the local-contract reference for writing Pi extensions in Origin.

## Source Of Truth

- `dist/core/extensions/loader.js`
- `dist/core/extensions/types.d.ts`

## Required File Format

Extension file must default-export a factory function:

```ts
import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";

export default function (api: ExtensionAPI) {
  // register handlers/tools/providers here
}
```

## Native Load Locations

- Project: `.pi/extensions/*.ts`, `.pi/extensions/*/index.ts`
- Global: `~/.pi/agent/extensions/*.ts`, `~/.pi/agent/extensions/*/index.ts`

No manual `extensionFactories.push(...)` needed for workspace-native loading.

## Core API Surface

- `api.on(event, handler)`
- `api.registerTool(definition)`
- `api.registerProvider(name, config)`
- `api.unregisterProvider(name)`
- `api.registerCommand(name, options)`
- `api.registerShortcut(shortcut, options)`
- `api.registerFlag(name, options)`
- Runtime actions: `sendMessage`, `sendUserMessage`, `appendEntry`, `setModel`, `setThinkingLevel`, etc.

## Event Types

### Session Lifecycle

- `resources_discover`
- `session_start`
- `session_before_switch`
- `session_switch`
- `session_before_fork`
- `session_fork`
- `session_before_compact`
- `session_compact`
- `session_before_tree`
- `session_tree`
- `session_shutdown`

### Agent Lifecycle

- `context`
- `before_agent_start`
- `agent_start`
- `agent_end`
- `turn_start`
- `turn_end`
- `message_start`
- `message_update`
- `message_end`

### Tool/Model/Input

- `tool_execution_start`
- `tool_execution_update`
- `tool_execution_end`
- `tool_call`
- `tool_result`
- `model_select`
- `user_bash`
- `input`

## Return Contracts (Must Match Types)

### `resources_discover`

```ts
{ skillPaths?: string[]; promptPaths?: string[]; themePaths?: string[] }
```

### `session_before_switch`

```ts
{ cancel?: boolean }
```

### `session_before_fork`

```ts
{ cancel?: boolean; skipConversationRestore?: boolean }
```

### `session_before_compact`

```ts
{ cancel?: boolean; compaction?: CompactionResult }
```

### `session_before_tree`

```ts
{
  cancel?: boolean;
  summary?: { summary: string; details?: unknown };
  customInstructions?: string;
  replaceInstructions?: boolean;
  label?: string;
}
```

### `context`

```ts
{ messages?: AgentMessage[] }
```

### `before_agent_start`

```ts
{
  message?: { customType: string; content: unknown; display?: boolean; details?: unknown };
  systemPrompt?: string;
}
```

### `tool_call`

```ts
{ block?: boolean; reason?: string }
```

### `tool_result`

```ts
{ content?: (TextContent | ImageContent)[]; details?: unknown; isError?: boolean }
```

### `user_bash`

```ts
{ operations?: BashOperations; result?: BashResult }
```

### `input`

```ts
{ action: "continue" }
| { action: "transform"; text: string; images?: ImageContent[] }
| { action: "handled" }
```

All other event handlers are effectively `void` return.

## Minimal Pattern Snippets

### Block dangerous tool calls

```ts
api.on("tool_call", async (event) => {
  if (event.toolName === "bash" && typeof event.input?.command === "string") {
    if (event.input.command.includes("rm -rf")) {
      return { block: true, reason: "blocked by policy" };
    }
  }
});
```

### Inject dynamic context

```ts
api.on("context", async (event) => {
  const filtered = event.messages.filter((m) => m.role !== "toolResult");
  return { messages: filtered };
});
```

### Register proxy provider from extension

```ts
api.registerProvider("proxy-claude", {
  baseUrl: process.env.LLM_PROXY_URL,
  apiKey: process.env.LLM_PROXY_KEY,
  api: "anthropic-messages",
  models: [/* ... */],
});
```

## Practical Rule For Origin

All Origin behaviors that currently live in `origin-agent/src/extensions/*.ts` should be moved as-is into `.pi/extensions/*.ts` files with this contract, then loaded natively by Pi.
