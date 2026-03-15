# Agent Skills

可复用的 Agent 技能（Skill）与命令行工具（CLI）集合。

## 仓库结构

```
agent-skills/
├── skills/              # Skill 层 — 领域知识，教 Agent 做什么
│   ├── godotforge/      # Godot 游戏开发
│   ├── slide-deck/      # AI 幻灯片生成
│   ├── openclaw-best-practices/
│   └── pi-agent-sdk/
│
└── cli/                 # CLI 层 — 可执行工具，让 Agent 能操作
    └── godot-forge/     # GodotForge 的 CLI 工具
```

**Skill 和 CLI 分离**：Skill 是知识（教 Agent 做什么），CLI 是能力（让 Agent 能执行）。一个 Skill 可以依赖多个 CLI，一个 CLI 可以被多个 Skill 使用。

## 技能列表

| Skill | 用途 | 依赖的 CLI | 其他依赖 |
|-------|------|-----------|---------|
| [`godotforge`](./skills/godotforge/) | AI 驱动的 Godot 4.x 游戏开发 | `godot-forge` | Godot 4.4+ |
| [`slide-deck`](./skills/slide-deck/) | AI 幻灯片生成（内容 → 图片 → PPTX/PDF） | — | Python 3.9+, google-genai |
| [`openclaw-best-practices`](./skills/openclaw-best-practices/) | OpenClaw 网关最佳实践 | — | — |
| [`pi-agent-sdk`](./skills/pi-agent-sdk/) | Pi Agent SDK 开发指南 | — | — |

## CLI 列表

| CLI | 用途 | 安装方式 |
|-----|------|---------|
| [`godot-forge`](./cli/godot-forge/) | Godot 项目操作（场景/资产/引擎/导出） | `pip install godot-forge` |

## 使用

### 安装 Skill

```bash
git clone https://github.com/fran0220/agent-skills.git

# 符号链接单个 skill 到项目
ln -s /path/to/agent-skills/skills/godotforge .agents/skills/godotforge

# 或用户级
ln -s /path/to/agent-skills/skills/godotforge ~/.config/amp/skills/godotforge
```

### 安装 CLI

```bash
cd agent-skills/cli/godot-forge
pip install -e .   # 开发模式
# 或
pip install godot-forge  # 发布后从 PyPI 安装
```

## 开发新 Skill / CLI

参见 [AGENTS.md](./AGENTS.md) 中的开发约定。
