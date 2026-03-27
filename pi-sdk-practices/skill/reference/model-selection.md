# Model Selection Strategy (Origin)

This is the runtime model-routing policy used in Origin.

## Primary Role Mapping

- Creator: `proxy-claude/claude-sonnet-4-6`
- Paal reviewer: `proxy-gemini/gemini-3-flash-preview`
- Deep reasoning: `proxy-claude/claude-opus-4-6` or `proxy-gemini/gemini-3.1-pro-preview`
- Web/search-heavy tasks: `proxy-grok/grok-4.1-fast`
- Chinese reasoning/output: `proxy-glm/glm-5`

## Decision Table

| Scenario | Preferred Model | Why |
|---|---|---|
| General feature implementation | `claude-sonnet-4-6` | Stable coding quality and planning balance |
| Strict quality evaluation / Paal scoring | `gemini-3-flash-preview` | Fast and cost-efficient evaluator role |
| Hard architectural tradeoffs | `claude-opus-4-6` | Better long-chain reasoning |
| Broad external search summarization | `grok-4.1-fast` | Strong web-centric response style |
| Chinese-heavy instruction sets | `glm-5` | Better Chinese comprehension and expression |

## Thinking Level Guidance

- Reasoning models: default `medium`
- Non-reasoning models: `off`
- Raise to `high` only for hard synthesis tasks where latency is acceptable

## Failure / Fallback Policy

1. Keep provider stable, downshift model size first if latency/cost spikes.
2. If provider fails, switch to equivalent capability tier:
   - Sonnet -> GPT/Gemini Pro backup
   - Gemini Flash reviewer -> Sonnet reviewer fallback
3. Log role-switch decisions into workspace memory artifacts to keep behavior auditable.

## Registration Dependency

This strategy assumes providers are registered up front by extension (`providers.ts`) using:

- `proxy-claude`
- `proxy-gpt`
- `proxy-gemini`
- `proxy-grok`
- `proxy-glm`

## Operational Rule

Do not hardcode model IDs in many modules. Centralize role defaults in one runtime policy module or settings layer to reduce upgrade drift.
