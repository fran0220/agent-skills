# Game Design Theory for AI-Assisted Development

> **Purpose**: Give the Agent enough design vocabulary to make informed decisions about
> game structure, pacing, and economy—without requiring a human designer in the loop.

---

## 1. Core Loop

Every game revolves around a **Core Loop**: a tight cycle the player repeats hundreds
of times. If this loop isn't fun in isolation, nothing else will save the game.

```
ACTION  →  REWARD  →  EXPANSION
  ↑                       │
  └───────────────────────┘
```

### Genre Examples

| Genre | Action | Reward | Expansion |
|-------|--------|--------|-----------|
| **Platformer** | Jump, dodge, run | Coins, reach checkpoint | Unlock new level/ability |
| **RPG** | Fight enemies | XP, loot drops | Level up, new skills |
| **Puzzle** | Observe, arrange, solve | Completion feedback | Harder puzzle, new mechanic |
| **Roguelike** | Explore, fight | Items, power-ups | Deeper floor, meta-progression |
| **Idle/Clicker** | Click/wait | Currency | Buy upgrade, automate |
| **Survival** | Gather, craft | Tools, shelter | Explore further, fight bosses |

### How the Agent Should Use This

- When scaffolding a new project (`godot-forge project init`), identify the core loop
  first. If the user hasn't defined one, propose one based on the genre.
- Build the core loop **before** menus, settings, or polish. The first playable should
  be: one action, one reward, one feedback signal.
- When generating a GDD (`godot-forge design gdd`), the core loop should be the first
  section after the elevator pitch.

---

## 2. MDA Framework

MDA separates game design into three lenses:

| Layer | What It Is | Who Thinks About It |
|-------|-----------|---------------------|
| **Mechanics** | Rules, systems, numbers | Designer / Agent |
| **Dynamics** | Emergent runtime behavior | Playtesters |
| **Aesthetics** | Emotional player experience | Players |

### The Reverse-Engineering Approach

Design **backwards**: start from the feeling you want, then figure out how to create it.

```
1. Target Aesthetic    → "I want tension and relief"
2. Design Dynamics     → "Random enemy spawns + safe rooms between waves"
3. Implement Mechanics → "Spawn timer, wave counter, room trigger zones"
```

### 8 Aesthetic Categories (Hunicke/LeBlanc/Zubek)

1. **Sensation** — Stimulates senses (visuals, audio, haptics)
2. **Fantasy** — Make-believe, role-play
3. **Narrative** — Story, unfolding drama
4. **Challenge** — Obstacle course, mastery
5. **Fellowship** — Social, cooperation
6. **Discovery** — Exploration, uncovering
7. **Expression** — Self-expression, creativity
8. **Submission** — Relaxation, pastime

### How the Agent Should Use This

- When the user says "I want a scary game," translate to Aesthetics (Sensation + Narrative),
  then propose Dynamics (limited visibility, ambient sound cues, resource scarcity), then
  implement Mechanics (fog of war system, audio trigger zones, ammo caps).
- Include the target aesthetics in the GDD (`godot-forge design gdd`) so all future
  decisions can be validated against them.
- If a feature doesn't serve any target aesthetic, question whether it belongs.

---

## 3. Game Pillars

Pillars are 3–5 non-negotiable design principles that guide **every** decision.

### Template

```markdown
## Game Pillars

1. **[Pillar Name]** — [One-sentence description]
   - Example decision this pillar drives
2. **[Pillar Name]** — [One-sentence description]
   - Example decision this pillar drives
3. **[Pillar Name]** — [One-sentence description]
   - Example decision this pillar drives
```

### Example: Platformer

```markdown
## Game Pillars

1. **Precision** — Every jump should feel fair and intentional
   - → Implement coyote time and jump buffering
2. **Flow** — Levels should encourage continuous movement
   - → No dead-end rooms, coins guide the path
3. **Discovery** — Hidden secrets reward curiosity
   - → Breakable walls, off-screen alcoves with collectibles
```

### How the Agent Should Use This

- Ask for or propose pillars during `godot-forge design gdd`.
- When adding a new feature, check: "Does this serve at least one pillar?"
- When cutting scope, keep features that serve multiple pillars.
- Store pillars in the project's `design/gdd.md` for persistent reference.

---

## 4. Difficulty Curves

How challenge escalates over time determines whether players stay engaged.

### Curve Shapes

```
Linear          Exponential       Sawtooth            Staircase
│    /          │      /          │  /\  /\  /\       │    ┌──
│   /           │     /           │ /  \/  \/  \      │  ┌─┘
│  /            │   /             │/                   │┌─┘
│ /             │  /              │                    ││
│/              │/                │                    ││
└──────         └──────           └──────              └──────
```

| Shape | When to Use | Example |
|-------|-------------|---------|
| **Linear** | Tutorial-heavy games, casual | Most puzzle games |
| **Exponential** | Hardcore, "git gud" | Dark Souls, bullet hells |
| **Sawtooth** | Tension/relief rhythm | Levels with rest areas |
| **Staircase** | Mastery gates | Boss → easy zone → harder boss |

### Practical Implementation

- **Sawtooth** is the safest default — alternate hard/easy sections.
- Each "tooth" introduces ONE new mechanic, then tests it, then lets the player breathe.
- Boss fights are peaks; the level after should start easy.

### How the Agent Should Use This

- When generating levels (`godot-forge scene create --template level`), place difficulty
  markers: enemies-per-room, platform gap distances, time limits.
- For auto-balancing: use `godot-forge design balance` to check if enemy counts follow
  the chosen curve pattern.
- Default to sawtooth unless the user explicitly wants something else.

---

## 5. Economy Design Basics

Any game with numbers (health, gold, XP, ammo) is an economy.

### Four Economy Elements

```
SOURCE  ──→  POOL  ──→  SINK
              ↕
          CONVERTER
```

| Element | Definition | Examples |
|---------|-----------|----------|
| **Source** | Creates currency from nothing | Enemy drops, quest rewards, daily login |
| **Sink** | Removes currency permanently | Shops, upgrades, consumables, death penalty |
| **Converter** | Transforms one resource into another | Crafting (wood → arrows), XP → levels |
| **Trader** | Moves resources between pools | Shops, NPC barter |

### Balancing Rules of Thumb

1. **Sinks ≥ Sources** — If players can accumulate infinitely, the economy breaks.
2. **Every source should have a corresponding sink** — Gold from enemies → spent in shops.
3. **Converters create depth** — More conversion paths = more strategic choices.
4. **Test with 10x play time** — If a 1-hour session breaks the economy, a 10-hour one
   will be catastrophic.
5. **Cap resources when in doubt** — Max HP, max ammo, max gold. Prevents runaway.

### How the Agent Should Use This

- When creating Custom Resources (`godot-forge resource create`), define whether each
  numeric property is a source, sink, or pool.
- When generating enemy stats or loot tables, ensure drop rates create a balanced
  source that matches existing sinks.
- When the user asks for a "shop system," always ask: "What are the sinks?" If there's
  nothing to spend on, the shop is pointless.

---

## 6. Minimum Viable Game (MVG)

Build the smallest version that proves the core loop is fun.

### Build Order

```
Step 1: Core Mechanic         ← Can the player DO the thing?
Step 2: Single Level/Arena    ← Is there a space to do it in?
Step 3: Win/Lose Condition    ← Does it end?
Step 4: Basic Feedback        ← Does the player know what happened?
Step 5: Iterate               ← Is it fun? If not, fix Step 1.
```

### What to SKIP in MVG

- ❌ Main menu, settings, pause screen
- ❌ Multiple levels
- ❌ Save/load
- ❌ Audio (unless audio IS the mechanic)
- ❌ Story, cutscenes, dialogue
- ❌ Polish, particles, screen shake

### What to INCLUDE in MVG

- ✅ Player movement that feels right
- ✅ One enemy or obstacle type
- ✅ One feedback signal (score, health bar, death)
- ✅ Restart capability (press R to restart)

### How the Agent Should Use This

- When a user says "make me a game," build the MVG first. Resist the urge to scaffold
  a full project with menus, save systems, and multiple levels.
- Use `godot-forge project init --template <genre>` to bootstrap, then focus the first
  iteration on Steps 1–4 only.
- After MVG is playable (`godot-forge engine run`), ask: "Is the core loop fun?"
  Only then expand to polish, content, and systems.
- Track MVG status in the project config; mark the transition from "prototype" to
  "production" explicitly.
