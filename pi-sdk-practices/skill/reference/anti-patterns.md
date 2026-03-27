# Anti-Patterns We Hit (And The Correct Path)

These are real mistakes observed during Origin migration.

## 1) Wrapping Pi With A Custom WS Agent Layer

### Wrong

- `origin-agent/src/server.ts` implements custom WS command routing and lifecycle.
- Pi is treated as an internal library behind another runtime protocol.

### Correct

- Use Pi RPC mode (`runRpcMode` / `RpcClient`) for machine integration.
- Keep WS (if any) as transport-only adapter, not agent semantics owner.

## 2) Manual Provider Bootstrap In Runtime Init

### Wrong

- `origin-agent/src/agent.ts` registers proxy providers in app bootstrap (`registerProxyModels`).

### Correct

- Register providers in `.pi/extensions/providers.ts` via `api.registerProvider()`.
- Keep model/proxy logic in extension layer so Pi runtime owns model registry behavior.

## 3) Extension Factory Aggregation In App Code

### Wrong

- `origin-agent/src/agent.ts` builds `extensionFactories[]` and injects it into `DefaultResourceLoader`.

### Correct

- Put each behavior into standalone `.pi/extensions/*.ts` files.
- Let package manager + resource loader auto-discover them.

## 4) Additional Skill Paths As Primary Mechanism

### Wrong

- `SKILLS_PATHS` and `additionalSkillPaths` are used as core skill mechanism.

### Correct

- Use `.pi/skills/` as primary project skill location.
- Use settings `skills` array only for external/shared skill repos.

## 5) Re-Implementing Skill Discovery Helpers

### Wrong

- Custom `listAvailableSkills()` in `origin-agent/src/agent.ts` reproduces SDK discovery behavior.

### Correct

- Use Pi-native skill discovery and command surfaces (`get_commands` in RPC, `pi.getCommands()` in extensions).

## 6) Treating Pi As A Framework To Wrap, Not A Runtime To Configure

### Wrong

- Maintaining custom config glue (`origin-agent/src/config.ts`) for behavior already handled by `.pi/` conventions.

### Correct

- Encode runtime behavior in:
  - `.pi/settings.json`
  - `.pi/extensions/*`
  - `.pi/skills/*`
  - `AGENTS.md`

## Target End State For Origin

```text
/Users/fan/Origin/.pi/
  extensions/
    providers.ts
    memory.ts
    review.ts
    worker.ts
    image-gen.ts
    web-search.ts
    visual.ts
  skills/
    pi-sdk-practices/
      SKILL.md
      reference/
  settings.json
```

## Why This Matters

Each anti-pattern above increases divergence from upstream Pi behavior, which increases migration cost every time Pi changes contracts.
Native `.pi` layout keeps maintenance near-zero and makes upgrades mostly declarative.
