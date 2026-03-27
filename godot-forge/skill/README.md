# 🎮 GodotForge Skill

![Godot 4.4+](https://img.shields.io/badge/Godot-4.4+-blue?logo=godotengine) ![Node.js 18+](https://img.shields.io/badge/Node.js-18+-green?logo=node.js) ![Agent-Primary](https://img.shields.io/badge/User-Agent--Primary-purple)

> AI 驱动的 Godot 4.x 游戏开发 Skill —— 教 AI Agent 使用 `godot-forge` CLI 完成从概念到发布的完整游戏开发流程。

## 这是什么

GodotForge 是一个 **Agent Skill**（知识层），让 AI Agent 学会通过 `godot-forge` CLI（能力层）构建完整的 Godot 4.x 游戏。Agent 读取 Skill 获得知识和工作流，然后调用 CLI 执行实际操作。

**不是**：游戏引擎、编辑器插件、或人类直接使用的工具。

## 前置条件

| 依赖 | 版本要求 | 说明 |
|------|---------|------|
| **Godot** | 4.4+ | 必须在 PATH 中可用（命令 `godot`） |
| **Node.js** | 18+ | 运行 godot-forge CLI |
| **godot-forge CLI** | latest | `npm install -g @doufunao123/godot-forge` |

## 安装

### 1. 安装 CLI（能力层）

```bash
npm install -g @doufunao123/godot-forge

# 验证
godot-forge --version
godot --version
```

### 2. 安装 Skill（知识层）

将 Skill 目录符号链接到 Agent 的 skills 目录：

```bash
# Amp Agent
ln -s /path/to/agent-skills/skills/godotforge ~/.config/amp/skills/godotforge

# 其他 Agent —— 按各自 skill 安装方式
```

> **注意**：只链接 `skills/godotforge/`，不链接 `cli/godot-forge/`。CLI 通过 npm 全局安装。

## 架构

```
┌─────────────────────────────────────────────────┐
│                  AI Agent                        │
│  ┌───────────────────┐  ┌─────────────────────┐ │
│  │   GodotForge Skill│  │                     │ │
│  │   (知识层)         │──│  Agent 推理 + 决策   │ │
│  │   SKILL.md        │  │                     │ │
│  │   reference/      │  └──────────┬──────────┘ │
│  │   knowledge/      │             │             │
│  └───────────────────┘             │ 调用        │
└────────────────────────────────────┼─────────────┘
                                     ▼
                          ┌─────────────────────┐
                          │  godot-forge CLI     │
                          │  (能力层)             │
                          │  JSON in → JSON out  │
                          └──────────┬──────────┘
                                     │ L3 操作
                                     ▼
                          ┌─────────────────────┐
                          │  Godot 4.x Engine   │
                          │  (headless 模式)     │
                          └─────────────────────┘
```

**分离原则**：Skill 告诉 Agent _该做什么_ → Agent 调 CLI _去执行_。Skill 不含可执行代码。

## Skill 内容

### SKILL.md — Agent 入口

Agent 加载 Skill 时首先读取此文件，包含 CLI 命令速查、控制层策略和关键规则。

### reference/ — 规范文档

| 文件 | 内容 | 状态 |
|------|------|------|
| `workflow.md` | 19 步标准作业流程（SOP） | 📋 计划中 |
| `cli-commands.md` | CLI 完整命令参考 | 📋 计划中 |
| `godot-formats.md` | `.tscn` / `.tres` 文本格式规范 | 📋 计划中 |
| `quality-gates.md` | 各阶段质量门禁定义 | 📋 计划中 |

### knowledge/ — 领域知识

| 文件 | 内容 | 状态 |
|------|------|------|
| `game-design.md` | 核心循环、MDA 框架 | 📋 计划中 |
| `godot-patterns.md` | 场景树 / 信号 / 状态机模式 | 📋 计划中 |
| `scene-organization.md` | 项目结构与命名约定 | 📋 计划中 |
| `feel-presets.md` | 游戏手感参数预设（跳跃、移动等） | 📋 计划中 |
| `anti-patterns.md` | 常见错误与规避方法 | 📋 计划中 |

## 快速开始

```bash
# 1. 安装
npm install -g @doufunao123/godot-forge

# 2. 初始化项目
godot-forge project init my-platformer --template platformer-2d

# 3. 创建场景
echo '{"name":"player","root_type":"CharacterBody2D"}' | godot-forge scene create
```

Agent 会自动读取 SKILL.md，按 19 步 SOP 引导完整的开发流程。

## 19 步工作流概览

GodotForge 定义了从概念到发布的标准作业流程：

| 阶段 | 步骤 | 说明 |
|------|------|------|
| **设计** | 1. GDD 生成 → 2. 美术风格 → 3. 叙事设计 → 4. 数值平衡 | 设计文档（L1 纯文件） |
| **搭建** | 5. 项目初始化 → 6. 场景创建 → 7. 节点配置 → 8. 脚本编写 | 游戏骨架（L1/L2） |
| **资源** | 9. 资源创建 → 10. 资源导入 → 11. UID 分配 | 素材管理（L2/L3） |
| **验证** | 12. 脚本校验 → 13. 场景校验 → 14. 项目校验 | 质量门禁 |
| **测试** | 15. 引擎预览 → 16. 运行测试 | 运行验证（L3） |
| **发布** | 17. 导出预设 → 18. 构建 → 19. 发布 | 最终产出（L3） |

详见 `reference/workflow.md`（计划中）。

## CLI 命令组

| 命令组 | 子命令 | 说明 |
|--------|--------|------|
| `project` | `init` `info` `config` `validate` | 项目生命周期管理 |
| `design` | `gdd` `art` `narrative` `balance` | 设计文档生成 |
| `scene` | `create` `read` `update` `delete` `list` | 场景 CRUD |
| `node` | `add` `remove` `move` `update` `list` | 节点树操作 |
| `script` | `create` `edit` `validate` `list` | GDScript 管理 |
| `resource` | `create` `import` `list` `check` | 资源管理 |
| `engine` | `import` `uid` `validate` `run` `preview` | 引擎操作（需 Godot） |
| `export` | `preset` `build` | 构建导出 |
| `describe` | `<command-group>` | Schema 自省 |

所有命令默认输出 JSON，添加 `--human` 可获得人类可读格式。

## 四层控制模型

GodotForge 将操作分为四个层级，按复杂度和依赖递进：

| 层级 | 方法 | 需要 Godot? | 典型操作 |
|------|------|:-----------:|---------|
| **L1: 纯文件** | 生成 `.md` / `.json` | ❌ | 设计文档、脚本模板 |
| **L2: Godot 文本** | 读写 `.tscn` / `.tres` / `.gd` | ❌ | 场景编辑、节点配置 |
| **L3a: 引擎只读** | `godot --headless` 查询 | ✅ | UID 分配、格式校验 |
| **L3b: 引擎写入** | `godot --headless` 执行 | ✅ | 资源导入、运行测试、构建导出 |

**核心原则**：
- 能用 L1/L2 完成的，不升级到 L3
- 涉及渲染、物理、导出必须使用真实 Godot 引擎
- 绝不猜测 UID —— 必须通过 `godot-forge engine uid` 让引擎分配
- 绝不手动编辑 `.godot/` 目录 —— 这是引擎缓存

## 开发者指南

如果你需要修改或扩展本 Skill：

1. **阅读 [AGENTS.md](./AGENTS.md)** — 包含内容维护规则和 Skill↔CLI 同步约定
2. **了解分离原则** — Skill 只放知识（Markdown），可执行逻辑在 `cli/godot-forge/`
3. **CLI 蓝图** — 详见 `cli/godot-forge/BLUEPRINT.md` 了解 CLI 设计决策
4. **同步规则** — CLI 命令接口变更时，必须同步更新 SKILL.md 中的命令段

### 目录结构

```
skills/godotforge/
├── SKILL.md          # Agent 入口 — YAML frontmatter + 指令
├── README.md         # 本文件 — 人类文档
├── AGENTS.md         # 开发指令 — 维护规则
├── reference/        # 规范文档 — Agent 按需读取
└── knowledge/        # 领域知识 — Agent 按需读取
```

## 许可

本 Skill 作为 [agent-skills](https://github.com/fran0220/agent-skills) monorepo 的一部分发布。
