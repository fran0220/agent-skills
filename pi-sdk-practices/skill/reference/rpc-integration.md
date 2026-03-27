# RPC Integration (Gateway <-> Pi)

Use Pi RPC mode for headless integration instead of a custom WS runtime wrapper.

## Source Of Truth

- `dist/modes/rpc/rpc-types.d.ts`
- `dist/modes/rpc/rpc-client.d.ts`
- `dist/modes/rpc/rpc-mode.d.ts`

## Architecture Pattern

```text
Gateway / service process
  -> spawn pi process with --mode rpc
  -> write JSONL commands to stdin
  <- read JSONL responses/events from stdout
```

## RPC Commands (0.56.2)

The protocol supports full session control:

- Prompting: `prompt`, `steer`, `follow_up`, `abort`
- Session: `new_session`, `switch_session`, `fork`, `get_fork_messages`
- State: `get_state`, `get_messages`, `get_last_assistant_text`, `set_session_name`
- Models: `set_model`, `cycle_model`, `get_available_models`
- Thinking: `set_thinking_level`, `cycle_thinking_level`
- Queue modes: `set_steering_mode`, `set_follow_up_mode`
- Compaction/retry: `compact`, `set_auto_compaction`, `set_auto_retry`, `abort_retry`
- Shell: `bash`, `abort_bash`
- Utilities: `get_session_stats`, `export_html`, `get_commands`

## `RpcClient` Method Surface

`RpcClient` exposes typed methods for all operations:

- Lifecycle: `start()`, `stop()`, `onEvent()`, `getStderr()`
- Prompt flow: `prompt()`, `steer()`, `followUp()`, `abort()`
- Session flow: `newSession()`, `switchSession()`, `fork()`, `getForkMessages()`, `setSessionName()`
- Query flow: `getState()`, `getMessages()`, `getLastAssistantText()`, `getSessionStats()`, `getCommands()`
- Model flow: `setModel()`, `cycleModel()`, `getAvailableModels()`, `setThinkingLevel()`, `cycleThinkingLevel()`
- Control flow: `setSteeringMode()`, `setFollowUpMode()`, `compact()`, `setAutoCompaction()`, `setAutoRetry()`, `abortRetry()`
- Shell/export: `bash()`, `abortBash()`, `exportHtml()`
- Convenience: `waitForIdle()`, `collectEvents()`, `promptAndWait()`

## Minimal Typed Client Example

```ts
import { RpcClient } from "@mariozechner/pi-coding-agent";

const client = new RpcClient({
  cwd: "/Users/fan/Origin",
  provider: "proxy-claude",
  model: "claude-sonnet-4-6",
});

await client.start();

const unsubscribe = client.onEvent((event) => {
  if (event.type === "message_update" && event.assistantMessageEvent.type === "text_delta") {
    process.stdout.write(event.assistantMessageEvent.delta);
  }
});

await client.prompt("Summarize current task status");
await client.waitForIdle();

unsubscribe();
await client.stop();
```

## Extension UI Sub-Protocol

RPC streams extension UI requests as separate JSON messages:

- Request: `type: "extension_ui_request"`
- Response: `type: "extension_ui_response"`

Supported request methods include:

- Dialog-like: `select`, `confirm`, `input`, `editor`
- Fire-and-forget: `notify`, `setStatus`, `setWidget`, `setTitle`, `set_editor_text`

## Framing Rules

Treat protocol as JSONL and split by `\n` boundaries only.
Avoid generic line splitting behavior that may split valid Unicode separators inside JSON strings.
For upstream `>=0.57`, strict LF framing is explicitly documented and should be treated as mandatory.

## External Reference Implementations

- `clawdbot` and `openclaw` are practical examples of embedding Pi in larger systems.
- `dnouri/pi-coding-agent` demonstrates an alternate frontend adapter over the same agent core.

## Why This Replaces Custom WS Wrapper

- You still get full streaming events (`message_update`, `tool_execution_*`, `agent_end`).
- You keep typed APIs via `RpcClient`.
- You remove duplicated command routing/parsing logic from app code.
- You stay aligned with upstream protocol evolution.

## Migration Notes For Origin

- Replace `origin-agent/src/server.ts` command transport with a process supervisor that hosts one or more Pi RPC sessions.
- Keep WS only as frontend transport if needed; do not re-implement agent semantics above Pi.
