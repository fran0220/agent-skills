# Provider Registration (Proxy-First)

Origin uses one proxy endpoint and key to expose multiple logical providers.

## Source Of Truth

- `dist/core/extensions/types.d.ts` (`ProviderConfig`, `ProviderModelConfig`)
- `dist/core/extensions/loader.js` (`registerProvider` runtime queueing)
- `origin-agent/src/agent.ts` (current 5-provider proxy mapping)

## Registration Strategy

Register providers in an extension under `.pi/extensions/providers.ts`:

```ts
import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";

export default function (api: ExtensionAPI) {
  const base = process.env.LLM_PROXY_URL;
  const key = process.env.LLM_PROXY_KEY;
  if (!base || !key) return;

  api.registerProvider("proxy-claude", {
    baseUrl: base,
    apiKey: key,
    api: "anthropic-messages",
    models: [
      {
        id: "claude-sonnet-4-6",
        name: "Claude Sonnet 4.6",
        reasoning: true,
        input: ["text", "image"],
        cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
        contextWindow: 200000,
        maxTokens: 16384,
      },
    ],
  });
}
```

## Origin Proxy Providers

From current runtime behavior in `origin-agent/src/agent.ts`:

- `proxy-claude` (`anthropic-messages`)
- `proxy-gpt` (`openai-completions`)
- `proxy-gemini` (`openai-completions` in current implementation)
- `proxy-grok` (`openai-completions`)
- `proxy-glm` (`openai-completions`)

## ProviderConfig Fields

- `baseUrl?: string`
- `apiKey?: string`
- `api?: Api`
- `streamSimple?` custom stream handler
- `headers?: Record<string, string>`
- `authHeader?: boolean`
- `models?: ProviderModelConfig[]`
- `oauth?` login/refresh/getApiKey hooks

## ProviderModelConfig Fields

- `id`, `name`
- `api?` (override provider-level API)
- `reasoning`
- `input: ("text" | "image")[]`
- `cost` object
- `contextWindow`, `maxTokens`
- `headers?`, `compat?`

## Runtime Semantics

- Calls made during extension initialization are queued and applied when runtime binds core components.
- Calls made later (commands/events) apply immediately.
- `api.unregisterProvider(name)` removes provider registrations and restores overridden built-ins.

## Recommended Origin Layout

```text
.pi/extensions/
  providers.ts
  memory.ts
  review.ts
  worker.ts
  image-gen.ts
  web-search.ts
  visual.ts
```

## Practical Guardrails

- Keep provider registration in one extension file.
- Keep model IDs centralized and versioned with this repository.
- Avoid scattering model/provider literals across multiple runtime modules.
- If proxy contract changes, update one place (`providers.ts`) plus model strategy docs.
